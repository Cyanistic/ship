# Layout and sidebar program

Status: Locked on 2026-10-10, approved by Cyan in chat ("yep! that's approved to me!") together with product A-2 and architecture A-2/A-3. Written on 2026-10-09 from the locked [product](product.md) paper (with amendment A-1) and the originally locked [architecture](architecture.md) paper, against the crates at 79c091b. Revised for architecture amendment A-2. Revised on 2026-10-10 for Cyan's answers to the seven planning gates (G1 to G7 in `../design.md`), given in chat ("i agree withll of those! go for it!"); they are recorded in the deviation log. Nothing here is created source, and no snippet has been compiled. Approval doesn't start implementation; Cyan requests that separately, slice by slice.

Needs Cyan's attention:

- **Pane creation names its place on the wire, not in `PaneInput`, accepted by Cyan (G1).** Architecture decision 8 says `PaneInput` gains the anchor. But `PaneInput` is also the starter pane of a new tab, where an anchor means nothing. So `CreatePane`'s `parent` becomes `at`, either a tab or a pane, beside a `direction`, and `PaneInput` stays as it is. This mirrors `CreateTab`'s `at`. An anchor and a parent tab can't disagree, because only one is sent.
- **Swap target resolution is client-side, confirmed by Cyan.** `POST /panes/{id}/swap` takes only `{other}`. The server validates same-tab membership and exchanges the two leaves. `client.pane.swap` resolves a direction with published geometry and local recent selections. In a zoomed tab it unzooms first and resolves on the unzoomed geometry, staying unzoomed when no neighbor is found (G6). `ship pane swap` takes only an explicit `--other`; the CLI has no directional form, because directional swap is a `client.` action (G7). The earlier server-direction/recency proposal is superseded by product A-2 and architecture A-3, both locked on 2026-10-10.
- **Zoom's desired state is hidden from the CLI and config.** `pane::Zoom` gets a `zoomed` field that clap and serde skip, so only the client's unzoom-before-move sets it. `ship pane zoom` and `server.pane.zoom = {}` stay toggles, as the product paper says. Exposing it (`--on`/`--off`) would be a product change.
- **`--child` borrows `--pane`, accepted by Cyan (G1).** `tab create --child` needs the current pane on the CLI, and clap is the only reader of `SHIP_PANE_ID` (keys and config A-3). So `tab::Create` flattens a `PaneTarget`, which adds `--pane` to `ship tab create`, meaning "the tab of this pane" when `--child` is given and ignored otherwise. It can't be an error without `--child`: inside a pane the environment fills `--pane` on every plain `ship tab create`.
- **The formerly ⚠ choices are settled (G2 to G5):** user edits keep at least 1x1 content, refusing splits and clamping resizes; an omitted resize amount is 1 cell; hints label each chord with its action's value, else its name; and a `ClientSettings` struct owns `[client]`. See "Settled choices" under the build order. Cyan chose a floating-point fraction for the ratio and squeeze-to-fit behavior when the terminal shrinks; these are recorded below. The architecture amendments are locked.

## Rationale

The skeleton follows the architecture's structure, one box per file, with one refinement: the layout types and the neighbor search sit in ship-core, beside `TabGeometry`, the type the search reads. Only the interactive client calls the search, for focus and swap. The server does not resolve swap directions.

- **`ship-core/src/layout.rs` (new):** `Layout`, `Split`, `Axis` and `Ratio`, with the read-only walks every reader of `Tab.panes` switches to (architecture decisions 2 and 3).
- **`ship-core/src/geometry.rs` (new):** `TabGeometry` and `PaneGeometry` on the wire, and `TabGeometry::neighbor`, the directional search (decisions 1 and 12).
- **`ship-server/src/state/layout.rs` (new):** the tree edits: split, close, swap, resize, each one local rewrite (decision 2).
- **`ship-server/src/state/geometry.rs` (new):** tree and size to rectangles with ratatui's `Layout` (decisions 4 and 5).
- **`ship-server/src/state.rs`:** `commit` publishes geometry and sizes each pane to its own content, `repair` follows the tree on close and onto the zoomed pane (decisions 6 and 11), and new handlers serve zoom, swap and resize.
- **`ship-client/src/ui/panes.rs`, `ui/sidebar.rs` and `ui/memory.rs` (new):** the three client boxes (decision 12).
- **Commands, routes and `execute`:** `pane zoom`, `pane swap`, a working `pane resize`, `tab create --child`, and pane creation that splits an anchor (decisions 7 to 9).

Estimated change: implementation Rust grows by about 900 to 1,200 lines from today's 6,463. The layout model and walks account for about 150, geometry and the neighbor search about 150, the server's edits and handlers about 300, the client's panes, sidebar and memory about 300, and commands, routes, API wrappers and `execute` about 200. That's an estimate from the declarations below. The docs slice reports the measured number. Tests stay at zero.

## Skeleton map

### Proposed settings

```toml
# Cargo.toml, workspace
[workspace.dependencies]
ratatui-core = { version = "0.1", features = ["serde"] }   # already in the tree through ratatui 0.30
tui-tree-widget = "0.24"                                     # built on ratatui-core 0.1 and ratatui-widgets 0.3

# crates/ship-core/Cargo.toml
ratatui-core.workspace = true

# crates/ship-client/Cargo.toml
tui-tree-widget.workspace = true
```

