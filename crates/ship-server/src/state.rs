use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use futures_util::future::join_all;
use indexmap::IndexMap;
use kameo::prelude::*;
use ship_core::{
    id::*,
    model::*,
    prelude::*,
    protocol::{
        AttachRequest, Attached, CreatePane, CreateTab, MoveTab, PaneInput, Replica, ViewInput,
        ViewingRecord,
    },
    screen::Size,
};
use ship_macros::Actor;
use tokio::sync::watch;
use uuid::Uuid;

use ship_core::relay::{Publish, RelayBus};

use crate::pane::{
    self, Launch, LivePanes, PaneChange, PaneCommand, PaneEnv, PaneEvent, PaneRuntime, Spawned,
};

pub(crate) mod tree;

use tree::Tabs;

pub(crate) type Viewers = IndexMap<IdOf<Attachment>, ViewingRecord>;

/// Sole owner of the tree, of every active attachment's viewing record and of
/// every pane's running program.
#[derive(Actor)]
#[actor(on_stop = Self::stop_panes)]
pub struct ServerState {
    incarnation: Uuid,
    revision: u64,
    tabs: Tabs,
    viewers: Viewers,
    /// Exactly the panes in `tabs` once each `commit` returns.
    runtimes: HashMap<IdOf<Pane>, PaneRuntime>,
    /// The runtimes' handles, written directly rather than through the bus,
    /// so streams find every pane of a replica they hold.
    live: watch::Sender<LivePanes>,
    pane_env: PaneEnv,
    bus: ActorRef<RelayBus>,
}

impl ServerState {
    pub(crate) fn new(
        bus: ActorRef<RelayBus>,
        live: watch::Sender<LivePanes>,
        pane_env: PaneEnv,
    ) -> Self {
        Self {
            incarnation: Uuid::now_v7(),
            revision: 0,
            tabs: Tabs::new(),
            viewers: Viewers::new(),
            runtimes: HashMap::new(),
            live,
            pane_env,
            bus,
        }
    }

