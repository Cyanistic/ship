//! One path from a command to the API, for the CLI and for keys.

use std::{
    env, fs,
    path::{self, PathBuf},
};

use serde::Serialize;
use ship_core::{
    command::{
        Command, PaneTarget, TabTarget,
        pane::{self, PaneCommand},
        tab::{self, TabCommand},
    },
    id::IdOf,
    model::{NodeId, Pane, Tab},
    prelude::*,
    protocol::{CreatePane, CreateTab, MoveTab, PaneAt, PaneInput, PaneSpec},
    tree::{self, Tabs},
};

use crate::Client;

/// Where a missing target comes from.
pub enum Scope {
    /// The CLI. clap already filled a missing `--pane` from `SHIP_PANE_ID`,
    /// so a target still missing is an error. CLI-made panes default to the
    /// process's current directory.
    Cli,
    /// A key binding, resolved against the client's view.
    Keys(KeyScope),
}

/// The client's selection and the tabs it was made in. Key-made tabs go
/// after the selected tab; key-made panes start in the server's home
/// directory.
pub struct KeyScope {
    pub selection: Option<NodeId>,
    pub tabs: Tabs,
}

/// What a command produced. Serializes as the resource itself; the CLI
/// prints nothing for `Closed`.
#[derive(Serialize)]
#[serde(untagged)]
pub enum Outcome {
    Tabs(Tabs),
    Tab(Tab),
    Pane(Pane),
    Closed,
}

impl Client {
    /// Resolves targets (an explicit ID, then the scope, then an error
    /// naming the flag), calls the API and returns what it produced.
    pub async fn execute(&self, command: Command, scope: &Scope) -> Result<Outcome> {
        Ok(match command {
            Command::Tab(command) => match command {
                TabCommand::List => Outcome::Tabs(self.tabs().await?),
                TabCommand::Create(tab::Create { to, name, starter }) => {
                    let at = match (to.resolve()?, scope) {
                        (Some(at), _) => at,
                        (None, Scope::Keys(KeyScope { selection, tabs })) => selection
                            .and_then(|selection| tree::viewed_tab(tabs, selection))
                            .map_or_else(MoveTab::top, MoveTab::After),
                        (None, Scope::Cli) => MoveTab::top(),
                    };
                    let body = CreateTab {
                        at,
                        name: name.into(),
                        starter,
                    };
                    Outcome::Tab(self.create_tab(&body).await?)
                }
                TabCommand::Get(tab::Get { tab }) => {
                    Outcome::Tab(self.get::<Tab>(self.tab_target(tab, scope).await?).await?)
                }
                TabCommand::Rename(tab::Rename { tab, name }) => {
                    let id = self.tab_target(tab, scope).await?;
                    Outcome::Tab(self.rename::<Tab>(id, name.as_deref()).await?)
                }
                TabCommand::Close(tab::Close { tab }) => {
                    self.remove::<Tab>(self.tab_target(tab, scope).await?)
                        .await?;
                    Outcome::Closed
                }
                TabCommand::Move(tab::Move { tab, to }) => {
                    let id = self.tab_target(tab, scope).await?;
                    let to = to.resolve()?.unwrap_or_else(MoveTab::top);
                    Outcome::Tab(self.move_tab(id, &to).await?)
                }
            },
            Command::Pane(command) => match command {
                PaneCommand::Create(pane::Create {
                    tab,
                    direction,
                    name,
                    cwd,
                    command,
                }) => {
                    let at = Self::pane_at(tab, scope)?;
                    let cwd = match (cwd, scope) {
                        (Some(cwd), _) => Some(cwd),
                        (None, Scope::Cli) => Some(current_dir()?),
                        (None, Scope::Keys(_)) => None,
                    };
                    let cwd = cwd
                        .map(|cwd| {
                            path::absolute(&cwd).map_err(
                                |error| err!(Io, "cannot resolve '{}'", cwd.display(), @external: error),
                            )
                        })
                        .transpose()?;
                    let input = PaneInput {
                        name: name.into(),
                        spec: PaneSpec {
                            command: Some(command).filter(|command| !command.is_empty()),
                            cwd: cwd.map(|cwd| cwd.display().to_string()),
                        },
                    };
                    Outcome::Pane(
                        self.create_pane(&CreatePane {
                            at,
                            direction: direction.unwrap_or_default(),
                            input,
                        })
                        .await?,
                    )
                }
                PaneCommand::Get(pane::Get { pane }) => {
                    Outcome::Pane(self.get::<Pane>(Self::pane_target(pane, scope)?).await?)
                }
                PaneCommand::Rename(pane::Rename { pane, name }) => {
                    let id = Self::pane_target(pane, scope)?;
                    Outcome::Pane(self.rename::<Pane>(id, name.as_deref()).await?)
                }
                PaneCommand::Close(pane::Close { pane }) => {
                    self.remove::<Pane>(Self::pane_target(pane, scope)?).await?;
                    Outcome::Closed
                }
                PaneCommand::Resize(_) => {
                    return Err(err!(Unavailable, "pane resize is not available yet"));
                }
            },
        })
    }