ship-core uses `ratatui_core::layout::Rect` on the wire instead of a copy of it, since both the server and the client already speak it and ratatui-core's `serde` feature derives it (checked in `ratatui-core-0.1.2/src/layout/rect.rs`). It serializes as `{"x", "y", "width", "height"}`.

The config gains one setting, `[client] sidebar_width = 26`.

### Added

#### `crates/ship-core/src/layout.rs` (slice 1)

```rust
//! A tab's panes and where they go: a binary split tree whose leaves are the
//! panes. The only record of which panes a tab has and in what order.

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::{id::IdOf, model::Pane};

/// A pane, or two layouts side by side or stacked.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Layout {
    Pane(Pane),
    Split(Split),
}

/// Two layouts sharing an area along `axis`; `ratio` of it goes to `first`.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Split {
    pub axis: Axis,
    pub ratio: Ratio,
    pub first: Box<Layout>,
    pub second: Box<Layout>,
}

/// As ratatui's `Direction`: `Horizontal` puts `first` left of `second`,
/// `Vertical` puts it above.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum Axis { Horizontal, Vertical }

/// The first child's share of the whole split: 0.5 means equal halves,
/// 0.7 means first gets 70% and second gets the remaining 30%. For a
/// horizontal split first is left; for a vertical split first is top.
/// A floating-point fraction, not cells or first divided by second.
/// Must be finite and strictly between 0.0 and 1.0.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize, ToSchema)]
#[serde(try_from = "f32", into = "f32")]
pub struct Ratio(f32);

impl Ratio {
    pub const HALF: Self; // 0.5
}

impl Layout {
    /// The leaves, first child before second: the tab's pane order (FR-018).
    pub fn panes(&self) -> impl Iterator<Item = &Pane>;
    pub fn pane(&self, id: IdOf<Pane>) -> Option<&Pane>;
    pub fn pane_mut(&mut self, id: IdOf<Pane>) -> Option<&mut Pane>;
}
```

`Layout`'s schema recurses through `Box<Layout>`. If utoipa's derive inlines it without end, as it did for `Tabs`, it gets a hand-written schema like `tabs_schema`.

#### `crates/ship-core/src/geometry.rs` (slice 1; `neighbor` in slice 2)

```rust
//! Where a viewed tab's panes are, computed by the server once per commit
//! and published in the replica (architecture decision 1). Derived, never
//! stored in the model.

use indexmap::IndexMap;
use ratatui_core::layout::Rect;

use crate::{command::Direction, id::IdOf, model::Pane, screen::Size};

/// One viewed tab at its size. Rectangles are relative to the tab area's
/// top-left corner; the client offsets them by its sidebar.
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct TabGeometry {
    pub size: Size,
    /// The visible panes in pane order: all of them, or only the zoomed one.
    #[schema(schema_with = pane_geometry_schema)]
    pub panes: IndexMap<IdOf<Pane>, PaneGeometry>,
}

/// With two or more visible panes, `frame` includes the border it shares
/// with its neighbors and `content` is the frame minus the border. A lone or
/// zoomed pane has no border, so both are the whole area.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PaneGeometry {
    #[schema(schema_with = rect_schema)]
    pub frame: Rect,
    #[schema(schema_with = rect_schema)]
    pub content: Rect,
}

impl TabGeometry {
    /// The pane adjacent to `from` in `direction`: frames that share some of
    /// `from`'s edge on that side. Ties go to the first of `recent` among
    /// them, else the nearest center to `from`'s, else pane order (FR-004,
    /// FR-007).
    pub fn neighbor(
        &self,
        from: IdOf<Pane>,
        direction: Direction,
        recent: &[IdOf<Pane>],
    ) -> Option<IdOf<Pane>>;

    /// The visible pane with the largest content area, ties to pane order
    /// (FR-019).
    pub fn largest(&self) -> Option<IdOf<Pane>>;
}

fn rect_schema() -> Object;
fn pane_geometry_schema() -> Object;
```

#### `crates/ship-server/src/state/layout.rs` (slice 1; `swap` and `resize` in slice 4)