    /// Clone `tabs` and `viewers`, run `edit` on the clones, repair every
    /// viewing record, swap the clones in, bump `revision` and publish. An
    /// `Err` from `edit` discards the clones, leaving state unchanged. Cloning
    /// copies only `Arc`s; `Arc::make_mut` copies the tabs an edit touches. A publish failure after the swap
    /// returns `Unavailable`; the edit stays committed. After publishing, each
    /// viewed tab's panes are sent its size.
    ///
    /// On both paths, runtimes whose pane is not in the resulting tree are
    /// dropped, which tears their programs down. That covers removed panes and
    /// a runtime started for an edit that failed.
    async fn commit<R>(
        &mut self,
        edit: impl FnOnce(&mut Tabs, &mut Viewers) -> Result<R>,
    ) -> Result<R> {
        let mut tabs = self.tabs.clone();
        let mut viewers = self.viewers.clone();
        let result = edit(&mut tabs, &mut viewers);
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                self.retain_runtimes();
                return Err(error);
            }
        };
        self.viewers = viewers
            .into_iter()
            .map(|(attachment, record)| (attachment, Self::repair(&self.tabs, &tabs, record)))
            .collect();
        self.tabs = tabs;
        self.retain_runtimes();
        self.revision += 1;
        tracing::debug!(revision = self.revision, "publishing replica");
        self.bus
            .tell(Publish(self.replica()))
            .await
            .map_err(|error| err!(Unavailable, "cannot publish state", @external: error))?;
        self.apply_sizes();
        Ok(result)
    }

    /// Keep `record.selection` if it is still in `new`. A removed pane goes
    /// to the next pane of its tab, else the previous one, while the tab
    /// stays (FR-017). Otherwise walk its ancestors in `old`, nearest first,
    /// and pick the first one still in `new`, else nothing.
    fn repair(old: &Tabs, new: &Tabs, record: ViewingRecord) -> ViewingRecord {
        let Some(selection) = record.selection else {
            return record;
        };
        if tree::path(new, selection).is_some() {
            return record;
        }
        // A removed pane whose tab stayed: the pane that took its place, else
        // the new last one, which was its predecessor.
        if let NodeId::Pane(pane) = selection
            && let Ok(owner) = tree::pane_owner(old, pane)
            && let (Ok(before), Ok(after)) = (tree::tab(old, owner), tree::tab(new, owner))
            && let Some(index) = before.panes.get_index_of(&pane)
            && let Some((&next, _)) = after.panes.get_index(index).or_else(|| after.panes.last())
        {
            return ViewingRecord {
                selection: Some(NodeId::Pane(next)),
                ..record
            };
        }
        let selection = tree::path(old, selection)
            .unwrap_or_default()
            .into_iter()
            .rev()
            .find(|node| tree::path(new, *node).is_some());
        ViewingRecord {
            selection,
            ..record
        }
    }

    /// Drop runtimes whose panes are gone and publish the rest's handles,
    /// before `commit` publishes the replica.
    fn retain_runtimes(&mut self) {
        let panes: HashSet<_> = tree::pane_ids(&self.tabs).collect();
        self.runtimes.retain(|pane, _| panes.contains(pane));
        self.live.send_replace(
            self.runtimes
                .iter()
                .map(|(pane, runtime)| (*pane, runtime.handle.clone()))
                .collect(),
        );
    }

    /// Each viewed tab's size: the smallest of its viewers' terminals in each
    /// dimension, one row less for the status line. Records without a
    /// selection are skipped. Derived, never stored.
    fn tab_sizes(&self) -> HashMap<IdOf<Tab>, Size> {
        let mut sizes: HashMap<IdOf<Tab>, Size> = HashMap::new();
        for record in self.viewers.values() {
            let Some(tab) = record
                .selection
                .and_then(|selection| tree::viewed_tab(&self.tabs, selection))
            else {
                continue;
            };
            let size = Size {
                cols: record.size.cols.max(1),
                rows: record.size.rows.saturating_sub(1).max(1),
            };
            sizes
                .entry(tab)
                .and_modify(|smallest| {
                    smallest.cols = smallest.cols.min(size.cols);
                    smallest.rows = smallest.rows.min(size.rows);
                })
                .or_insert(size);
        }
        sizes
    }

    /// Send each viewed tab's size to its own panes; pane tasks drop resizes
    /// that change nothing. A tab nobody views gets nothing and keeps its size.
    fn apply_sizes(&self) {
        for (tab, size) in self.tab_sizes() {
            let Ok(tab) = tree::tab(&self.tabs, tab) else {
                continue;
            };
            for pane in tab.panes.keys() {
                if let Some(runtime) = self.runtimes.get(pane) {
                    runtime.handle.commands.send(PaneCommand::Resize(size)).ok();
                }
            }
        }
    }

    /// Start a pane's program before the commit that inserts it into `tab`,
    /// and keep its runtime. It starts at the tab's size, else 80x24. Returns
    /// the pane, running, for the edit to insert.
    fn start_pane(&mut self, tab: IdOf<Tab>, input: PaneInput) -> Result<Pane> {
        let cwd = match input.spec.cwd {
            Some(cwd) => PathBuf::from(cwd),
            None => std::env::home_dir()
                .ok_or_else(|| err!(Configuration, "the server has no home directory"))?,
        };
        if !cwd.is_absolute() || !cwd.is_dir() {
            return Err(err!(
                Validation,
                "'{}' is not an existing absolute directory",
                cwd.display()
            ));
        }
        let id = Id::new();
        let Spawned { runtime, command } = pane::spawn(
            Launch {
                pane: id,
                command: input.spec.command,
                cwd: cwd.clone(),
                size: self
                    .tab_sizes()
                    .get(&tab)
                    .copied()
                    .unwrap_or(Size::FALLBACK),
            },
            &self.pane_env,
            self.bus.clone(),
        )?;
        self.runtimes.insert(id, runtime);
        Ok(Pane {
            id,
            name: input.name,
            command,
            cwd: cwd.display().to_string(),
            title: None,
            status: PaneStatus::Running,
        })
    }

    /// kameo's `on_stop`. Every pane in the tree gets the full hangup, grace
    /// and kill before the actor finishes stopping.
    async fn stop_panes(
        &mut self,
        _: WeakActorRef<Self>,
        _: ActorStopReason,
    ) -> std::result::Result<(), kameo::error::Infallible> {
        join_all(self.runtimes.drain().map(|(_, runtime)| runtime.stop())).await;
        Ok(())
    }

    pub fn replica(&self) -> Arc<Replica> {
        Arc::new(Replica {
            incarnation: self.incarnation,
            revision: self.revision,
            tabs: self.tabs.clone(),
            viewers: self.viewers.clone(),
        })
    }
}

