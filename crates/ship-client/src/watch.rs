//! Reload signals for the config file (architecture decision 11).

use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use futures_util::{Stream, stream};
use notify_debouncer_mini::{DebounceEventResult, new_debouncer, notify::RecursiveMode};
use ship_core::prelude::*;
use tokio::sync::mpsc;

/// Editors write a save in bursts; one reload per burst.
const QUIET: Duration = Duration::from_millis(100);

/// Watches the file's directory, and the symlink target's directory when
/// they differ, non-recursively, debounced. Yields once per change to the
/// file's name. Dropping the stream stops the watcher. A directory that
/// doesn't exist yet isn't watched, so a missing config folder never yields.
pub fn changes(path: &Path) -> Result<impl Stream<Item = ()> + use<>> {
    let mut watched: Vec<(PathBuf, OsString)> = Vec::new();
    for file in [Some(path.to_owned()), fs::canonicalize(path).ok()]
        .into_iter()
        .flatten()
    {
        if let (Some(dir), Some(name)) = (file.parent(), file.file_name())
            && let Ok(dir) = fs::canonicalize(dir)
            && !watched.iter().any(|(seen, _)| *seen == dir)
        {
            watched.push((dir, name.to_owned()));
        }
    }
    let names: Vec<OsString> = watched.iter().map(|(_, name)| name.clone()).collect();
    let (changed, received) = mpsc::unbounded_channel();
    let mut debouncer = new_debouncer(QUIET, move |result: DebounceEventResult| {
        let ours = result.is_ok_and(|events| {
            events.iter().any(|event| {
                event
                    .path
                    .file_name()
                    .is_some_and(|name| names.iter().any(|ours| ours == name))
            })
        });
        if ours {
            changed.send(()).ok();
        }
    })
    .map_err(|error| err!(Io, "cannot watch the config file", @external: error))?;
    for (dir, _) in &watched {
        debouncer
            .watcher()
            .watch(dir, RecursiveMode::NonRecursive)
            .map_err(|error| err!(Io, "cannot watch {}", dir.display(), @external: error))?;
    }
    Ok(stream::unfold(
        (debouncer, received),
        |(debouncer, mut received)| async move {
            received.recv().await.map(|()| ((), (debouncer, received)))
        },
    ))
}