```rust
//! Edits on one tab's split tree, each a local rewrite run inside a commit's
//! edit closure. Structural edits clear the tab's zoom first (decision 6).

use ship_core::{
    command::Direction,
    geometry::TabGeometry,
    layout::{Axis, Layout, Ratio, Split},
    model::{Pane, Tab},
    protocol::SplitDirection,
};

/// Replaces `anchor`'s leaf with a half-and-half split holding it and `pane`,
/// `pane` second. A tab with no layout gets `pane` as its whole layout and
/// `anchor` must be `None`. Doesn't check size: `Message<CreatePane>` refuses
/// an anchor too small for two `MIN_CONTENT` halves before starting the pane
/// (R-2, G2).
pub(crate) fn split(
    tab: &mut Tab,
    anchor: Option<IdOf<Pane>>,
    direction: SplitDirection,
    pane: Pane,
) -> Result<()>;

/// Removes `pane`'s leaf and replaces its parent split with the sibling, or
/// empties the layout when it was the only pane. `NotFound` if absent.
pub(crate) fn close(tab: &mut Tab, pane: IdOf<Pane>) -> Result<Pane>;

/// Exchanges two leaves in place; both panes keep running.
pub(crate) fn swap(tab: &mut Tab, a: IdOf<Pane>, b: IdOf<Pane>) -> Result<()>;

/// Moves the edge `pane` has on the `direction` side by `cells` that way, or,
/// when that side is the tab's edge, the opposite edge that way (tmux). The
/// edge belongs to the nearest ancestor split on that axis; its ratio moves
/// by `cells` over that split's size in `geometry`, clamped so every pane
/// keeps `MIN_CONTENT` (R-2). An edge that can't move is a no-op.
pub(crate) fn resize(
    tab: &mut Tab,
    geometry: &TabGeometry,
    pane: IdOf<Pane>,
    direction: Direction,
    cells: u16,
) -> Result<()>;

/// For repair on close, read from the old tree: the pane in `closed`'s
/// sibling subtree nearest the closed side, descending toward it at splits
/// on the parent's axis and taking the first child otherwise (decision 11).
/// `None` when `closed` was the only pane.
pub(crate) fn successor(layout: &Layout, closed: IdOf<Pane>) -> Option<IdOf<Pane>>;
```

#### `crates/ship-server/src/state/geometry.rs` (slice 1)

```rust
//! A tab's tree and size to rectangles (decisions 4 and 5).

use ratatui::layout::{Constraint, Direction as TuiDirection, Layout as TuiLayout, Rect, Spacing};
use ship_core::{geometry::{PaneGeometry, TabGeometry}, model::Tab, screen::Size};

/// The minimum content for splits and manual resizes: 1 column and 1 row,
/// a mechanical limit rather than a comfortable one (R-2, G2). A split that
/// would leave either half below it is refused; a resize is clamped to it.
/// This does not constrain terminal shrink; existing panes squeeze into the
/// available area even below this size.
pub(crate) const MIN_CONTENT: Size = Size { cols: 1, rows: 1 };

/// Splits divide their area from the floating-point fraction, rounding to
/// whole cells through ratatui's layout constraints. The R-1 probe checks
/// the constraint mapping and cell-based resize behavior before use.
/// Use `Spacing::Overlap(1)` when bordered, so neighbors share one cell. Two
/// or more visible panes are bordered and `content` is `frame` shrunk by one
/// cell on each side; a zoomed tab is its zoomed pane alone, unbordered.
pub(crate) fn tab(tab: &Tab, size: Size) -> TabGeometry;
```

#### `crates/ship-client/src/ui/panes.rs` (slice 2)

```rust
//! The pane area: frames, labels and screens, from the published geometry.

use ratatui::{Frame, layout::Rect, widgets::{Block, BorderType, MergeStrategy}};
use ship_core::{geometry::TabGeometry, model::Tab};

/// Each visible pane's frame, offset to `area`. Bordered frames get
/// `Block::merge_borders(MergeStrategy::Exact)`, the pane's label in the top
/// border and, on the selected pane, a distinct border style (FR-002). A
/// zoomed tab's lone pane gets a zoom marker in the bottom bar instead,
/// since it has no border. Each screen paints into its content rectangle,
/// with the filler where the screen is smaller (today's `fill` and `paint`,
/// moved here).
pub(super) fn draw(
    frame: &mut Frame,
    area: Rect,
    tab: &Tab,
    geometry: &TabGeometry,
    screens: &HashMap<IdOf<Pane>, Arc<Screen>>,
    selected: Option<IdOf<Pane>>,
);

/// Where the selected pane's cursor goes on the client's terminal.
pub(super) fn cursor(area: Rect, geometry: &TabGeometry, pane: IdOf<Pane>, screen: &Screen) -> Option<(u16, u16)>;
```

#### `crates/ship-client/src/ui/memory.rs` (slice 2)

```rust
//! What this client selected, never sent anywhere (decision 12).

#[derive(Default)]
pub(super) struct Memory {
    /// The pane this client last selected in each tab.
    last: HashMap<IdOf<Tab>, IdOf<Pane>>,
    /// Panes in the order this client last selected them, newest first.
    recent: Vec<IdOf<Pane>>,
}

impl Memory {
    /// Called with each selection the client sees in its record.
    pub fn selected(&mut self, tabs: &Tabs, selection: NodeId);
    /// Where selecting `tab` lands: its zoomed pane, else this client's last
    /// pane there, else its first pane, else the tab (FR-012).
    pub fn landing(&self, tab: &Tab) -> NodeId;
    pub fn recent(&self) -> &[IdOf<Pane>];
    /// Forgets panes and tabs no longer in the replica.
    pub fn retain(&mut self, tabs: &Tabs);
}
```

#### `crates/ship-client/src/ui/sidebar.rs` (slice 3)