/// GET /tabs.
pub struct ListTabs;
pub struct Get<T: Identified>(pub IdOf<T>);
pub struct Rename<T: Identified> {
    pub id: IdOf<T>,
    pub name: OptionalName,
}
pub struct Remove<T: Identified>(pub IdOf<T>);
pub struct Move {
    pub id: IdOf<Tab>,
    pub to: MoveTab,
}
pub struct Attach {
    pub attachment: IdOf<Attachment>,
    pub request: AttachRequest,
}
pub struct Detach(pub IdOf<Attachment>);
/// Whether an attachment is active, before the input route reads its body.
pub struct CheckAttachment(pub IdOf<Attachment>);
/// Replace an attachment's view.
pub struct SetView {
    pub attachment: IdOf<Attachment>,
    pub view: ViewInput,
}

impl Message<ListTabs> for ServerState {
    type Reply = Result<Tabs>;

    async fn handle(&mut self, _: ListTabs, _: &mut Context<Self, Self::Reply>) -> Self::Reply {
        Ok(self.tabs.clone())
    }
}

impl Message<CreateTab> for ServerState {
    type Reply = Result<Tab>;

    async fn handle(
        &mut self,
        create: CreateTab,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|tabs, _| {
            let tab = Tab {
                id: Id::new(),
                name: create.name,
                tabs: IndexMap::new(),
                panes: IndexMap::new(),
            };
            tree::children_mut(tabs, create.parent)?.insert(tab.id, Arc::new(tab.clone()));
            Ok(tab)
        })
        .await
    }
}

impl Message<Get<Tab>> for ServerState {
    type Reply = Result<Tab>;

    async fn handle(
        &mut self,
        Get(id): Get<Tab>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        tree::tab(&self.tabs, id).cloned()
    }
}

impl Message<Rename<Tab>> for ServerState {
    type Reply = Result<Tab>;

    async fn handle(
        &mut self,
        rename: Rename<Tab>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|tabs, _| {
            let tab = tree::tab_mut(tabs, rename.id)?;
            tab.name = rename.name;
            Ok(tab.clone())
        })
        .await
    }
}

impl Message<Remove<Tab>> for ServerState {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        Remove(id): Remove<Tab>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|tabs, _| tree::take_tab(tabs, id).map(drop))
            .await
    }
}

impl Message<Move> for ServerState {
    type Reply = Result<Tab>;

    /// A destination parent or sibling inside the moving tab, or the tab
    /// itself, is rejected before anything is taken.
    async fn handle(
        &mut self,
        Move { id, to }: Move,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|tabs, _| {
            let node = NodeId::Tab(id);
            tree::path(tabs, node).ok_or_else(|| tree::not_found(node))?;
            let target = match to {
                MoveTab::Parent(None) => None,
                MoveTab::Parent(Some(tab)) | MoveTab::Before(tab) | MoveTab::After(tab) => {
                    Some(NodeId::Tab(tab))
                }
            };
            if let Some(target) = target
                && tree::path(tabs, target)
                    .ok_or_else(|| tree::not_found(target))?
                    .contains(&node)
            {
                return Err(err!(
                    InvalidStructure,
                    "cannot move tab {} into itself or its own descendant",
                    id
                ));
            }
            let tab = tree::take_tab(tabs, id)?;
            let moved = Tab::clone(&tab);
            tree::place(tabs, tab, to)?;
            Ok(moved)
        })
        .await
    }
}

impl Message<CreatePane> for ServerState {
    type Reply = Result<Pane>;

    async fn handle(
        &mut self,
        create: CreatePane,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        tree::tab(&self.tabs, create.parent)?;
        let pane = self.start_pane(create.parent, create.input)?;
        self.commit(|tabs, _| {
            tree::tab_mut(tabs, create.parent)?
                .panes
                .insert(pane.id, pane.clone());
            Ok(pane)
        })
        .await
    }
}

impl Message<Get<Pane>> for ServerState {
    type Reply = Result<Pane>;

