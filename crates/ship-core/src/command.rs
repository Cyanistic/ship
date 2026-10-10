//! Every `server.` action: the config's `server.*` bindings, the `ship tab`
//! and `ship pane` subcommands, and the input to `ship_client`'s `execute`.
//! The Rust path matches the config path: `tab::Close` is `server.tab.close`.

#[cfg(feature = "clap")]
use clap::{Args, Subcommand};
use serde::Deserialize;

use crate::{
    id::IdOf,
    model::{NodeId, Pane, Tab},
    prelude::*,
    protocol::MoveTab,
};

#[derive(Clone, Debug, Deserialize)]
#[cfg_attr(feature = "clap", derive(Subcommand))]
#[serde(rename_all = "snake_case")]
pub enum Command {
    /// List, create, inspect, rename, close or move tabs
    #[cfg_attr(feature = "clap", command(subcommand))]
    Tab(tab::TabCommand),
    /// Create, inspect, rename, close or resize panes
    #[cfg_attr(feature = "clap", command(subcommand))]
    Pane(pane::PaneCommand),
}

/// `--pane ID`. A missing ID comes from `SHIP_PANE_ID` on the CLI and the
/// selection on keys. `{}` in the config.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[cfg_attr(feature = "clap", derive(Args))]
#[serde(transparent)]
pub struct PaneTarget {
    /// Pane ID; defaults to the pane this runs in
    #[cfg_attr(
        feature = "clap",
        arg(long = "pane", env = "SHIP_PANE_ID", value_name = "ID")
    )]
    pub id: Option<IdOf<Pane>>,
}

/// `--tab ID`, or `--pane ID` for the tab holding that pane; `--tab` wins
/// when both are set. A missing target is the tab holding the current pane on
/// the CLI (clap fills `--pane` from `SHIP_PANE_ID`), or the selected tab on
/// keys. The config writes one ID, `"tab:…"` or `"pane:…"`, or `{}`.
///
/// Not a clap group: clap counts an env-filled `--pane` as given, so a group
/// would reject `--tab` inside a pane.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[cfg_attr(feature = "clap", derive(Args))]
#[serde(from = "Option<NodeId>")]
pub struct TabTarget {
    /// Tab ID; defaults to the tab holding the pane this runs in
    #[cfg_attr(feature = "clap", arg(long = "tab", value_name = "ID"))]
    pub id: Option<IdOf<Tab>>,
    /// Pane ID, for the tab holding it
    #[cfg_attr(
        feature = "clap",
        arg(long = "pane", env = "SHIP_PANE_ID", value_name = "ID")
    )]
    pub pane: Option<IdOf<Pane>>,
}

impl From<Option<NodeId>> for TabTarget {
    fn from(node: Option<NodeId>) -> Self {
        match node {
            Some(NodeId::Tab(id)) => Self {
                id: Some(id),
                pane: None,
            },
            Some(NodeId::Pane(pane)) => Self {
                id: None,
                pane: Some(pane),
            },
            None => Self::default(),
        }
    }
}

/// Where a tab goes: at most one of the three.
#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[cfg_attr(feature = "clap", derive(Args), group(multiple = false))]
#[serde(default, deny_unknown_fields)]
pub struct Destination {
    /// Parent tab ID; omitted means the top level
    pub parent: Option<IdOf<Tab>>,
    /// Insert before this sibling, under its parent
    #[cfg_attr(feature = "clap", arg(long, value_name = "ID"))]
    pub before: Option<IdOf<Tab>>,
    /// Insert after this sibling, under its parent
    #[cfg_attr(feature = "clap", arg(long, value_name = "ID"))]
    pub after: Option<IdOf<Tab>>,
}