```rust
//! The "tabs" title and the tab tree (FR-009 to FR-011, FR-013).

use tui_tree_widget::{Tree, TreeItem, TreeState};

/// Client-local presentation state.
pub(super) struct Sidebar {
    /// Folded tabs. Everything else is expanded, so a new tab shows up
    /// unfolded. The selection's ancestors are expanded whatever this says.
    collapsed: HashSet<IdOf<Tab>>,
    pub shown: bool,
    pub width: u16,
}

impl Sidebar {
    /// Rows under expanded ancestors, in tree order, whether or not the
    /// sidebar is shown. What `client.tab.next` and `prev` walk.
    pub fn visible(&self, tabs: &Tabs, selection: Option<NodeId>) -> Vec<IdOf<Tab>>;
    pub fn collapse(&mut self, tab: IdOf<Tab>);
    pub fn expand(&mut self, tab: IdOf<Tab>);
    /// Builds `TreeItem`s with the derived labels and a `TreeState` opened
    /// to match `collapsed` and selected on the selection's tab, then renders
    /// `Tree` under the title. The state is rebuilt each draw, since
    /// `collapsed` is the record of folds; the widget scrolls to the
    /// selected row itself.
    pub fn draw(&self, frame: &mut Frame, area: Rect, tabs: &Tabs, selection: Option<NodeId>);
}
```

### Moved

- `fill`, `paint`, `style` and `color`: from `crates/ship-client/src/ui/draw.rs` to `ui/panes.rs` (slice 2).
- `command::pane::Split`: from `crates/ship-core/src/command.rs` to `protocol.rs` as `SplitDirection`, since it now crosses the wire, with the `clap` derive behind the feature like `Starter`. The new name avoids `layout::Split` (slice 1).

### Replaced

#### `crates/ship-core/src/model.rs` (slices 1 and 5)

```rust
pub struct Tab {
    pub id: IdOf<Tab>,
    #[serde(default)]
    pub name: OptionalName,
    #[schema(schema_with = tabs_schema)]
    pub tabs: Tabs,
    /// Absent while the tab has no panes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub layout: Option<Layout>,                   // replaces `panes: IndexMap<IdOf<Pane>, Pane>`
    /// The pane filling the tab, while zoomed. Always a pane in `layout`:
    /// every edit that removes a pane clears it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zoomed: Option<IdOf<Pane>>,
}

impl Tab {
    /// `layout`'s panes in order, or none.
    pub fn panes(&self) -> impl Iterator<Item = &Pane>;
    pub fn pane(&self, id: IdOf<Pane>) -> Option<&Pane>;
}

pub struct Pane {
    // …as today, plus (slice 5):
    /// The directory the shell last reported through OSC 7; absent until it
    /// reports one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dir: Option<String>,
}
```

`panes_schema` is deleted.

#### `crates/ship-core/src/tree.rs` (slice 1)

`tree::pane`, `pane_ids` and `pane_owner` walk `Tab::panes()` instead of the map. `first_pane` is deleted: it descended into child tabs, which FR-012 rules out, and its two callers in `ui/mod.rs` use `Memory::landing` or `tab.panes().next()`.

#### `crates/ship-core/src/protocol.rs` (slices 1 and 4)

```rust
pub const PROTOCOL_VERSION: u32 = 8;              // 8: layout, geometry, CreatePane's `at`

/// POST /panes body, e.g. `{"at": {"pane": "pane:…"}, "direction": "down"}`.
pub struct CreatePane {
    pub at: PaneAt,                               // replaces `parent: IdOf<Tab>`
    #[serde(default)]
    pub direction: SplitDirection,
    #[serde(flatten)]
    pub input: PaneInput,                         // unchanged
}

/// Where a new pane goes: next to a pane, or into a tab, splitting its
/// largest pane (FR-019).
#[serde(rename_all = "camelCase")]
pub enum PaneAt { Tab(IdOf<Tab>), Pane(IdOf<Pane>) }

#[derive(Default)]
#[cfg_attr(feature = "clap", derive(clap::ValueEnum))]
#[serde(rename_all = "snake_case")]
pub enum SplitDirection { #[default] Right, Down }

/// POST /panes/{id}/zoom body. Absent toggles (slice 4).
pub struct ZoomPane { #[serde(default)] pub zoomed: Option<bool> }

/// POST /panes/{id}/swap body (slice 4).
pub struct SwapPane {
    /// Explicit target in the same tab; adjacency is not required.
    pub other: IdOf<Pane>,
}

/// POST /panes/{id}/resize body (slice 4).
pub struct ResizePane {
    pub direction: Direction,
    /// Cells; absent is `RESIZE_STEP`, 1 cell (G5).
    #[serde(default)]
    pub amount: Option<u16>,
}

pub struct Replica {
    // …as today, plus:
    /// Each viewed tab's geometry at its size. Derived in `commit`.
    #[schema(schema_with = geometry_schema)]
    pub geometry: IndexMap<IdOf<Tab>, TabGeometry>,
}
```

#### `crates/ship-core/src/command.rs` (slices 1, 4 and 5)

