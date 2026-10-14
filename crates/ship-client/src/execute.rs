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
    id::{Attachment, IdOf},
    model::{NodeId, Pane, Tab},
    prelude::*,
    protocol::{
        CreatePane, CreateTab, MoveTab, PaneAt, PaneInput, PaneSpec, ReadPane, ResizePane,
        ScrollPane, SetPane, SetTab, SwapPane, ZoomPane,
    },
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
/// after the selected tab, or under it as its child; key-made panes send no
/// directory, so the server starts them in the anchor's.
pub struct KeyScope {
    pub selection: Option<NodeId>,
    pub tabs: Tabs,
    pub attachment: Option<IdOf<Attachment>>,
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
    Text(String),
}

impl Client {
    /// Resolves targets (an explicit ID, then the scope, then an error
    /// naming the flag), calls the API and returns what it produced.
    pub async fn execute(&self, command: Command, scope: &Scope) -> Result<Outcome> {
        Ok(match command {
            Command::Tab(command) => match command {
                TabCommand::List => Outcome::Tabs(self.tabs().await?),
                TabCommand::Create(tab::Create {
                    to,
                    name,
                    starter,
                    child,
                    pane,
                    select,
                    attachment,
                }) => {
                    let select = Self::select_attachment(select, attachment, scope)?;
                    let at = match (to.resolve()?, child) {
                        (Some(_), true) => {
                            return Err(err!(Validation, "child takes no parent, before or after"));
                        }
                        (Some(at), false) => at,
                        (None, true) => self
                            .current_tab(pane, scope)
                            .await?
                            .map_or_else(MoveTab::top, |tab| MoveTab::Parent(Some(tab))),
                        (None, false) => self
                            .current_tab(PaneTarget::default(), scope)
                            .await?
                            .map_or_else(MoveTab::top, MoveTab::After),
                    };
                    let body = CreateTab {
                        at,
                        name: name.into(),
                        starter,
                        select,
                    };
                    Outcome::Tab(self.create_tab(&body).await?)
                }
                TabCommand::Get(tab::Get { tab }) => {
                    Outcome::Tab(self.get::<Tab>(self.tab_target(tab, scope).await?).await?)
                }
                TabCommand::Set(tab::Set { tab, name }) => {
                    let id = self.tab_target(tab, scope).await?;
                    Outcome::Tab(
                        self.set::<Tab>(
                            id,
                            &SetTab {
                                name: name.map(|name| Some(name).into()),
                            },
                        )
                        .await?,
                    )
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
                    right_click_passthrough,
                    cwd,
                    command,
                    select,
                    attachment,
                }) => {
                    let select = Self::select_attachment(select, attachment, scope)?;
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
                        right_click_passthrough,
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
                            select,
                        })
                        .await?,
                    )
                }
                PaneCommand::Get(pane::Get { pane }) => {
                    Outcome::Pane(self.get::<Pane>(Self::pane_target(pane, scope)?).await?)
                }
                PaneCommand::Set(pane::Set {
                    pane,
                    name,
                    right_click_passthrough,
                }) => {
                    let id = Self::pane_target(pane, scope)?;
                    Outcome::Pane(
                        self.set::<Pane>(
                            id,
                            &SetPane {
                                name: name.map(|name| Some(name).into()),
                                right_click_passthrough,
                            },
                        )
                        .await?,
                    )
                }
                PaneCommand::Close(pane::Close { pane }) => {
                    self.remove::<Pane>(Self::pane_target(pane, scope)?).await?;
                    Outcome::Closed
                }
                PaneCommand::Resize(pane::Resize {
                    pane,
                    direction,
                    amount,
                    to,
                }) => {
                    let id = Self::pane_target(pane, scope)?;
                    Outcome::Tab(
                        self.resize(
                            id,
                            &ResizePane {
                                direction,
                                amount,
                                to,
                            },
                        )
                        .await?,
                    )
                }
                PaneCommand::Scroll(pane::Scroll {
                    pane,
                    direction,
                    amount,
                }) => {
                    let id = Self::pane_target(pane, scope)?;
                    self.scroll(id, &ScrollPane { direction, amount }).await?;
                    Outcome::Closed
                }
                PaneCommand::Read(pane::Read {
                    pane,
                    lines,
                    unwrap,
                    ansi,
                }) => {
                    let id = Self::pane_target(pane, scope)?;
                    Outcome::Text(
                        self.read(
                            id,
                            &ReadPane {
                                lines,
                                unwrap,
                                ansi,
                            },
                        )
                        .await?,
                    )
                }
                PaneCommand::Zoom(pane::Zoom { pane, zoomed }) => {
                    let id = Self::pane_target(pane, scope)?;
                    Outcome::Tab(self.zoom(id, &ZoomPane { zoomed }).await?)
                }
                PaneCommand::Swap(pane::Swap { pane, other }) => {
                    let id = Self::pane_target(pane, scope)?;
                    Outcome::Tab(self.swap(id, &SwapPane { other }).await?)
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
            (
                _,
                Scope::Keys(KeyScope {
                    selection, tabs, ..
                }),
            ) => selection
                .and_then(|selection| tree::viewed_tab(tabs, selection))
                .ok_or_else(|| err!(Validation, "no tab selected")),
            (_, Scope::Cli) => Err(err!(
                Validation,
                "pass --tab or --pane, or run inside a pane"
            )),
        }
    }

    /// The tab holding `pane`, else the selected tab or the selected pane's
    /// tab on keys, else none.
    async fn current_tab(&self, pane: PaneTarget, scope: &Scope) -> Result<Option<IdOf<Tab>>> {
        Ok(match (pane.id, scope) {
            (Some(pane), Scope::Keys(KeyScope { tabs, .. })) => Some(tree::pane_owner(tabs, pane)?),
            (Some(pane), Scope::Cli) => Some(tree::pane_owner(&self.tabs().await?, pane)?),
            (
                None,
                Scope::Keys(KeyScope {
                    selection, tabs, ..
                }),
            ) => selection.and_then(|selection| tree::viewed_tab(tabs, selection)),
            (None, Scope::Cli) => None,
        })
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

    fn select_attachment(
        select: bool,
        attachment: Option<IdOf<Attachment>>,
        scope: &Scope,
    ) -> Result<Option<IdOf<Attachment>>> {
        if !select {
            return Ok(None);
        }
        attachment
            .or(match scope {
                Scope::Keys(keys) => keys.attachment,
                Scope::Cli => None,
            })
            .map(Some)
            .ok_or_else(|| err!(Validation, "pass --attachment or set SHIP_ATTACHMENT_ID"))
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