impl Destination {
    /// `None` when empty, so the caller picks the default; an error when the
    /// config sets more than one (clap's group already stops that on the CLI).
    pub fn resolve(self) -> Result<Option<MoveTab>> {
        match (self.parent, self.before, self.after) {
            (None, None, None) => Ok(None),
            (Some(parent), None, None) => Ok(Some(MoveTab::Parent(Some(parent)))),
            (None, Some(sibling), None) => Ok(Some(MoveTab::Before(sibling))),
            (None, None, Some(sibling)) => Ok(Some(MoveTab::After(sibling))),
            _ => Err(err!(
                Validation,
                "give at most one of parent, before and after"
            )),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Left,
    Down,
    Up,
    Right,
}

pub mod tab {
    #[cfg(feature = "clap")]
    use clap::{Args, Subcommand};
    use serde::Deserialize;

    use super::{Destination, TabTarget};
    use crate::protocol::Starter;

    /// `get` and `list` are CLI-only, so the config rejects them.
    #[derive(Clone, Debug, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Subcommand))]
    #[serde(rename_all = "snake_case")]
    pub enum TabCommand {
        /// List top-level tabs and their descendants as JSON, keyed by ID
        #[serde(skip)]
        List,
        /// Create a tab, at the top level or where given, and print it as JSON
        Create(Create),
        /// Print a tab and its descendants as JSON
        #[serde(skip)]
        Get(Get),
        /// Rename a tab and print it as JSON
        Rename(Rename),
        /// Close a tab and everything in it
        Close(Close),
        /// Append under PARENT or to the top level, or place before or after a sibling
        Move(Move),
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(default, deny_unknown_fields)]
    pub struct Create {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub to: Destination,
        /// Tab name; omitted or blank means none
        #[cfg_attr(feature = "clap", arg(long))]
        pub name: Option<String>,
        /// Open the tab with one pane running the shell
        #[cfg_attr(
            feature = "clap",
            arg(long, num_args = 0..=1, default_missing_value = "shell")
        )]
        pub starter: Option<Starter>,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(default, deny_unknown_fields)]
    pub struct Get {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub tab: TabTarget,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(default, deny_unknown_fields)]
    pub struct Rename {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub tab: TabTarget,
        /// New name; omitted or blank clears it
        pub name: Option<String>,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(default, deny_unknown_fields)]
    pub struct Close {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub tab: TabTarget,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(default, deny_unknown_fields)]
    pub struct Move {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub tab: TabTarget,
        #[cfg_attr(feature = "clap", command(flatten))]
        pub to: Destination,
    }
}

pub mod pane {
    use std::path::PathBuf;

    #[cfg(feature = "clap")]
    use clap::{Args, Subcommand};
    use serde::Deserialize;

    use super::{Direction, PaneTarget, TabTarget};
    use crate::protocol::SplitDirection;

    /// `get` is CLI-only, so the config rejects it.
    #[derive(Clone, Debug, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Subcommand))]
    #[serde(rename_all = "snake_case")]
    pub enum PaneCommand {
        /// Create a pane running a program and print it as JSON
        Create(Create),
        /// Print a pane as JSON
        #[serde(skip)]
        Get(Get),
        /// Rename a pane and print it as JSON
        Rename(Rename),
        /// Close a pane
        Close(Close),
        /// Resize a pane (not available yet)
        Resize(Resize),
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(default, deny_unknown_fields)]
    #[cfg_attr(
        feature = "clap",
        command(
            mut_arg("pane", |arg| arg.help("Split anchor pane ID; defaults to the pane this runs in")),
            mut_arg("id", |arg| arg.help("Tab ID; splits its largest pane or creates its first pane; overrides --pane"))
        )
    )]
    pub struct Create {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub tab: TabTarget,
        /// Which side of the anchor to split; right by default
        #[cfg_attr(feature = "clap", arg(long))]
        pub direction: Option<SplitDirection>,
        /// Pane name; omitted or blank means none
        #[cfg_attr(feature = "clap", arg(long))]
        pub name: Option<String>,
        /// Starting directory; defaults to the current directory
        #[cfg_attr(feature = "clap", arg(long))]
        pub cwd: Option<PathBuf>,
        /// Command and arguments; defaults to your login shell
        #[cfg_attr(feature = "clap", arg(last = true, value_name = "COMMAND"))]
        pub command: Vec<String>,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(default, deny_unknown_fields)]
    pub struct Get {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub pane: PaneTarget,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(default, deny_unknown_fields)]
    pub struct Rename {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub pane: PaneTarget,
        /// New name; omitted or blank clears it
        pub name: Option<String>,
    }

    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(default, deny_unknown_fields)]
    pub struct Close {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub pane: PaneTarget,
    }

    #[derive(Clone, Debug, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(deny_unknown_fields)]
    pub struct Resize {
        #[cfg_attr(feature = "clap", command(flatten))]
        #[serde(default)]
        pub pane: PaneTarget,
        /// Which edge moves
        #[cfg_attr(feature = "clap", arg(long))]
        pub direction: Direction,
        /// Cells; omitted means one step
        #[cfg_attr(feature = "clap", arg(long))]
        #[serde(default)]
        pub amount: Option<u16>,
    }
}