```rust
pub mod tab {
    pub struct Create {
        pub to: Destination,
        pub name: Option<String>,
        pub starter: Option<Starter>,
        /// Make the tab the last child of the current tab: the selected one
        /// on keys, the one holding --pane on the CLI (slice 5)
        #[cfg_attr(feature = "clap", arg(long, conflicts_with_all = ["parent", "before", "after"]))]
        pub child: bool,
        #[cfg_attr(feature = "clap", command(flatten))]
        pub pane: PaneTarget,
    }
}

pub mod pane {
    pub enum PaneCommand {
        Create(Create),
        Get(Get),
        Rename(Rename),
        Close(Close),
        /// Resize a pane by moving its edge
        Resize(Resize),                           // doc no longer "not available yet"
        /// Zoom a pane to fill its tab, or unzoom it (slice 4)
        Zoom(Zoom),
        /// Exchange a pane with another pane in its tab (slice 4)
        Swap(Swap),
    }

    pub struct Create {
        /// The pane to split, or the tab whose largest pane splits
        pub tab: TabTarget,                       // `--pane` now anchors instead of naming its tab
        /// Which side of the anchor the pane goes; right by default
        pub direction: Option<SplitDirection>,
        // name, cwd, command as today
    }

    pub struct Zoom {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub pane: PaneTarget,
        /// Set only by the client's unzoom before a move (decision 7).
        #[cfg_attr(feature = "clap", arg(skip))]
        #[serde(skip)]
        pub zoomed: Option<bool>,
    }

    pub struct Swap {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub pane: PaneTarget,
        /// The pane to exchange with; adjacency is not required. Directional
        /// swap is `client.pane.swap`, a key action with no CLI form (G7).
        #[cfg_attr(feature = "clap", arg(long))]
        pub other: IdOf<Pane>,
    }
}
```

#### `crates/ship-server/src/state.rs` (slices 1, 2, 4 and 5)

- `commit`: after the swap, builds the replica with `geometry` for each tab in `tab_sizes`, publishes it, then `apply_sizes(&replica.geometry)`.
- `apply_sizes(&IndexMap<IdOf<Tab>, TabGeometry>)`: each visible pane gets its content size; hidden and unviewed panes get nothing and keep theirs (decisions 4 and 6).
- `replica()` computes geometry the same way, for `Attach`'s seed.
- `repair(old, new, record)`: a closed selected pane goes to `layout::successor` in the old tree, else the ancestor rule as today; then a selected pane hidden by its tab's `zoomed` goes to the zoomed pane (slice 4).
- `start_pane(size: Size, cwd: Option<String>, …)`: the caller passes the size, the anchor's content size, else the tab's size, else 80x24. The commit's resize corrects it. Its cwd rule moves to `CreatePane` (below).
- `Message<CreatePane>`: resolves `at` to a tab and an anchor (`PaneAt::Pane` names it; `PaneAt::Tab` takes `TabGeometry::largest` at the tab's size, else 80x24), refuses with `InvalidStructure` ("no space for new pane") when either half would get less than `MIN_CONTENT`, resolves cwd (given, else the anchor's `dir`, else its `cwd`, else home; `dir` from slice 5), starts the pane, and commits `layout::split`. The refusal comes before the pane starts, so nothing leaks.
- `Message<CreateTab>`: the starter pane becomes the layout.
- `Message<Remove<Pane>>`: `layout::close` on the owner.
- `Message<Rename<Pane>>` and `Message<PaneEvent>`: `Layout::pane_mut` instead of indexing the map. `PaneChange::Dir` sets `dir` (slice 5).
- New `Zoom`, `Swap` and `Resize` messages (slice 4): zoom sets or clears `zoomed`; swap validates both named panes belong to the same tab, clears zoom and calls `layout::swap` in one commit, with no neighbor search. Missing panes or cross-tab pairs are rejected. Resize clears zoom, computes geometry, and calls `layout::resize` with `amount` or `RESIZE_STEP` (1 cell) cells. An immovable edge commits nothing and answers with the unchanged tab.

#### `crates/ship-server/src/state/tree.rs` (slice 1)

Re-exports stay. No edit needs the map any more, so nothing else changes.

#### `crates/ship-server/src/attach.rs` (slice 1)

`Progress::view` watches the panes in `replica.geometry` for the viewed tab, so a zoomed tab streams only its zoomed pane. A viewed tab is always in `geometry`, since `tab_sizes` covers every selection.

#### `crates/ship-server/src/pane.rs` (slice 5)

```rust
pub(crate) enum PaneChange {
    Title(Option<String>),
    /// From `SessionEvent::CwdChanged`, OSC 7's path.
    Dir(String),
    Exited(ExitStatus),
}
```

`publish` forwards `CwdChanged` beside `TitleChanged`, once per drain, only when it changed.

#### `crates/ship-server/src/routes.rs` and `lib.rs` (slices 1 and 4)

`create_pane` takes the new `CreatePane` (404 for a missing tab or anchor). `zoom_pane`, `swap_pane` and `resize_pane` are `POST /api/v0/panes/{id}/zoom`, `/swap` and `/resize`, each answering with the pane's tab, and register in `lib.rs`'s router and OpenAPI list.

#### `crates/ship-client/src/api.rs` (slices 1 and 4)

`create_pane(&CreatePane)` replaces `create_pane(parent, input)`. `zoom(pane, &ZoomPane)`, `swap(pane, &SwapPane)` and `resize(pane, &ResizePane)` return `Tab`.

#### `crates/ship-client/src/execute.rs` (slices 1, 4 and 5)

