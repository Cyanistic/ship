//! Saved tabs, read once at startup and atomically written as they change.

use std::{
    fs,
    future::pending,
    io::{self, ErrorKind, Write},
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use serde::{Deserialize, Serialize};
use ship_core::{model::PaneStatus, prelude::*, protocol::Replica, tree::Tabs};
use ship_macros::traced;
use tokio::{
    sync::watch,
    task::spawn_blocking,
    time::{Instant, sleep_until},
};

const DEADLINE: Duration = Duration::from_secs(5);
const RETRY: Duration = Duration::from_secs(5);

/// A tree with its live fields blanked: what the file holds, and the only
/// input `ServerState::new` restores from. Made only by `load` and `project`.
#[derive(Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub(crate) struct Saved(Tabs);

impl Saved {
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub(crate) fn into_tabs(self) -> Tabs {
        self.0
    }
}

/// A bad file never prevents startup and stays in place for manual recovery.
/// The contents are trusted like a command's edit: a hand-edited file that
/// parses is restored as written.
#[traced]
pub(crate) fn load(path: &Path) -> Saved {
    let bytes = match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            tracing::info!(path = %path.display(), "no saved tabs");
            return Saved::default();
        }
        Err(error) => {
            tracing::warn!(path = %path.display(), prev = %prev(path).display(), %error, "cannot read saved tabs");
            return Saved::default();
        }
    };
    match serde_json::from_slice(&bytes) {
        Ok(saved) => saved,
        Err(error) => {
            tracing::warn!(path = %path.display(), prev = %prev(path).display(), %error, "cannot parse saved tabs");
            Saved::default()
        }
    }
}

/// The saved form of a published tree (decision 4). Cloning copies only
/// `Arc`s; `Arc::make_mut` copies each tab as it's blanked.
pub(crate) fn project(tabs: &Tabs) -> Saved {
    let mut tabs = tabs.clone();
    blank(&mut tabs);
    Saved(tabs)
}

fn blank(tabs: &mut Tabs) {
    for tab in tabs.values_mut() {
        let tab = Arc::make_mut(tab);
        tab.geometry = None;
        blank(&mut tab.tabs);
        for pane in tab.panes_mut() {
            if let Some(dir) = pane.dir.take() {
                pane.cwd = dir;
            }
            pane.title = None;
            pane.command.clear();
            pane.program.clear();
            pane.status = PaneStatus::Running;
        }
    }
}

/// Owns the last written projection and the first-save backup for this run.
pub(crate) struct Saver {
    path: PathBuf,
    written: Saved,
    kept_prev: bool,
}

impl Saver {
    pub(crate) fn new(path: PathBuf, baseline: Saved) -> Self {
        Self {
            path,
            written: baseline,
            kept_prev: false,
        }
    }

    pub(crate) async fn run(mut self, mut replicas: watch::Receiver<Arc<Replica>>) {
        let mut deadline = None;
        loop {
            let tabs = project(&replicas.borrow_and_update().tabs);
            if tabs == self.written {
                deadline = None;
            } else {
                deadline.get_or_insert_with(|| Instant::now() + DEADLINE);
            }
            tokio::select! {
                biased;
                () = async {
                    match deadline {
                        Some(at) => sleep_until(at).await,
                        None => pending().await,
                    }
                } => {
                    // Take the latest projection, not the one from before the wait.
                    let tabs = project(&replicas.borrow_and_update().tabs);
                    if tabs != self.written {
                        match self.save(tabs).await {
                            Ok(()) => deadline = None,
                            Err(error) => {
                                tracing::error!(path = %self.path.display(), %error, "cannot save tabs");
                                deadline = Some(Instant::now() + RETRY);
                            }
                        }
                    } else {
                        deadline = None;
                    }
                }
                changed = replicas.changed() => {
                    if changed.is_err() {
                        let tabs = project(&replicas.borrow_and_update().tabs);
                        if tabs != self.written
                            && let Err(error) = self.save(tabs).await
                        {
                            tracing::error!(path = %self.path.display(), %error, "cannot save tabs on stop");
                        }
                        return;
                    }
                }
            }
        }
    }

    #[traced(no_err)]
    async fn save(&mut self, tabs: Saved) -> Result<()> {
        let bytes = serde_json::to_vec_pretty(&tabs)
            .map_err(|error| err!(Internal, "cannot serialize saved tabs", @external: error))?;
        let path = self.path.clone();
        let kept_prev = self.kept_prev;
        spawn_blocking(move || {
            if !kept_prev {
                match fs::read(&path) {
                    Ok(old) => replace(&prev(&path), &old)?,
                    Err(error) if error.kind() == ErrorKind::NotFound => {}
                    Err(error) => {
                        return Err(err!(Io, "cannot read previous saved tabs at {}", path.display(), @external: error));
                    }
                }
            }
            replace(&path, &bytes)
        })
        .await
        .map_err(|error| err!(Internal, "save task failed", @external: error))??;
        self.written = tabs;
        self.kept_prev = true;
        Ok(())
    }
}

/// Same-directory temporary files are private and rename over the target atomically.
fn replace(target: &Path, bytes: &[u8]) -> Result<()> {
    let write = || -> io::Result<()> {
        let mut file = tempfile::NamedTempFile::new_in(target.parent().unwrap_or(Path::new(".")))?;
        file.write_all(bytes)?;
        file.as_file().sync_all()?;
        file.persist(target).map_err(|error| error.error)?;
        Ok(())
    };
    write().map_err(
        |error| err!(Io, "cannot replace saved tabs at {}", target.display(), @external: error),
    )
}

fn prev(path: &Path) -> PathBuf {
    path.with_extension("json.prev")
}
