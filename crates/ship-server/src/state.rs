use std::{
    collections::{HashMap, HashSet},
    env,
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
};

use futures_util::future::join_all;
use indexmap::IndexMap;
use kameo::{error::Infallible, prelude::*};
use ship_core::{
    id::*,
    layout::{Layout, Ratio},
    model::*,
    prelude::*,
    protocol::{
        AttachRequest, Attached, CreatePane, CreateTab, MoveTab, PaneAt, PaneInput, PaneSpec,
        Replica, ResizePane, SetPane, SetTab, Starter, SwapPane, ViewInput, ViewingRecord,
        ZoomPane,
    },
    screen::Size,
};
use ship_macros::{Actor, named, traced};
use tokio::sync::watch;
use uuid::Uuid;

use ship_core::relay::{Publish, RelayBus};

use crate::pane::{
    self, Launch, LivePanes, PaneChange, PaneCommand, PaneEnv, PaneEvent, PaneRuntime, Spawned,
};
use crate::saver::Saved;

mod geometry;
mod layout;
pub(crate) mod tree;

use tree::Tabs;

pub(crate) type Viewers = IndexMap<IdOf<Attachment>, ViewingRecord>;

/// Cells a resize moves without an amount.
const RESIZE_STEP: u16 = 1;

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
        saved: Saved,
    ) -> Self {
        let mut state = Self {
            incarnation: Uuid::now_v7(),
            revision: 0,
            written: Arc::default(),
            tabs: Tabs::new(),
            viewers: Viewers::new(),
            runtimes: HashMap::new(),
            live,
            pane_env,
            bus,
        };
        state.tabs = state.restore(saved.into_tabs());
        // Drops nothing here; publishes the restored runtimes' handles.
        state.retain_runtimes();
        state
    }

    /// Build the tree before the actor runs, keeping each saved pane's ID.
    #[traced]
    fn restore(&mut self, saved: Tabs) -> Tabs {
        saved
            .into_iter()
            .map(|(id, tab)| (id, Arc::new(self.restore_tab(Arc::unwrap_or_clone(tab)))))
            .collect()
    }

    fn restore_tab(&mut self, mut tab: Tab) -> Tab {
        tab.tabs = self.restore(tab.tabs);
        tab.geometry = None;
        let zoomed = tab.zoomed;
        let panes: Vec<_> = tab.panes().cloned().collect();
        for saved in panes {
            let cwd = match Self::pane_cwd(Some(saved.cwd.clone())) {
                Ok(_) => Some(saved.cwd.clone()),
                Err(error) => {
                    tracing::warn!(pane = %saved.id, directory = %saved.cwd, %error, "saved directory unavailable; starting in home");
                    None
                }
            };
            let input = PaneInput {
                name: saved.name,
                right_click_passthrough: saved.right_click_passthrough,
                spec: PaneSpec { command: None, cwd },
            };
            match self.start_pane(saved.id, Size::FALLBACK, input) {
                Ok(pane) => *tab.pane_mut(saved.id).expect("saved pane belongs to tab") = pane,
                Err(error) => {
                    tracing::warn!(pane = %saved.id, directory = %saved.cwd, %error, "cannot start restored pane");
                    layout::close(&mut tab, saved.id).expect("saved pane belongs to tab");
                }
            }
        }
        tab.zoomed = zoomed.filter(|id| tab.pane(*id).is_some());
        tab
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
            .tell(Publish(Arc::clone(&replica)))
            .await
            .map_err(|error| err!(Unavailable, "cannot publish state", @external: error))?;
        self.apply_sizes(&replica.tabs);
        Ok(result)
    }

    /// A closed pane lands in the sibling subtree taking its space, using
    /// the old layout. Removed tabs retain the nearest-ancestor fallback. A
    /// pane hidden by its tab's zoom then moves to the zoomed pane.
    fn repair(old: &Tabs, new: &Tabs, record: ViewingRecord) -> ViewingRecord {
        let record = Self::repair_removed(old, new, record);
        if let Some(NodeId::Pane(pane)) = record.selection
            && let Ok(owner) = tree::pane_owner(new, pane)
            && let Ok(tab) = tree::tab(new, owner)
            && let Some(zoomed) = tab.zoomed.filter(|zoomed| *zoomed != pane)
        {
            return ViewingRecord {
                selection: Some(NodeId::Pane(zoomed)),
                ..record
            };
        }
        record
    }

    fn repair_removed(old: &Tabs, new: &Tabs, record: ViewingRecord) -> ViewingRecord {
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

    /// The published copy of tab `id`.
    fn published_tab(&self, id: IdOf<Tab>) -> Result<Tab> {
        tree::tab(&self.published_tabs(), id).cloned()
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

    /// Resolve and validate separately so restore handles only directory errors
    /// with a home fallback, never shell-spawn errors.
    fn pane_cwd(cwd: Option<String>) -> Result<PathBuf> {
        let cwd = match cwd {
            Some(cwd) => PathBuf::from(cwd),
            None => env::home_dir()
                .ok_or_else(|| err!(Configuration, "the server has no home directory"))?,
        };
        if !cwd.is_absolute() || !cwd.is_dir() {
            return Err(err!(
                Validation,
                "'{}' is not an existing absolute directory",
                cwd.display()
            ));
        }
        Ok(cwd)
    }

    /// Start before insertion; commit cleanup owns failed-edit teardown.
    fn start_pane(&mut self, id: IdOf<Pane>, size: Size, input: PaneInput) -> Result<Pane> {
        let cwd = Self::pane_cwd(input.spec.cwd)?;
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
            right_click_passthrough: input.right_click_passthrough,
            command,
            cwd: cwd.display().to_string(),
            dir: None,
            title: None,
            status: PaneStatus::Running,
            program: Default::default(),
        })
    }

    /// kameo's `on_stop`. Every pane in the tree gets the full hangup, grace
    /// and kill before the actor finishes stopping.
    async fn stop_panes(
        &mut self,
        _: WeakActorRef<Self>,
        _: ActorStopReason,
    ) -> Result<(), Infallible> {
        join_all(self.runtimes.drain().map(|(_, runtime)| runtime.stop())).await;
        Ok(())
    }

    /// The revision as each commit sets it, shared with the HTTP layer.
    pub(crate) fn written(&self) -> Arc<AtomicU64> {
        Arc::clone(&self.written)
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
pub struct Set<T: Identified, B> {
    pub id: IdOf<T>,
    pub set: B,
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
pub struct Zoom {
    pub id: IdOf<Pane>,
    pub zoom: ZoomPane,
}
pub struct Swap {
    pub id: IdOf<Pane>,
    pub swap: SwapPane,
}
pub struct Resize {
    pub id: IdOf<Pane>,
    pub resize: ResizePane,
}
/// Whether an attachment is active, before the input route reads its body.
pub struct CheckAttachment(pub IdOf<Attachment>);
/// GET /views.
pub struct ListViews;
pub struct GetView(pub IdOf<Attachment>);
pub struct Select {
    pub attachment: IdOf<Attachment>,
    pub selection: Option<NodeId>,
}
/// Update an attachment's area.
pub struct SetView {
    pub attachment: IdOf<Attachment>,
    pub view: ViewInput,
}

#[named]
impl Message<ListTabs> for ServerState {
    type Reply = Result<Tabs>;

    async fn handle(&mut self, _: ListTabs, _: &mut Context<Self, Self::Reply>) -> Self::Reply {
        Ok(self.published_tabs())
    }
}

/// Select inside a create commit, before placing anything in the tree.
fn select_created(viewers: &mut Viewers, attachment: IdOf<Attachment>, node: NodeId) -> Result<()> {
    viewers
        .get_mut(&attachment)
        .ok_or_else(|| err!(NotFound, "attachment {} not found", attachment))?
        .selection = Some(node);
    Ok(())
}

#[named]
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
            Some(Layout::Pane(self.start_pane(
                Id::new(),
                Size::FALLBACK,
                PaneInput::default(),
            )?))
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
        self.commit(|tabs, viewers| {
            if let Some(attachment) = create.select {
                let node = tab
                    .panes()
                    .next()
                    .map_or(NodeId::Tab(id), |pane| NodeId::Pane(pane.id));
                select_created(viewers, attachment, node)?;
            }
            tree::place(tabs, Arc::new(tab.clone()), create.at)?;
            Ok(tab)
        })
        .await
    }
}