```rust
/// The client's selection and the tabs it was made in.
/// Directional swap is resolved before executing the server command.
pub struct KeyScope {
    pub selection: Option<NodeId>,
    pub tabs: Tabs,
}

impl Client {
    /// `TabTarget` to `PaneAt`: `--tab` wins, as elsewhere; else `--pane`
    /// as the anchor; else the selected pane, or the selected tab, on keys.
    fn pane_at(target: TabTarget, scope: &Scope) -> Result<PaneAt>;
}
```

- `PaneCommand::Create`: `pane_at`, the direction, and no `cwd` from keys, so the server picks it.
- `PaneCommand::Zoom` and `Resize` call their wrappers; `Outcome::Tab`.
- `PaneCommand::Swap`: calls the ID-only wrapper with `other`, on the CLI and from a bound `server.pane.swap`. There is no directional CLI form and no geometry fetch (G7). A changed layout does not cause server re-resolution: the named pair is swapped if still in the same tab.
- `TabCommand::Create` with `child`: `MoveTab::Parent(Some(tab))`, where `tab` is the selection's tab on keys or `pane`'s owner on the CLI, else the top level (FR-022).

#### `crates/ship-client/src/keymap.rs` and `keymap/defaults.toml` (slices 3 to 5)

```rust
/// All of `[client]` (G4). `load` and `defaults` move here from `Keymap`;
/// `ship config check` and the client's startup and reload in `ui/mod.rs`
/// call them instead.
pub struct ClientSettings {
    pub keymap: Keymap,
    /// `[client] sidebar_width`, default 26 (FR-020).
    pub sidebar_width: u16,
}

pub struct Keymap {
    pub modes: HashMap<ModeName, Mode>,           // unchanged
}

struct RawClient {
    #[serde(default)] modes: BTreeMap<String, IgnoredAny>,
    sidebar_width: Option<u16>,
}

impl Mode {
    /// The bottom bar's hints, in the file's order (FR-014, G3). Each chord
    /// is labeled from its binding's first action: the leaf's scalar value
    /// when it has one, else the leaf's name. `server.pane.resize.direction
    /// = "left"` is `left`, `server.pane.zoom = {}` is `zoom` and
    /// `client.mode = "normal"` is `normal`, so the default resize mode reads
    /// `h left  j down  k up  l right  esc normal`. The status line cuts
    /// hints from the end when they don't fit. No per-binding override.
    pub fn hints(&self) -> String;
}
```

`Mode` keeps its chords in an `IndexMap` so hints follow the file's order. The module doc becomes "`[client]` in the config file: settings, modes and their bindings". Defaults: `alt-1` to `alt-9` go (FR-016, slice 3); `alt-f` binds `server.pane.zoom = {}` and `alt-shift-h/j/k/l` bind `client.pane.swap.direction` (FR-021, slice 4); `alt-shift-n` binds `server.tab.create = { starter = "shell", child = true }` (FR-022, slice 5).

#### `crates/ship-client/src/ui/draw.rs` (slices 1 to 3)

- `tab_area(cols, rows, sidebar: Option<u16>)`: the terminal minus the bottom bar and, when shown, the sidebar (FR-015).
- `draw`: splits the frame into sidebar, pane area and bottom bar; the pane area goes to `panes::draw`, or the hints as today.
- `tab_label`: the first of `tab.panes()` (FR-018).
- `status_line`: the mode's `hints()` after its name, and a zoom marker on a zoomed tab.

#### `crates/ship-client/src/ui/observer.rs` (slice 1)

`selected` uses `tab.pane(id)`. `apply` reports a screen as visible when its pane is in the viewed tab's geometry, not only when selected.

#### `crates/ship-client/src/ui/mod.rs` (slices 1 to 4)

- `navigate(Step::Pane)` steps through `tab.panes()` (slice 1).
- `Step::Tab` steps through `Sidebar::visible` instead of top-level tabs, and lands on `Memory::landing` (slice 3).
- `ClientPane::Focus` takes `TabGeometry::neighbor` with `Memory::recent` (slice 2).
- `ClientPane::Swap { direction }` (new client action, slice 4) uses the same neighbor search with `Memory::recent`, then queues `server.pane.swap` with the explicit `other` ID. Recency never enters `KeyScope` or the wire. No neighbor is a no-op. On a zoomed tab it pushes `server.pane.zoom` with `zoomed: Some(false)` ahead of itself, like `Focus`, waits for the revision, then resolves on the unzoomed geometry; with no neighbor the tab stays unzoomed (G6).
- `ClientTab::Expand`, `Collapse` and `Sidebar::Toggle` work; toggling marks the view dirty, since the area changes (slice 3).
- Leaving a zoomed pane: `Focus` and `Next` on a zoomed tab push `server.pane.zoom` with `zoomed: Some(false)` ahead of themselves on the queue, which waits for the revision and then runs the move on the unzoomed tab (slice 4).
- `Memory::selected` runs whenever the record's selection changes; `Memory::retain` and the sidebar's folds follow `Observer::reconcile`.

### Untouched

- `ship-core` `id.rs`, `screen.rs`, `relay.rs` and `error.rs`. `ErrorCode::InvalidStructure` already exists for a refused split.
- `ship-server` `input.rs`, `health.rs`, `app.rs` and `settings.rs`.
- `ship-client` `ui/keys.rs`, `ui/terminal.rs` and `watch.rs`.
- `ship` binary: `cli.rs` and `commands.rs` take the new subcommands through the shared command tree with no edits of their own.
- `ship-macros`, `vendor/ratatui-ghostty`. `CwdChanged` already exists.
- `openspec/specs/*`. The change's spec deltas are written when it's archived, not in a slice.

## Build order

1. **The split tree and geometry on the server.**
   - Files: `ship-core` `layout.rs`, `geometry.rs` (without `neighbor`), `model.rs`, `tree.rs`, `protocol.rs`, `command.rs` and `lib.rs`; workspace and `ship-core` manifests (ratatui-core); `ship-server` `state.rs`, `state/layout.rs` (split, close, successor), `state/geometry.rs`, `attach.rs` and `routes.rs`; `ship-client` `api.rs`, `execute.rs`, `ui/observer.rs`, `ui/draw.rs` (`tab_label`) and `ui/mod.rs` (`navigate`).
   - First, a temporary probe outside the workspace for architecture R-1: map floating-point split fractions to ratatui constraints with `Spacing::Overlap(1)` at odd widths and heights and three-way nested splits, rendered with `merge_borders`. Check that shared edges line up, content rectangles fit the area, and repeated cell-based resize steps move the intended number of cells without drift. If it fails, stop and reopen the relevant geometry decision before going on.
   - For R-2, probe shrinking an existing layout below the 1x1 user-edit minimum, to zero-content panes, and enlarging it again. Panes stay alive, stored ratios stay unchanged, and no frame extends beyond the tab area. Verify safe handling of zero-content rectangles and PTY sizes; stop for a decision if this needs a fallback not yet approved.
   - Delivers: FR-001, FR-005, FR-018, FR-019, and the server half of FR-002 and FR-015. The client still shows only the selected pane, now at its own content size.
   - Checks:
     - Build, fmt and Clippy, with and without `ship-core`'s `clap` feature.
     - Inside a pane, `ship pane create` and `ship pane create --direction down` split it; `ship tab get` shows the nested layout, and `stty size` in each pane matches its content rectangle in the replica.
     - `ship pane create --tab <id>` splits the largest pane; on an empty tab it makes the first pane.
     - Closing a pane gives its space to its sibling, and a client that had it selected lands on the pane that took the space.
     - The R-1 probe's output is recorded in the deviation log and the probe deleted.

2. **Drawing splits and directional focus.**
   - Files: `ship-core` `geometry.rs` (`neighbor`); `ship-client` `ui/panes.rs`, `ui/memory.rs`, `ui/draw.rs` and `ui/mod.rs`.
   - Delivers: FR-002, FR-004 and the memory half of FR-012. The P1 story "See several panes at once".
   - Checks:
     - Neovim, Pi and a shell running tests side by side through repeated splitting and closing, with no corruption (SC-001), and an alternate-screen program entering and leaving cleanly (SC-002), on macOS; Linux unverified unless run there.
     - A single pane has no border; two or more have shared borders with labels, and the selected one stands out.
     - `alt-h/j/k/l` move to the adjacent pane; with two panes adjacent, the one selected last wins.
     - Typing in one split while another runs `yes` feels as responsive as a single pane today (SC-005).

3. **The sidebar.**
   - Files: `ship-client` `ui/sidebar.rs`, `ui/draw.rs`, `ui/mod.rs`, `keymap.rs` and `keymap/defaults.toml`; manifests (tui-tree-widget).
   - Delivers: FR-009 to FR-016 and FR-020. The P1 story "Move around the tab tree" and the P2 story "Hide the sidebar".
   - Checks:
     - A three-level tree folds, navigates and opens from `tabs` mode, and each tab shows only its own panes (SC-003).
     - Two clients fold differently and select different panes without affecting each other (SC-004, first half).
     - `alt-b` hides the sidebar, and on the smallest client the panes grow (`stty size`).
     - `sidebar_width = 40` widens it; `ship config check` rejects `sidebar_width = "wide"` with its key path.
     - `alt-1` reaches the pane as a plain key; `client.tab.select` still reports "not available yet".
     - In `tabs` mode the bottom bar shows `j next  k prev  h collapse  l expand  esc normal`, and `resize` mode shows `h left  j down  k up  l right  esc normal`. Rebinding a chord in the file changes its hint on reload; a narrow terminal cuts hints from the end.

4. **Zoom, swap and resize.**
   - Files: `ship-core` `command.rs` and `protocol.rs`; `ship-server` `state.rs`, `state/layout.rs` (swap, resize), `routes.rs` and `lib.rs`; `ship-client` `api.rs`, `execute.rs`, `ui/mod.rs`, `ui/draw.rs` and `keymap/defaults.toml`.
   - Delivers: FR-006 to FR-008, FR-017 and FR-021. The P2 story "Zoom, swap and resize".
   - Checks:
     - `alt-f` zooms for both clients viewing the tab, and their hidden-pane selections move to the zoomed pane (SC-004, second half).
     - `alt-h` from a zoomed pane unzooms for everyone and moves; two clients doing it at once leave the tab unzoomed.
     - Creating, closing, swapping or resizing in a zoomed tab unzooms it first.
     - `alt-shift-l` resolves and swaps with the right neighbor with both screens intact. From a zoomed pane it unzooms, then swaps; with no neighbor that way it leaves the tab unzoomed and sends no swap. `ship pane swap --pane A --other B` exchanges nonadjacent panes in the same tab, and `ship pane swap` without `--other` is rejected by clap. Cross-tab and missing targets are rejected without a partial swap. Verify requests contain only `other`, never direction or recency.
     - `alt-r` then `h/j/k/l` moves the edge one cell per press; `ship pane resize --direction right --amount 5` moves it five cells, checked with `stty size`. Resizing against a pane with 1 column of content moves as far as it can, then commits nothing. Splitting a pane too small for two 1x1 halves fails with "no space for new pane".
     - SC-001 again with zoom, swap and resize in the mix.

5. **New panes where you are, and child tabs.**
   - Files: `ship-core` `model.rs` (`dir`) and `command.rs` (`--child`); `ship-server` `pane.rs` and `state.rs`; `ship-client` `execute.rs` and `keymap/defaults.toml`.
   - Delivers: FR-003 and FR-022.
   - Checks:
     - After `cd /tmp` in a pane, `alt--` opens a shell in `/tmp`, and `ship pane get` shows `dir`; with OSC 7 off, it opens in the pane's starting directory (architecture R-5, checked in Cyan's zsh).
     - `ship pane create` from the CLI still starts in the caller's directory.
     - `alt-shift-n` makes a child tab with a shell as the selected tab's last child and selects its pane; with nothing selected, a top-level tab.
     - `ship tab create --child` inside a pane makes a child of that pane's tab; `--child --parent X` is rejected.

6. **Docs and size.**
   - Files: `README.md`, `GLOSSARY.md` (layout, zoom).
   - Delivers: the README matches the new keys, commands and config setting.
   - Checks:
     - `grep -rn 'alt-[1-9]\|not available yet' README.md` finds only `client.tab.select`.
     - Implementation Rust and test line counts are reported separately against the estimate.

Settled choices, formerly ⚠, answered by Cyan in chat on 2026-10-10:

| Choice | Decision | Slice |
| --- | --- | --- |
| Smallest pane for user edits (R-2, G2) | 1 column by 1 row of content. A split that would go below it is refused with "no space for new pane", as tmux does; a resize is clamped and commits nothing if the edge can't move. Terminal shrink may go below this limit. | 1 |
| `RESIZE_STEP` (G5) | 1 cell, tmux's `resize-pane` default. The default resize chords omit `amount`; bigger steps are a user binding with `amount`. | 4 |
| Key hints (G3) | chord and its first action's scalar value, else the leaf's name, in file order, cut from the end when the bar is full | 3 |
| `sidebar_width`'s home (G4) | `ClientSettings`, which owns the `Keymap` and the width | 3 |

## Deviation log

- **Planning gates G1 to G7, answered by Cyan in chat on 2026-10-10:** `CreatePane.at` beside `direction` and an unchanged `PaneInput` (G1); `tab create --child` reads `--pane` only with `--child` (G1); a 1x1 mechanical content minimum for splits and resizes, not 2x1 (G2); hints from each binding's value or leaf name (G3); `ClientSettings` owns `[client]` (G4); a 1-cell resize step with no amount in the default chords (G5); directional swap in a zoomed tab unzooms first and stays unzoomed with no neighbor (G6); `ship pane swap --direction` is dropped, so the CLI needs no geometry read (G7). G6 and G7 amend product A-2 and architecture A-3, locked the same day.

- **Swap boundary, confirmed by Cyan in chat:** the directional server request and recency hint are superseded by an ID-only request. Directional resolution belongs to `client.pane.swap` and the CLI adapter; recency remains client-local. Product A-2 and architecture A-3 record the amendment, pending review. The slice 4 zoomed-resolution sequence still needs a decision. (Superseded on 2026-10-10 by the G6 and G7 entry above: the CLI adapter is dropped and the zoomed sequence is decided.)

- **Ratio representation, confirmed by Cyan in chat:** the ten-thousandths `u16` proposal is superseded by a floating-point fraction, matching the proportional approach in Herdr and Zellij. The skeleton uses `f32`, as Herdr does. `0.5` means half the total split area goes to the first child; the second gets `1.0 - ratio`. The named type rejects nonfinite values and values outside the open interval `(0.0, 1.0)`. Ratatui constraint mapping and cell rounding must be verified by the slice 1 probe; floating point does not remove whole-cell rounding.
- **Terminal shrink, confirmed by Cyan in chat:** existing panes squeeze into the available tab area. Shrinking does not delete panes, change stored ratios, or keep a larger layout offscreen. Enlarging restores the proportions, subject to cell rounding. At extreme sizes some panes can have no visible content. Minimum sizes for new splits and manual resizes remain a separate unresolved choice (settled on 2026-10-10 as 1x1; see the G1 to G7 entry). Safe zero-content rendering and PTY sizing remain an implementation question under architecture R-2; no fallback is approved here.
- These decisions amend the architecture's ratio description and R-2. The architecture amendment and this paper were locked on 2026-10-10.
