use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use futures_util::future::join_all;
use indexmap::IndexMap;
use kameo::prelude::*;
use ship_core::{
    id::*,
    layout::{Layout, Ratio},
    model::*,
    prelude::*,
    protocol::{
        AttachRequest, Attached, CreatePane, CreateTab, MoveTab, PaneAt, PaneInput, Replica,
        Starter, ViewInput, ViewingRecord,
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

mod geometry;
mod layout;
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
    /// `revision`, readable outside the actor for `x-ship-revision`.
    written: Arc<AtomicU64>,
    /// Source layouts only. Geometry is absent until publication.
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
            written: Arc::default(),
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
    /// copies only `Arc`s; `Arc::make_mut` copies the tabs an edit touches.
    /// Publication separately copies viewed tabs and ancestors for geometry.
    /// A publish failure after the swap returns `Unavailable`; the edit stays committed. After publishing, each
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
        self.written.store(self.revision, Ordering::Release);
        tracing::debug!(revision = self.revision, "publishing replica");
        let replica = self.replica();
        self.bus
            .tell(Publish(replica.clone()))
            .await
            .map_err(|error| err!(Unavailable, "cannot publish state", @external: error))?;
        self.apply_sizes(&replica.tabs);
        Ok(result)
    }

    /// A closed pane lands in the sibling subtree taking its space, using
    /// the old layout. Removed tabs retain the nearest-ancestor fallback.
    fn repair(old: &Tabs, new: &Tabs, record: ViewingRecord) -> ViewingRecord {
        let Some(selection) = record.selection else {
            return record;
        };
        if tree::path(new, selection).is_some() {
            return record;
        }
        if let NodeId::Pane(pane) = selection
            && let Ok(owner) = tree::pane_owner(old, pane)
            && let (Ok(before), Ok(after)) = (tree::tab(old, owner), tree::tab(new, owner))
            && let Some(next) = before
                .layout
                .as_ref()
                .and_then(|tree| layout::successor(tree, pane))
            && after.pane(next).is_some()
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

    /// Each viewed tab's size: the smallest of its viewers' areas in each
    /// dimension. Records without a selection are skipped. Derived, never
    /// stored.
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
                cols: record.area.cols,
                rows: record.area.rows,
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

    /// Publish actual geometry on each viewed tab, including nested tabs.
    fn published_tabs(&self) -> Tabs {
        geometry::publish(&self.tabs, &self.tab_sizes())
    }

    /// Use this commit's published content sizes, not another computation.
    /// Unviewed and zoom-hidden panes keep their last terminal sizes.
    fn apply_sizes(&self, tabs: &Tabs) {
        for tab in tabs.values() {
            if let Some(geometry) = &tab.geometry {
                for (pane, geometry) in &geometry.panes {
                    if let Some(runtime) = self.runtimes.get(pane) {
                        let size = Size {
                            cols: geometry.content.width,
                            rows: geometry.content.height,
                        };
                        runtime.handle.commands.send(PaneCommand::Resize(size)).ok();
                    }
                }
            }
            self.apply_sizes(&tab.tabs);
        }
    }

    /// Start before insertion; commit cleanup owns failed-edit teardown.
    fn start_pane(&mut self, size: Size, input: PaneInput) -> Result<Pane> {
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
                size,
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

    /// The revision as each commit sets it, shared with the HTTP layer.
    pub(crate) fn written(&self) -> Arc<AtomicU64> {
        self.written.clone()
    }

    pub fn replica(&self) -> Arc<Replica> {
        Arc::new(Replica {
            incarnation: self.incarnation,
            revision: self.revision,
            tabs: self.published_tabs(),
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
        Ok(self.published_tabs())
    }
}

impl Message<CreateTab> for ServerState {
    type Reply = Result<Tab>;

    /// A starter pane starts before the one commit that places the tab with
    /// it; a missing parent or sibling then fails the commit, which drops the
    /// pane's runtime.
    async fn handle(
        &mut self,
        create: CreateTab,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let id = Id::new();
        let layout = if let Some(Starter::Shell) = create.starter {
            Some(Layout::Pane(
                self.start_pane(Size::FALLBACK, PaneInput::default())?,
            ))
        } else {
            None
        };
        let tab = Tab {
            id,
            name: create.name,
            tabs: IndexMap::new(),
            layout,
            geometry: None,
            zoomed: None,
        };
        self.commit(|tabs, _| {
            tree::place(tabs, Arc::new(tab.clone()), create.at)?;
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
        tree::tab(&self.published_tabs(), id).cloned()
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
            Ok(())
        })
        .await?;
        tree::tab(&self.published_tabs(), rename.id).cloned()
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
            tree::place(tabs, tab, to)
        })
        .await?;
        tree::tab(&self.published_tabs(), id).cloned()
    }
}

impl Message<CreatePane> for ServerState {
    type Reply = Result<Pane>;

    async fn handle(
        &mut self,
        create: CreatePane,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let (owner, explicit) = match create.at {
            PaneAt::Tab(tab) => (tab, None),
            PaneAt::Pane(pane) => (tree::pane_owner(&self.tabs, pane)?, Some(pane)),
        };
        let tab = tree::tab(&self.tabs, owner)?;
        let size = self
            .tab_sizes()
            .get(&owner)
            .copied()
            .unwrap_or(Size::FALLBACK);
        // Validate the unzoomed edit without changing shared state or starting a program.
        let mut full = tab.clone();
        full.zoomed = None;
        let geometry = geometry::tab(&full, size);
        let anchor = explicit.or_else(|| geometry.largest());
        let mut start = size;
        if let Some(anchor) = anchor {
            let frame = geometry.panes[&anchor].frame;
            let [first, second] =
                geometry::halves(frame, layout::axis(create.direction), Ratio::HALF);
            let block = ratatui::widgets::Block::bordered();
            let first = block.inner(first);
            let second = block.inner(second);
            if [first, second].iter().any(|content| {
                content.width < geometry::MIN_CONTENT.cols
                    || content.height < geometry::MIN_CONTENT.rows
            }) {
                return Err(err!(InvalidStructure, "no space for new pane"));
            }
            start = Size {
                cols: second.width,
                rows: second.height,
            };
        }
        let pane = self.start_pane(start, create.input)?;
        self.commit(|tabs, _| {
            layout::split(
                tree::tab_mut(tabs, owner)?,
                anchor,
                create.direction,
                pane.clone(),
            )?;
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
            let pane = tree::tab_mut(tabs, owner)?
                .pane_mut(rename.id)
                .ok_or_else(|| tree::not_found(NodeId::Pane(rename.id)))?;
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
            layout::close(tree::tab_mut(tabs, owner)?, id)?;
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
                area: request.area,
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
            area: view.area,
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
            let pane = tree::tab_mut(tabs, owner)?
                .pane_mut(id)
                .ok_or_else(|| tree::not_found(NodeId::Pane(id)))?;
            match change {
                PaneChange::Title(title) => pane.title = title,
                PaneChange::Exited(status) => pane.status = PaneStatus::Exited(status),
            }
            Ok(())
        })
        .await
    }
}
