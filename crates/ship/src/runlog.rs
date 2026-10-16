//! Synchronous run files with numbered parts, a current link and bounded history.

use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{self, Path, PathBuf},
    sync::Mutex,
};

use etcetera::BaseStrategy;
#[cfg(not(windows))]
use etcetera::base_strategy::Xdg;
use ship_core::prelude::*;

const PART_BYTES: u64 = 50 * 1024 * 1024;
const DIR_BYTES: u64 = 200 * 1024 * 1024;
const CURRENT: &str = "current.log";

pub fn logs_dir() -> Result<PathBuf> {
    // Ship uses XDG state paths on macOS too, not Apple's Application Support.
    #[cfg(not(windows))]
    let strategy = Xdg::new();
    #[cfg(windows)]
    let strategy = etcetera::choose_base_strategy();
    let strategy = strategy.map_err(
        |error| err!(Configuration, "cannot find the state directory", @external: error),
    )?;
    let directory = strategy.state_dir().unwrap_or_else(|| strategy.data_dir());
    path::absolute(directory.join("ship/logs"))
        .map_err(|error| err!(Io, "cannot resolve log directory", @external: error))
}

pub fn stem(now: &chrono::DateTime<chrono::Local>) -> String {
    // Subseconds distinguish quick restarts without ever reusing an earlier file.
    now.format("%Y-%m-%dT%H-%M-%S%.9f").to_string()
}

pub struct RunLog {
    dir: PathBuf,
    stem: String,
    part: Mutex<Part>,
}

struct Part {
    number: u32,
    file: File,
    written: u64,
}

impl RunLog {
    pub fn start(dir: &Path, stem: String) -> Result<Self> {
        let mut builder = fs::DirBuilder::new();
        builder.recursive(true);
        platform::private_dir(&mut builder);
        builder.create(dir)
            .map_err(|error| err!(Io, "cannot create private log directory {}; no file was created", dir.display(), @external: error))?;
        let path = dir.join(format!("{stem}.log"));
        let file = create(&path).map_err(
            |error| err!(Io, "cannot create private log in {}", dir.display(), @external: error),
        )?;
        let log = Self {
            dir: dir.to_owned(),
            stem,
            part: Mutex::new(Part {
                number: 1,
                file,
                written: 0,
            }),
        };
        log.maintain(&path);
        Ok(log)
    }

    pub fn resume(path: &Path) -> Result<Self> {
        let dir = path
            .parent()
            .ok_or_else(|| err!(Configuration, "log file has no directory"))?;
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| err!(Configuration, "log file has no valid stem"))?;
        let file = OpenOptions::new()
            .append(true)
            .open(path)
            .map_err(|error| err!(Io, "cannot resume log {}", path.display(), @external: error))?;
        let written = file
            .metadata()
            .map_err(|error| err!(Io, "cannot inspect log {}", path.display(), @external: error))?
            .len();
        Ok(Self {
            dir: dir.to_owned(),
            stem: stem.to_owned(),
            part: Mutex::new(Part {
                number: 1,
                file,
                written,
            }),
        })
    }

    pub fn path(&self) -> PathBuf {
        self.part_path(self.part.lock().unwrap().number)
    }

    fn part_path(&self, number: u32) -> PathBuf {
        let suffix = if number == 1 {
            String::new()
        } else {
            format!(".{number}")
        };
        self.dir.join(format!("{}{suffix}.log", self.stem))
    }

    fn maintain(&self, current: &Path) {
        // Links and retention are best effort; neither may prevent logging.
        platform::link_current(&self.dir, current).ok();
        prune(&self.dir, current).ok();
    }
}

impl Write for &RunLog {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let mut part = self.part.lock().unwrap();
        if part.written > 0 && part.written.saturating_add(bytes.len() as u64) > PART_BYTES {
            let path = self.part_path(part.number + 1);
            // A failed rollover must not discard the event.
            if let Ok(file) = create(&path) {
                *part = Part {
                    number: part.number + 1,
                    file,
                    written: 0,
                };
                self.maintain(&path);
            }
        }
        part.file.write_all(bytes)?;
        part.written += bytes.len() as u64;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.part.lock().unwrap().file.flush()
    }
}

fn create(path: &Path) -> io::Result<File> {
    let mut options = OpenOptions::new();
    options.append(true).create_new(true);
    platform::private_file(&mut options);
    options.open(path)
}

fn prune(dir: &Path, current: &Path) -> io::Result<()> {
    let mut files = Vec::new();
    let mut total = 0;
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == CURRENT)
            || path.extension().is_none_or(|extension| extension != "log")
            || !entry.file_type()?.is_file()
        {
            continue;
        }
        let metadata = entry.metadata()?;
        total += metadata.len();
        files.push((metadata.modified()?, path, metadata.len()));
    }
    files.sort_by_key(|(modified, _, _)| *modified);
    for (_, path, bytes) in files {
        if total <= DIR_BYTES {
            break;
        }
        if path != current && fs::remove_file(path).is_ok() {
            total -= bytes;
        }
    }
    Ok(())
}

#[cfg(unix)]
mod platform {
    use super::*;
    use std::{
        os::unix::fs::{DirBuilderExt, OpenOptionsExt, symlink},
        process,
    };

    pub fn private_dir(builder: &mut fs::DirBuilder) {
        builder.mode(0o700);
    }
    pub fn private_file(options: &mut OpenOptions) {
        options.mode(0o600);
    }
    pub fn link_current(dir: &Path, target: &Path) -> io::Result<()> {
        let temporary = dir.join(format!(".current-{}", process::id()));
        fs::remove_file(&temporary).ok();
        symlink(target.file_name().unwrap(), &temporary)?;
        fs::rename(temporary, dir.join(CURRENT))
    }
}

#[cfg(not(unix))]
mod platform {
    use super::*;
    pub fn private_dir(_: &mut fs::DirBuilder) {}
    pub fn private_file(_: &mut OpenOptions) {}
    pub fn link_current(_: &Path, _: &Path) -> io::Result<()> {
        Ok(())
    }
}