    /// `--tab`, else the tab holding `--pane` (looked up in the keys' tabs,
    /// or in `GET /tabs` on the CLI), else the scope's.
    async fn tab_target(&self, target: TabTarget, scope: &Scope) -> Result<IdOf<Tab>> {
        match (target, scope) {
            (TabTarget { id: Some(id), .. }, _) => Ok(id),
            (
                TabTarget {
                    pane: Some(pane), ..
                },
                Scope::Keys(KeyScope { tabs, .. }),
            ) => tree::pane_owner(tabs, pane),
            (
                TabTarget {
                    pane: Some(pane), ..
                },
                Scope::Cli,
            ) => tree::pane_owner(&self.tabs().await?, pane),
            (_, Scope::Keys(KeyScope { selection, tabs })) => selection
                .and_then(|selection| tree::viewed_tab(tabs, selection))
                .ok_or_else(|| err!(Validation, "no tab selected")),
            (_, Scope::Cli) => Err(err!(
                Validation,
                "pass --tab or --pane, or run inside a pane"
            )),
        }
    }

    /// Preserve the split anchor. Explicit --tab wins over --pane.
    fn pane_at(target: TabTarget, scope: &Scope) -> Result<PaneAt> {
        if let Some(tab) = target.id {
            return Ok(PaneAt::Tab(tab));
        }
        if let Some(pane) = target.pane {
            return Ok(PaneAt::Pane(pane));
        }
        match scope {
            Scope::Keys(KeyScope {
                selection: Some(NodeId::Pane(pane)),
                ..
            }) => Ok(PaneAt::Pane(*pane)),
            Scope::Keys(KeyScope {
                selection: Some(NodeId::Tab(tab)),
                ..
            }) => Ok(PaneAt::Tab(*tab)),
            Scope::Keys(_) => Err(err!(Validation, "no tab selected")),
            Scope::Cli => Err(err!(
                Validation,
                "pass --tab or --pane, or run inside a pane"
            )),
        }
    }

    fn pane_target(target: PaneTarget, scope: &Scope) -> Result<IdOf<Pane>> {
        if let Some(id) = target.id {
            return Ok(id);
        }
        match scope {
            Scope::Cli => Err(err!(Validation, "pass --pane or run inside a pane")),
            Scope::Keys(KeyScope {
                selection: Some(NodeId::Pane(pane)),
                ..
            }) => Ok(*pane),
            Scope::Keys(_) => Err(err!(Validation, "no pane selected")),
        }
    }
}

/// `$PWD` when it names the current directory, so a symlinked path such as
/// macOS's `/tmp` is kept as the user typed it, as shells do.
pub fn current_dir() -> Result<PathBuf> {
    let current = env::current_dir()
        .map_err(|error| err!(Io, "cannot read the current directory", @external: error))?;
    let real = fs::canonicalize(&current).ok();
    Ok(env::var_os("PWD")
        .map(PathBuf::from)
        .filter(|pwd| pwd.is_absolute() && real.is_some() && fs::canonicalize(pwd).ok() == real)
        .unwrap_or(current))
}