    async fn handle(
        &mut self,
        Get(id): Get<Pane>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        tree::pane(&self.tabs, id).cloned()
    }
}

impl Message<Rename<Pane>> for ServerState {
    type Reply = Result<Pane>;

    async fn handle(
        &mut self,
        rename: Rename<Pane>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|tabs, _| {
            let owner = tree::pane_owner(tabs, rename.id)?;
            let pane = &mut tree::tab_mut(tabs, owner)?.panes[&rename.id];
            pane.name = rename.name;
            Ok(pane.clone())
        })
        .await
    }
}

impl Message<Remove<Pane>> for ServerState {
    type Reply = Result<()>;

    async fn handle(
        &mut self,
        Remove(id): Remove<Pane>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|tabs, _| {
            let owner = tree::pane_owner(tabs, id)?;
            tree::tab_mut(tabs, owner)?.panes.shift_remove(&id);
            Ok(())
        })
        .await
    }
}

impl Message<Attach> for ServerState {
    type Reply = Result<Attached>;

    /// Commit the new record and return it with the committed replica as the
    /// stream's seed. A requested selection is kept only if it is in the tree
    /// now; otherwise nothing is selected.
    async fn handle(
        &mut self,
        Attach {
            attachment,
            request,
        }: Attach,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|tabs, viewers| {
            if viewers.contains_key(&attachment) {
                return Err(err!(Conflict, "attachment {} already exists", attachment));
            }
            let record = ViewingRecord {
                selection: request
                    .selection
                    .filter(|node| tree::path(tabs, *node).is_some()),
                size: request.size,
            };
            viewers.insert(attachment, record);
            Ok(())
        })
        .await?;
        Ok(Attached {
            attachment,
            replica: self.replica(),
        })
    }
}

impl Message<Detach> for ServerState {
    type Reply = Result<()>;

    /// Idempotent: an attachment already gone commits nothing.
    async fn handle(
        &mut self,
        Detach(attachment): Detach,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if !self.viewers.contains_key(&attachment) {
            return Ok(());
        }
        self.commit(|_, viewers| {
            viewers.shift_remove(&attachment);
            Ok(())
        })
        .await
    }
}

impl Message<CheckAttachment> for ServerState {
    type Reply = Result<u64>;

    /// The current revision, whose replica holds the attachment; 404 if it
    /// isn't active.
    async fn handle(
        &mut self,
        CheckAttachment(attachment): CheckAttachment,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if !self.viewers.contains_key(&attachment) {
            return Err(err!(NotFound, "attachment {} not found", attachment));
        }
        Ok(self.revision)
    }
}

impl Message<SetView> for ServerState {
    type Reply = Result<ViewingRecord>;

    /// The record becomes the view and is returned as stored. No selection is
    /// always accepted; a selection no longer in the tree, such as a pane
    /// removed a moment ago, leaves the record as it was. 404 if the
    /// attachment isn't active.
    async fn handle(
        &mut self,
        SetView { attachment, view }: SetView,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let record = self
            .viewers
            .get(&attachment)
            .cloned()
            .ok_or_else(|| err!(NotFound, "attachment {} not found", attachment))?;
        if let Some(selection) = view.selection
            && tree::path(&self.tabs, selection).is_none()
        {
            return Ok(record);
        }
        let record = ViewingRecord {
            selection: view.selection,
            size: view.size,
        };
        self.commit(|_, viewers| {
            viewers.insert(attachment, record.clone());
            Ok(record)
        })
        .await
    }
}

impl Message<PaneEvent> for ServerState {
    type Reply = Result<()>;

    /// Title or exit. Ignored for panes no longer in the tree.
    async fn handle(
        &mut self,
        PaneEvent { pane: id, change }: PaneEvent,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        if tree::pane(&self.tabs, id).is_err() {
            return Ok(());
        }
        self.commit(|tabs, _| {
            let owner = tree::pane_owner(tabs, id)?;
            let pane = &mut tree::tab_mut(tabs, owner)?.panes[&id];
            match change {
                PaneChange::Title(title) => pane.title = title,
                PaneChange::Exited(status) => pane.status = PaneStatus::Exited(status),
            }
            Ok(())
        })
        .await
    }
}
