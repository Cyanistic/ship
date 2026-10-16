//! Saved tabs, loaded once at startup and projected without live state.

use std::{
    fs,
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::Arc,
};

use serde::{Deserialize, Serialize};
use ship_core::{model::PaneStatus, tree::Tabs};
use ship_macros::traced;

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
            pane.status = PaneStatus::Running;
        }
    }
}

fn prev(path: &Path) -> PathBuf {
    path.with_extension("json.prev")
}