#[named]
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

#[named]
impl Message<Set<Tab, SetTab>> for ServerState {
    type Reply = Result<Tab>;

    async fn handle(
        &mut self,
        Set { id, set }: Set<Tab, SetTab>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let tab = tree::tab(&self.tabs, id)?;
        if let Some(name) = set.name.filter(|name| *name != tab.name) {
            self.commit(|tabs, _| {
                tree::tab_mut(tabs, id)?.name = name;
                Ok(())
            })
            .await?;
        }
        self.published_tab(id)
    }
}

#[named]
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

#[named]
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

#[named]
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
        // Keys send no directory: the anchor's current one, else its starting one.
        let mut input = create.input;
        if input.spec.cwd.is_none() {
            input.spec.cwd = anchor
                .and_then(|anchor| tab.pane(anchor))
                .map(|anchor| anchor.dir.clone().unwrap_or_else(|| anchor.cwd.clone()));
        }
        let pane = self.start_pane(Id::new(), start, input)?;
        self.commit(|tabs, viewers| {
            if let Some(attachment) = create.select {
                select_created(viewers, attachment, NodeId::Pane(pane.id))?;
            }
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

#[named]
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

#[named]
impl Message<Set<Pane, SetPane>> for ServerState {
    type Reply = Result<Pane>;

    async fn handle(
        &mut self,
        Set { id, set }: Set<Pane, SetPane>,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let mut pane = tree::pane(&self.tabs, id)?.clone();
        let name = set.name.unwrap_or_else(|| pane.name.clone());
        let passthrough = set
            .right_click_passthrough
            .map_or(pane.right_click_passthrough, |switch| {
                switch.apply(pane.right_click_passthrough)
            });
        if pane.name != name || pane.right_click_passthrough != passthrough {
            pane.name = name;
            pane.right_click_passthrough = passthrough;
            self.commit(|tabs, _| {
                let owner = tree::pane_owner(tabs, id)?;
                *tree::tab_mut(tabs, owner)?
                    .pane_mut(id)
                    .ok_or_else(|| tree::not_found(NodeId::Pane(id)))? = pane.clone();
                Ok(())
            })
            .await?;
        }
        Ok(pane)
    }
}

#[named]
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

#[named]
impl Message<Zoom> for ServerState {
    type Reply = Result<Tab>;

    /// Absent toggles the pane; `false` unzooms its tab and `true` zooms it.
    /// A request that changes nothing commits nothing. Answers with the tab.
    async fn handle(
        &mut self,
        Zoom { id, zoom }: Zoom,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let owner = tree::pane_owner(&self.tabs, id)?;
        let current = tree::tab(&self.tabs, owner)?.zoomed;
        let zoomed = match zoom.zoomed.unwrap_or(current != Some(id)) {
            true => Some(id),
            false => None,
        };
        if zoomed != current {
            self.commit(|tabs, _| {
                tree::tab_mut(tabs, owner)?.zoomed = zoomed;
                Ok(())
            })
            .await?;
        }
        self.published_tab(owner)
    }
}

#[named]
impl Message<Swap> for ServerState {
    type Reply = Result<Tab>;

    /// Exchanges the named pair, whatever their positions, if both are in
    /// one tab; otherwise nothing changes. Answers with the tab.
    async fn handle(
        &mut self,
        Swap { id, swap }: Swap,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let owner = tree::pane_owner(&self.tabs, id)?;
        tree::pane(&self.tabs, swap.other)?;
        self.commit(|tabs, _| layout::swap(tree::tab_mut(tabs, owner)?, id, swap.other))
            .await?;
        self.published_tab(owner)
    }
}

#[named]
impl Message<Resize> for ServerState {
    type Reply = Result<Tab>;

    /// Moves the pane's edge at the tab's size, the 80x24 fallback when
    /// nobody views it. An edge that can't move commits nothing. Answers with
    /// the tab.
    async fn handle(
        &mut self,
        Resize { id, resize }: Resize,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        let change = match (resize.amount, resize.to) {
            (Some(_), Some(_)) => {
                return Err(err!(
                    Validation,
                    "resize accepts either amount or to, not both"
                ));
            }
            (_, Some(to)) => layout::Change::To(to),
            (amount, None) => layout::Change::By(amount.unwrap_or(RESIZE_STEP)),
        };
        let owner = tree::pane_owner(&self.tabs, id)?;
        let size = self
            .tab_sizes()
            .get(&owner)
            .copied()
            .unwrap_or(Size::FALLBACK);
        let mut tab = tree::tab(&self.tabs, owner)?.clone();
        if layout::resize(&mut tab, size, id, resize.direction, change)? {
            self.commit(|tabs, _| {
                *tree::tab_mut(tabs, owner)? = tab;
                Ok(())
            })
            .await?;
        }
        self.published_tab(owner)
    }
}

#[named]
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

#[named]
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

#[named]
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

#[named]
impl Message<ListViews> for ServerState {
    type Reply = Result<Viewers>;

    async fn handle(&mut self, _: ListViews, _: &mut Context<Self, Self::Reply>) -> Self::Reply {
        Ok(self.viewers.clone())
    }
}

#[named]
impl Message<GetView> for ServerState {
    type Reply = Result<ViewingRecord>;

    async fn handle(
        &mut self,
        GetView(attachment): GetView,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.viewers
            .get(&attachment)
            .cloned()
            .ok_or_else(|| err!(NotFound, "attachment {} not found", attachment))
    }
}

#[named]
impl Message<Select> for ServerState {
    type Reply = Result<ViewingRecord>;

    /// Validate both IDs before editing. Unzoom a hidden pane's tab in the
    /// same commit so selection repair preserves the requested pane.
    async fn handle(
        &mut self,
        Select {
            attachment,
            selection,
        }: Select,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|tabs, viewers| {
            let record = viewers
                .get_mut(&attachment)
                .ok_or_else(|| err!(NotFound, "attachment {} not found", attachment))?;
            if let Some(node) = selection {
                tree::path(tabs, node).ok_or_else(|| tree::not_found(node))?;
                if let NodeId::Pane(pane) = node {
                    let owner = tree::pane_owner(tabs, pane)?;
                    if tree::tab(tabs, owner)?
                        .zoomed
                        .is_some_and(|zoomed| zoomed != pane)
                    {
                        tree::tab_mut(tabs, owner)?.zoomed = None;
                    }
                }
            }
            record.selection = selection;
            Ok(())
        })
        .await?;
        Ok(self.viewers[&attachment].clone())
    }
}

#[named]
impl Message<SetView> for ServerState {
    type Reply = Result<ViewingRecord>;

    /// Update only the area, preserving selection. 404 for an inactive attachment.
    async fn handle(
        &mut self,
        SetView { attachment, view }: SetView,
        _: &mut Context<Self, Self::Reply>,
    ) -> Self::Reply {
        self.commit(|_, viewers| {
            let record = viewers
                .get_mut(&attachment)
                .ok_or_else(|| err!(NotFound, "attachment {} not found", attachment))?;
            record.area = view.area;
            Ok(record.clone())
        })
        .await
    }
}

#[named]
impl Message<PaneEvent> for ServerState {
    type Reply = Result<()>;

    /// Title, directory or exit. Ignored for panes no longer in the tree.
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
                PaneChange::Dir(dir) => pane.dir = Some(dir),
                PaneChange::Exited(status) => {
                    pane.status = PaneStatus::Exited(status);
                    pane.program.clear();
                }
                PaneChange::Program(records) => pane.program = records,
            }
            Ok(())
        })
        .await
    }
}
