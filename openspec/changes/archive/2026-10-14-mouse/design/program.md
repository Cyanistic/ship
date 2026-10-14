# Mouse drag and menus: program

Status: Locked on 2026-10-13, approved by Cyan in Plannotator review ("LGTM"), which accepts G1 and G2 as recommended. Revision 2 followed the first review: the lane's new items hold named types (`EdgeAt`, `TabMove`) and `Edge` an `Axis` ("remember the code conventions please"), and `draw` takes `Local` while `panes::draw` takes a `Scene` shared with the router ("should they be taking local instead? or some other self contained struct?"). Written on 2026-10-13 from the locked [product](product.md) and [architecture](architecture.md) papers, against the crates at `dac695b`. Nothing here authorizes implementation.

Settled gates, each accepted as recommended:

- **G1, when a drop unfolds its new parent.** Architecture decision 5 says the drop item runs `tab move` and "on success" unfolds the new parent. A lane turn hands back a request future that can't reach `Local` once it resolves, so "on success" needs either a follow-up item or a pending-unfold field the replica clears. Recommendation: unfold at the drop's turn, right after the re-check passes and before the move is sent. The re-check catches nearly every failure, and a move that still fails leaves the parent unfolded, which is harmless. Approving this amends decision 5's wording through the deviation log. Alternative: `Sidebar` remembers `(tab, parent)` and unfolds when the replica shows the tab under that parent, about 10 more lines. Settled as recommended: "LGTM".
- **G2, a terminal resize while a menu is open.** The product paper doesn't say. Recommendation: close the menu, as a press outside it would. The menu's position was fitted to the old screen, and refitting it under a still pointer is new behavior nobody asked for. Alternative: keep it open and refit it. Settled as recommended: "LGTM".

## Rationale

This is a document-only plan. Nothing below exists as source yet, and none of it has been compiled.

The skeleton follows the architecture's structure box by box. `Mouse` grows a gesture enum next to today's `held` pane and reads what's drawn through one `Scene`, which `panes::draw` also paints from. `Menu` is a new module that owns one open overlay. `Sidebar` keeps the configured width beside a chosen one, records each drawn row for drop gaps, and draws the drop line and the heading button. The lane gains two items, `Resize` and `Move`, and coalesces `Resize` in place. `Local` gains the open menu, a `now` method shared by keys and menu picks, and turns for the two new items. On the server, `Rename` becomes `Set`, resize takes a `Change` that is either relative or absolute, and the pane actor copies the vendored session's tracking flag onto `Screen`.

Applying `docs/agents/code.md`:

- **One source of truth.** A right-click's pane comes from one rule: of the frames holding the cell, the lowest, then the rightmost. That single rule gives FR-030's label, border, junction and outer-edge cases without a separate label hit-test, because each label sits on its own pane's top row. Drop validity is one function, `movable`, called at drag time for the line and at the drop's turn for the re-check.
- **Name your shapes.** `Edge`, `EdgeAt`, `TabMove`, `Gap`, `Grab`, `Target`, `Scene`, `Change` and `Switch` name what would otherwise be tuples, inline variant fields or loose parameters. `Edge` holds an `Axis`, not a `Direction`, so it can't name a left or top side. `Scene` is one shape for what's drawn: the router hit-tests it and `panes::draw` paints it, and `draw::draw` takes `Local` instead of four of its fields.
- **Earn every layer.** `Menu` doesn't know about the lane: it returns `Picked::Run(action)`, and `drive` feeds that through the same press-time-or-queue split a key binding takes (`keys::handled`), so FR-036's "exactly as a key" holds by construction.
- **Check what's already guaranteed.** The architecture's tracking-lag risk mostly goes away on reading the vendored session. Every chunk of PTY output, including the escape sequence that turns mouse mode on, triggers `render_to_shared`, and the pane actor publishes every render without comparing it to the last one. Slice 3 confirms this by hand with Neovim and `less`.
- **Model state as state.** A border drag queues absolute positions, never deltas, which is what makes replacing a waiting position safe.

Estimated size, against SC-006 (client about 650 to 900, server and core about 100 to 160):

| Area | Lines |
| --- | --- |
| Core: `Resize.to`, `Set` commands and bodies replacing `Rename` and `Named`, `Switch`, `Pane.right_click_passthrough`, create flag, `Screen.mouse` | about +70 |
| Server: `Change` in `layout::resize`, the resize handler's choice, `Set` handlers replacing `Rename`, the two routes, `start_pane`, `capture` | about +45 |
| Vendor, counted apart: `Rendered.mouse` and its snapshot field | about +5 |
| Client `mouse.rs`: gestures, edge hit-test, owner rule, right-click and passthrough routing | about +160 |
| Client `menu.rs` | about +140 |
| Client `sidebar.rs`: widths, hit areas, drawn rows, gaps, `movable`, drop line, button | about +140 |
| Client `ui/mod.rs`: `Local.menu`, `now`, `Scene`, menu and drag routing, `Resize` and `Move` turns | about +100 |
| Client `lane.rs`, `keys.rs`, `draw.rs`, `panes.rs` | about +55 |
| Client `keymap.rs`, `execute.rs`, `api.rs` | about +55 |

That is about +650 client, +115 server and core, and +5 vendored. If `menu.rs` passes about 250 lines, stop and revisit before continuing (architecture Risks).

## Skeleton map

### Proposed settings

`keymap/defaults.toml` gains the sidebar keys in resize mode (FR-019) and the three default menus (FR-029), labeled as the product's table has them:

```toml
[client.modes.resize.keys]
# …existing h, j, k, l, esc
"shift-h" = { action = { client.sidebar.resize = { direction = "left" } }, label = "sidebar left" }
"shift-l" = { action = { client.sidebar.resize = { direction = "right" } }, label = "sidebar right" }

[client.menus]
pane = [
  { label = "split right", action = { server.pane.create = { direction = "right", select = true } } },
  { label = "split down", action = { server.pane.create = { direction = "down", select = true } } },
  { label = "zoom", action = { server.pane.zoom = {} } },
  { label = "right-click passthrough", action = { server.pane.set = { right_click_passthrough = "toggle" } } },
  { label = "close", action = { server.pane.close = {} } },
]
tab = [
  { label = "new tab", action = { server.tab.create = { starter = "shell", select = true } } },
  { label = "new child tab", action = { server.tab.create = { starter = "shell", child = true, select = true } } },
  { label = "close", action = { server.tab.close = {} } },
]
global = [
  { label = "new tab", action = { server.tab.create = { starter = "shell", select = true } } },
  { label = "hide sidebar", action = { client.sidebar.toggle = {} } },
  { label = "reload config", action = { client.config.reload = {} } },
  { label = "detach", action = { client.detach = {} } },
]
```

The resize-mode hints are set explicitly because both pane `h` and sidebar `shift-h` would otherwise read "left". The status line shows `shift-h sidebar left  shift-l sidebar right` after the existing four.

The heading button (FR-031) is `≡`, drawn in the heading's last column before the divider, dark gray like the heading.

`PROTOCOL_VERSION` goes from 10 to 11 in slice 1. Slices 2 and 3 extend version 11 on the same branch. No new dependency.

### Added

#### `crates/ship-client/src/ui/menu.rs` (slice 7)

```rust
//! The open right-click menu: a client-local overlay that takes every key
//! and press while open (architecture decision 9).

use crossterm::event::Event;
use ratatui::{
    Frame,
    layout::{Position, Rect},
};
use ship_core::{
    id::IdOf,
    model::{NodeId, Pane, Tab},
};

use crate::keymap::{Action, MenuItem, Menus};

/// Whose menu a right-click opens (FR-030).
#[derive(Clone, Copy)]
pub(super) enum Target {
    Pane(IdOf<Pane>),
    Tab(IdOf<Tab>),
    Global,
}

impl Target {
    /// What opening this menu selects first: the pane or tab, never for
    /// global (FR-032).
    pub fn node(self) -> Option<NodeId>;
}

/// `target`'s items; empty means no menu (FR-028, FR-033).
pub(super) fn items(menus: &Menus, target: Target) -> &[MenuItem];

pub(super) struct Menu {
    /// Copied at open, so a reload while open doesn't change it.
    items: Vec<MenuItem>,
    highlight: usize,
    /// Where it's drawn: at the pointer, moved left and up to fit.
    area: Rect,
}

/// What one event did to the open menu.
pub(super) enum Picked {
    /// Run this and close (left click on an item, or Enter).
    Run(Action),
    /// Close without running (Esc, or a press of any button outside).
    Close,
    /// Stay open: highlight moved, or an event the menu swallows.
    Open,
}

impl Menu {
    /// `None` for an empty list. The first item is highlighted (FR-034).
    pub fn open(items: &[MenuItem], at: Position, screen: Rect) -> Option<Self>;
    /// Up, down, `j` and `k` move the highlight; Enter runs; Esc closes. A
    /// press on an item runs it, outside closes. Releases, motion, paste
    /// and every other key are swallowed (FR-035).
    pub fn handle(&mut self, event: &Event) -> Picked;
    /// A bordered list over everything, the highlight reversed. Drawn last.
    pub fn draw(&self, frame: &mut Frame);
}
```

### Replaced

#### `crates/ship-core/src/command.rs` (slices 1 and 2)

`tab::Rename` and `pane::Rename` become `Set`. The help on `Command::Tab` and `Command::Pane` says "set" where it says "rename" today. `pane::Resize` gains `to`; `pane::Create` gains the passthrough switch.

```rust
pub mod tab {
    pub enum TabCommand {
        // …List, Create, Get
        /// Change a tab's attributes and print it as JSON
        Set(Set),
        // …Close, Move; `Rename` is gone (FR-042)
    }

    /// `server.tab.set`. A field left out stays as it is (FR-041).
    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(default, deny_unknown_fields)]
    pub struct Set {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub tab: TabTarget,
        /// New name; blank clears it
        #[cfg_attr(feature = "clap", arg(long))]
        pub name: Option<String>,
    }
}

pub mod pane {
    use crate::model::Switch;

    pub enum PaneCommand {
        // …Create, Get
        /// Change a pane's attributes and print it as JSON
        Set(Set),
        // …Close, Resize, Scroll, Read, Zoom, Swap; `Rename` is gone
    }

    pub struct Create {
        // …existing fields
        /// Send right-clicks on its contents to its program (FR-039)
        #[cfg_attr(feature = "clap", arg(long))]
        pub right_click_passthrough: bool,
    }

    /// `server.pane.set` (FR-040).
    #[derive(Clone, Debug, Default, Deserialize)]
    #[cfg_attr(feature = "clap", derive(Args))]
    #[serde(default, deny_unknown_fields)]
    pub struct Set {
        #[cfg_attr(feature = "clap", command(flatten))]
        pub pane: PaneTarget,
        /// New name; blank clears it
        #[cfg_attr(feature = "clap", arg(long))]
        pub name: Option<String>,
        /// Send right-clicks on the contents to a program that asked for
        /// mouse input
        #[cfg_attr(feature = "clap", arg(long, value_name = "true|false|toggle"))]
        pub right_click_passthrough: Option<Switch>,
    }

    pub struct Resize {
        // …pane, direction
        /// Cells; omitted means one
        #[cfg_attr(feature = "clap", arg(long, conflicts_with = "to"))]
        #[serde(default)]
        pub amount: Option<u16>,
        /// Draw the edge's border at this cell, counted from the tab's left
        /// edge for left and right or its top edge for up and down (FR-008)
        #[cfg_attr(feature = "clap", arg(long))]
        #[serde(default)]
        pub to: Option<u16>,
    }
}
```

#### `crates/ship-core/src/model.rs` (slice 2)

`Named<N>` goes; nothing else uses it. `Pane` gains the setting, and the on/off/toggle value lives beside `OptionalName`.

```rust
pub struct Pane {
    // …id, name, command, cwd, dir, title
    /// Right-clicks on the contents go to the program when it has asked for
    /// mouse input. Always written, so `ship pane get` shows it (FR-037).
    #[serde(default)]
    pub right_click_passthrough: bool,
    pub status: PaneStatus,
}

/// A two-valued attribute's new value: `true`, `false`, or `"toggle"` for
/// the other one, resolved where the change takes effect. The same spelling
/// in the config, the CLI and request bodies (decision 13, FR-043).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(untagged)]
pub enum Switch {
    Set(bool),
    Toggle(Toggle),
}

/// The word `"toggle"`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum Toggle {
    Toggle,
}

impl Switch {
    /// The value after this change to `current`.
    pub fn apply(self, current: bool) -> bool;
}

/// `true`, `false` or `toggle`, for clap.
impl FromStr for Switch {
    type Err = AppError;
}
```

#### `crates/ship-core/src/protocol.rs` (slices 1 and 2)

```rust
pub const PROTOCOL_VERSION: u32 = 11;

/// POST /panes/{id}/resize body. At most one of `amount` and `to`.
pub struct ResizePane {
    pub direction: Direction,
    /// Cells; absent is one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<u16>,
    /// Where the edge's border is drawn, from the tab's left or top edge.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to: Option<u16>,
}

/// PATCH /tabs/{id} body. An absent or `null` field stays as it is; a
/// blank name clears it.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetTab {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub name: Option<OptionalName>,
}

/// PATCH /panes/{id} body, on the same terms as `SetTab`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SetPane {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schema(value_type = Option<String>)]
    pub name: Option<OptionalName>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub right_click_passthrough: Option<Switch>,
}

pub struct PaneInput {
    #[serde(default)]
    pub name: OptionalName,
    /// Start with right-click passthrough on.
    #[serde(default, skip_serializing_if = "is_false")]
    pub right_click_passthrough: bool,
    #[serde(flatten)]
    pub spec: PaneSpec,
}
```

#### `crates/ship-core/src/screen.rs` (slice 3)

```rust
pub struct Screen {
    // …size, cells, cursor, scrolled
    /// Whether the program has asked for mouse input. Absent when it hasn't.
    #[serde(default, skip_serializing_if = "is_false")]
    pub mouse: bool,
}

/// For `skip_serializing_if` here and in `protocol`.
pub(crate) fn is_false(value: &bool) -> bool;
```

#### `crates/ship-server/src/state/layout.rs` (slice 1)

`resize` takes a `Change` instead of `cells`. `To` sets the first side's target length to `to + 1 - area.x` (the first side's frame ends on the shared border cell), then picks `forward` and `moved` from that target and the current length, and the clamp search runs as today. Slice 1 checks the off-by-one against `ship tab get`'s geometry.

```rust
/// How far an edge moves.
pub(crate) enum Change {
    /// Cells in `direction`.
    By(u16),
    /// To this cell from the tab's left or top edge, whichever way that is.
    To(u16),
}

pub(crate) fn resize(
    tab: &mut Tab,
    size: Size,
    pane: IdOf<Pane>,
    direction: Direction,
    change: Change,
) -> Result<bool>;
```

#### `crates/ship-server/src/state.rs` (slices 1 and 2)

```rust
/// PATCH a tab or pane. Replaces `Rename<T>`.
pub struct Set<T: Identified, B> {
    pub id: IdOf<T>,
    pub set: B,
}

impl Message<Set<Tab, SetTab>> for ServerState {
    type Reply = Result<Tab>;
    /// Commits only when a field changes the tab; otherwise answers with it
    /// unchanged, so an empty `set` bumps no revision.
}

impl Message<Set<Pane, SetPane>> for ServerState {
    type Reply = Result<Pane>;
    /// Resolves `toggle` against the stored value inside the actor, then
    /// commits only on a change (FR-040).
}

impl Message<Resize> for ServerState {
    /// `amount` and `to` together are a validation error before anything
    /// is read. `to` becomes `Change::To`; otherwise
    /// `Change::By(amount.unwrap_or(RESIZE_STEP))`.
}

impl ServerState {
    /// Copies `input.right_click_passthrough` onto the new `Pane`.
    fn start_pane(&mut self, size: Size, input: PaneInput) -> Result<Pane>;
}
```

#### `crates/ship-server/src/routes.rs` and `lib.rs` (slice 2)

`rename_tab` and `rename_pane` become `set_tab` and `set_pane`: the same `PATCH /api/v0/tabs/{id}` and `/api/v0/panes/{id}` paths, with operation IDs `set_tab` and `set_pane` and bodies `SetTab` and `SetPane`. `lib.rs`'s `api_router` lists the new names.

```rust
pub(crate) async fn set_tab(
    State(app): App,
    Path(id): Path<IdOf<Tab>>,
    Json(set): Json<SetTab>,
) -> Result<Response>;

pub(crate) async fn set_pane(
    State(app): App,
    Path(id): Path<IdOf<Pane>>,
    Json(set): Json<SetPane>,
) -> Result<Response>;
```

#### `crates/ship-server/src/pane.rs` and `vendor/ratatui-ghostty/src/session.rs` (slice 3)

The vendored snapshot records tracking at each render, under the same lock as the cells, and `capture` copies it.

```rust
// vendor/ratatui-ghostty/src/session.rs
pub struct Rendered {
    pub cursor: CursorState,
    pub scrolled: Option<Scrolled>,
    /// `is_mouse_tracking()` when these cells were rendered.
    pub mouse: bool,
}

struct RenderSnapshot {
    // …buffer, cursor, scrolled
    mouse: bool,
}

// crates/ship-server/src/pane.rs
/// Also copies `rendered.mouse` onto `Screen.mouse`.
fn capture(session: &SessionHandle, buffer: &mut Buffer) -> Option<Screen>;
```

#### `crates/ship-client/src/ui/lane.rs` (slices 4 and 6)

```rust
/// A border, named by the pane left of or above it (decision 2, FR-009).
/// `Horizontal` is the border on the pane's right, `Vertical` the one below
/// it, so no edge can name a left or top side. Its turn sends `Right` or
/// `Down`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Edge {
    pub pane: IdOf<Pane>,
    pub axis: Axis,
}

/// Draw `edge`'s border at `to`, in cells from the tab's left or top edge.
#[derive(Clone, Copy)]
pub(super) struct EdgeAt {
    pub edge: Edge,
    pub to: u16,
}

/// Move a dropped row's tab to `to`.
#[derive(Clone, Copy)]
pub(super) struct TabMove {
    pub tab: IdOf<Tab>,
    pub to: MoveTab,
}

pub(super) enum Item {
    // …Action, Key, Paste, Select, Fold, Mouse
    /// Nothing if the edge's pane is gone by its turn.
    Resize(EdgeAt),
    /// Re-checked against the tree at its turn (FR-024).
    Move(TabMove),
}

impl<'a> Lane<'a> {
    /// Appends, except that a `Resize` for an edge already waiting
    /// overwrites that item's `to` where it stands (decision 3, FR-010).
    pub fn push(&mut self, item: Item);
}
```

#### `crates/ship-client/src/ui/mouse.rs` (slices 4 to 8)

`route` takes a `Scene` and the lane's `busy` instead of six parameters and keeps today's paths for pane contents, the wheel and held program drags. New paths are checked first, in this order: a live gesture's motion and release; sidebar presses (divider, row, button, right-clicks); a left press on a border; a right press on a pane. Then today's pane path. A border press never reaches the selecting branch (FR-007).

```rust
/// What a mouse event does.
pub(super) enum Clicked {
    Select(NodeId),
    Fold(IdOf<Tab>),
    Frame(InputFrame),
    /// New positions for the grabbed edges that moved, one or two.
    Resize(Vec<EdgeAt>),
    /// A row released on a valid gap (FR-023).
    Move(TabMove),
    /// The sidebar's width from a divider drag (FR-014).
    Width(u16),
    /// Open `Target`'s menu at the pointer (FR-030, FR-031).
    Menu(Target, Position),
}

#[derive(Default)]
pub(super) struct Mouse {
    /// The pane a program press went to, so its drag and release follow it.
    held: Option<IdOf<Pane>>,
    gesture: Option<Gesture>,
}

/// At most one left-button gesture (FR-001, FR-002).
enum Gesture {
    /// A row pressed and not yet moved a cell: on release, a select or fold
    /// (FR-005).
    Pressed { at: Position, row: Row },
    /// A row being dragged, and the gap under the pointer when valid.
    Row { tab: IdOf<Tab>, gap: Option<Gap> },
    /// One or two grabbed edges. A press on a border never clicks, so it
    /// starts here, and only motion queues anything.
    Border(Vec<Grab>),
    Divider,
}

/// A grabbed edge and the last position queued for it, which starts at
/// the pressed cell so an unmoved press queues nothing (FR-010).
struct Grab {
    edge: Edge,
    last: u16,
}

impl Mouse {
    /// `busy` is the lane's, read at press as today.
    pub fn route(&mut self, event: MouseEvent, scene: &Scene, busy: bool) -> Option<Clicked>;
    /// Whether a gesture is live, so `drive` redraws on its motion.
    pub fn dragging(&self) -> bool;
    /// The grabbed edges, drawn in the selected-border style (FR-012).
    pub fn held(&self) -> impl Iterator<Item = Edge> + '_;
    /// The drop gap under a dragged row, when valid (FR-021).
    pub fn gap(&self) -> Option<&Gap>;
}

/// The edges through tab cell `at`: a cell in two or more frames and no
/// content. A vertical border is named by the pane whose frame ends there,
/// a horizontal one by the pane above. Up to one each, so a junction gives
/// both (FR-006). Empty with fewer than two panes (FR-013).
fn edges_at(geometry: &TabGeometry, at: Position) -> Vec<Edge>;

/// The pane a right-click at tab cell `at` is for: of the frames holding
/// it, the lowest, then the rightmost. Gives the label's pane, the pane
/// below or right of a border, below-right at a junction, and the pane
/// inside on outer edges (FR-030).
fn owner(geometry: &TabGeometry, at: Position) -> Option<IdOf<Pane>>;

/// Whether a right press on `pane`'s content goes to its program:
/// passthrough is on and its latest screen says it tracks the mouse (FR-038).
fn passes(scene: &Scene, pane: IdOf<Pane>) -> bool;
```

A drag on a border emits a position for each grab whose axis coordinate changed since `last`; motion that ends a grab's pane in `geometry` ends the gesture quietly (decision 4). Release emits a final `Resize` only when it differs from `last`. A divider drag emits `Width(column + 1)`. A row drag recomputes its gap through `Sidebar::gap` on each motion, and release emits `Move` only when the gap is valid and the pointer is inside the sidebar. A right press on a target with an empty list takes today's path (FR-033).

#### `crates/ship-client/src/ui/sidebar.rs` (slices 5 to 7)

`width` stops being a public field, and `row_at` becomes `hit`.

```rust
const MIN_WIDTH: u16 = 10;
/// The pane area's minimum, which wins on a narrow terminal (FR-015).
const MIN_PANES: u16 = 20;
const BUTTON: &str = "≡";

pub(super) struct Sidebar {
    collapsed: HashSet<IdOf<Tab>>,
    pub shown: bool,
    /// `sidebar_width` as last loaded.
    configured: u16,
    /// This client's dragged or keyed width; dropped when `configured`
    /// changes (FR-016).
    chosen: Option<u16>,
    rendered: TreeState<IdOf<Tab>>,
    tree: Rect,
    /// Each row as last drawn, top to bottom, for drop gaps.
    drawn: Vec<Drawn>,
}

struct Drawn {
    tab: IdOf<Tab>,
    depth: u16,
    y: u16,
}

/// What a sidebar position is.
pub(super) enum Hit {
    Row(Row),
    /// The `tabs` heading, outside the button.
    Heading,
    Button,
    /// Space below the last row.
    Below,
    /// The right divider column.
    Divider,
}

/// A valid drop: the move it makes and where its line starts.
#[derive(Clone, Copy)]
pub(super) struct Gap {
    pub to: MoveTab,
    /// The underlined row (the blank row under the heading for the first
    /// gap) and the target depth's indent (decision 6).
    line: Position,
}

impl Sidebar {
    pub fn new(width: u16) -> Self;
    /// `chosen`, else `configured`; 0 while hidden.
    pub fn width(&self) -> u16;
    /// A reload's `sidebar_width`.
    pub fn configure(&mut self, width: u16);
    /// This client's width, clamped for a terminal `cols` wide (FR-015).
    pub fn set_width(&mut self, width: u16, cols: u16);
    /// `client.sidebar.resize`; nothing while hidden (FR-018).
    pub fn nudge(&mut self, side: Side, amount: u16, cols: u16);
    pub fn hit(&self, at: Position) -> Option<Hit>;
    /// The gap a dragged `tab` lands in at `at`: above the row under the
    /// pointer, or after the last row below it. Depth from the column in
    /// `INDENT` steps, clamped to the row below's depth (0 at the end)
    /// through one more than the row above's (decision 7). The deepest is
    /// before the row below when that row is the above row's child,
    /// otherwise `Parent(above)`; a shallower depth is after the above
    /// row's ancestor at that depth. `None` when `movable` says no.
    pub fn gap(&self, tabs: &Tabs, tab: IdOf<Tab>, at: Position) -> Option<Gap>;
    /// Also draws the button and, when given, the drop line.
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        tabs: &Tabs,
        selection: Option<NodeId>,
        gap: Option<&Gap>,
    );
    // …reconcile, reveal, visible, fold, toggle unchanged
}

/// Whether moving `tab` to `to` changes anything in `tabs`: every named tab
/// exists, none is `tab` or inside its subtree, and the tab would change
/// parent or position (FR-022, FR-024).
pub(super) fn movable(tabs: &Tabs, tab: IdOf<Tab>, to: MoveTab) -> bool;
```

#### `crates/ship-client/src/ui/mod.rs` (slices 4 to 8)

```rust
mod menu;

struct Local {
    // …observer, memory, sidebar, keys, mouse, input, message
    /// The open menu, which gets every key and press first (decision 9).
    menu: Option<Menu>,
}

impl Local {
    /// Mode, sidebar (toggle and resize) and reload actions, at press, from
    /// a key or a menu pick. Was inline in `drive`'s `Handled::Now` arm.
    fn now(
        &mut self,
        action: ClientAction,
        guard: &mut TerminalGuard,
        config: &Path,
        cols: u16,
    ) -> Result<()>;

    fn turn<'a>(&mut self, client: &'a Client, item: Item) -> Turn<'a>;
    // New arms:
    // Resize: `Done` if the pane is gone, else `Wait` on `client.resize`
    //   with `Right` for a `Horizontal` edge or `Down` for a `Vertical` one,
    //   `to`, and no `amount`; not-found counts as done.
    // Move: `Done` unless `movable`; unfold `Parent(Some(p))` (G1); `Wait` on
    //   `client.move_tab`, with not-found and invalid-structure treated as done.

    /// What is drawn, for the router and the pane painter.
    /// `screen` is the terminal's area; the pane area is what the sidebar
    /// leaves of it.
    fn scene(&self, screen: Rect) -> Scene<'_>;
}

/// The client's view as drawn: what the router hit-tests against and what
/// `panes::draw` paints. One shape, so both read the same frames.
struct Scene<'a> {
    /// The pane area.
    panes: Rect,
    tabs: &'a Tabs,
    /// The viewed tab, for each pane's label and passthrough setting.
    tab: Option<&'a Tab>,
    geometry: Option<&'a TabGeometry>,
    selected: Option<IdOf<Pane>>,
    /// Each pane's latest screen, including `Screen.mouse` (decision 11).
    screens: &'a HashMap<IdOf<Pane>, Arc<Screen>>,
    sidebar: &'a Sidebar,
    menus: &'a Menus,
}

/// Was `redraw(guard, observer, sidebar, status)`.
fn redraw(guard: &mut TerminalGuard, local: &mut Local) -> Result<()>;
```

`Scene` and `Local` stay private to `ui/mod.rs`; its child modules (`mouse`, `draw`, `panes`) see them through `super::`, as they already see `Request`.

In `drive`'s terminal-event arm, an open menu sees key, paste and mouse events first. `Picked::Run(action)` closes the menu and goes through `keys::handled(action)`: `Handled::Now` calls `Local::now`, `Handled::Queue` pushes to the lane as a key's would. A mouse event with no menu builds `local.scene(...)` and routes it. `Clicked::Menu(target, at)` pushes `Item::Select` for `target.node()`, then opens the menu. `Clicked::Resize` pushes one `Item::Resize` per edge, `Clicked::Move` pushes `Item::Move`, and `Clicked::Width` calls `set_width`. `visible` is set while `mouse.dragging()` and whenever a menu opens, moves or closes. `Event::Resize` closes an open menu (G2). `reload` calls `sidebar.configure` instead of assigning `width`, and closes any open menu along with resetting `Mouse`.

#### `crates/ship-client/src/ui/keys.rs` (slice 7)

```rust
/// A bound action at press: mode, sidebar and reload now, the rest queued.
/// Shared by `Keys::handle` and menu picks.
pub(super) fn handled(action: Action) -> Handled;
```

#### `crates/ship-client/src/ui/draw.rs` and `ui/panes.rs` (slices 4, 6 and 7)

```rust
// draw.rs
/// The sidebar with `local.mouse.gap()`, then the panes from
/// `local.scene(..)`, then the status line built from `local.keys` and
/// `local.message` (today's `status()`), then `local.menu` over everything.
/// Was `draw(frame, observer, sidebar, status)`.
pub(super) fn draw(frame: &mut Frame, local: &mut Local) -> Option<Cursor>;

// panes.rs
/// Paints `scene`'s tab into `scene.panes`, as today. Also restyles each
/// held edge's straight run of border cells in the selected-border style,
/// after the borders and before the labels (FR-012). Was six parameters.
pub(super) fn draw(
    frame: &mut Frame,
    scene: &Scene,
    held: impl Iterator<Item = Edge>,
) -> Option<(Position, Cursor)>;
```

#### `crates/ship-client/src/keymap.rs` (slices 5 and 7)

```rust
pub struct Keymap {
    pub modes: HashMap<ModeName, Mode>,
    /// Swapped with the modes on reload.
    pub menus: Menus,
}

/// `[client.menus]` over the defaults: a target the file names replaces
/// that list whole (FR-028).
#[derive(Clone, Default)]
pub struct Menus {
    pub pane: Vec<MenuItem>,
    pub tab: Vec<MenuItem>,
    pub global: Vec<MenuItem>,
}

/// One item: exactly one action and an optional label (FR-027).
#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MenuItem {
    pub action: Action,
    #[serde(default)]
    label: Option<String>,
}

impl MenuItem {
    /// The label, else `Action::label`.
    pub fn label(&self) -> Cow<'_, str>;
}

/// The file's and the defaults' `[client.menus]`. Unknown targets fail.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct MenusFile {
    pane: Option<Vec<MenuItem>>,
    tab: Option<Vec<MenuItem>>,
    global: Option<Vec<MenuItem>>,
}

pub enum Sidebar {
    Toggle {},
    /// Move the right edge; amount defaults to 1 (FR-018).
    Resize {
        direction: Side,
        #[serde(default)]
        amount: Option<u16>,
    },
}

/// `client.sidebar.resize`'s own direction, so `up` fails at load.
#[derive(Clone, Copy, Debug, Deserialize, strum::IntoStaticStr)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
pub enum Side {
    Left,
    Right,
}

// `Action::label` gains an arm: `Sidebar(Resize { direction })` gives the
// direction's name, like pane resize.
// `Client` (the file struct) gains `#[serde(default)] menus: MenusFile`.
```

#### `crates/ship-client/src/execute.rs` and `api.rs` (slices 1 and 2)

`execute` passes `to` into `ResizePane`, maps `TabCommand::Set` and `PaneCommand::Set` to the new bodies (an empty or blank `--name` becomes a clearing `OptionalName`), and sets `right_click_passthrough` on `PaneInput`. In `api.rs`, `rename` becomes:

```rust
/// PATCH the tab or pane with `body`; returns it.
pub async fn set<T: Resource>(&self, id: IdOf<T>, body: &impl Serialize) -> Result<T>;
```

### Moved

None.

### Untouched

- `crates/ship/` (the binary). The CLI derives `tab set`, `pane set`, `--to` and `--right-click-passthrough` from `ship-core`, and `rename` disappears with its variants, so no hand-written code changes.
- `ship-server`'s `input.rs` already routes mouse frames to any named pane, so passthrough to an unselected pane needs nothing there.
- `observer.rs`, `memory.rs`, `terminal.rs`, attach and views. Mouse capture already enables button and motion reporting.
- `ship-core`'s `geometry.rs`, `tree.rs` and `layout.rs`.

## Build order

1. **`to` on pane resize.** Files: `command.rs` (`Resize.to`, clap conflict), `protocol.rs` (`ResizePane.to`, version 11), `layout.rs` (`Change`), `state.rs` (the resize handler), `execute.rs`. Delivers FR-008 and P1's script scenario. Checks: in a two-pane tab with a client attached, `ship pane resize --direction right --to 40` on the left pane puts its frame's right column at 40 in `ship tab get`'s geometry, so the off-by-one is settled here. `--to 40` again reports the tab unchanged and bumps no revision. `--to 2` stops with the neighbor at 1 column. `--to 5 --amount 2` exits 2. The same pair in an HTTP body is rejected and changes nothing. `--direction left` on the right pane moves the same edge, and plain `--amount` behaves as before. `ship server status` reports protocol 11.
2. **`set` replaces `rename`; the passthrough attribute.** Files: `command.rs`, `model.rs` (`Switch`, `Pane` field, `Named` removed), `protocol.rs` (`SetTab`, `SetPane`, `PaneInput`), `routes.rs`, `lib.rs`, `state.rs` (`Set` handlers, `start_pane`), `execute.rs`, `api.rs`. Delivers FR-037 and FR-039 to FR-043, and P6. Checks: P6's four scenarios as written. `ship pane set --right-click-passthrough toggle` twice returns to the first value. `ship pane create --right-click-passthrough -- sh` shows `rightClickPassthrough: true`, and a tab's starter pane shows `false`. `--right-click-passthrough maybe` exits 2. A config binding `server.tab.rename` fails `ship config check` with line and column, and `server.pane.set = { right_click_passthrough = "toggle" }` passes. An empty `set` leaves the revision unchanged. A second client's replica shows the new value.
3. **Mouse tracking on `Screen`.** Files: `vendor/ratatui-ghostty/src/session.rs`, `pane.rs` (`capture`), `screen.rs`. Delivers what SC-005 allows for FR-038. Checks: on a raw attach stream, a pane's `Screen` carries `mouse: true` while Neovim with `mouse=a` runs, and no `mouse` at the shell prompt, in `less` without `--mouse`, or after Neovim quits. `less --mouse` turns it on. The flag flips with the program's mode change without any other screen change (confirms the reading in the rationale).
4. **Gestures and border drags.** Files: `lane.rs` (`Edge`, `EdgeAt`, `Item::Resize`, coalescing), `mouse.rs` (`Gesture` with `Pressed` and `Border`, `edges_at`, sidebar clicks on release), `mod.rs` (`Scene`, `Local::scene`, `redraw` taking `Local`, the `Resize` turn, redraw while dragging), `panes.rs` (drawing from `Scene`, held edges), `draw.rs` (`draw` taking `Local`). Delivers FR-001, FR-002, FR-004 to FR-007, FR-009 to FR-013 and P1. Checks: P1's drag scenarios by hand, with `ship tab get` geometry matching what's drawn. A 2x2 junction moves both edges. A press on the border of an unselected pane leaves selection alone, even with a `sleep`-delayed create waiting. A border click without motion does nothing. A fast drag over Neovim and a shell feels like a tmux drag and leaves no backlog (SC-002). Through a proxy adding 100 ms each way, the border trails and then settles at the pointer. Closing the dragged pane from a script mid-drag ends the drag with no error. Sidebar rows select and fold on release, not press, and a row press that moves does neither. Every other mouse behavior is as before (SC-003).
5. **Sidebar width.** Files: `sidebar.rs` (`configured`, `chosen`, `set_width`, `nudge`, `hit` with `Divider`), `mouse.rs` (`Divider`), `keymap.rs` (`Sidebar::Resize`, `Side`, label arm), `defaults.toml` (`shift-h`, `shift-l`), `mod.rs` (`Local::now`, `Width`, `configure` on reload). Delivers FR-014 to FR-019 and P3. Checks: P3's scenarios in two attached clients. On a 40-column terminal, the width stops so the pane area keeps 20. `client.sidebar.resize` with `direction = "up"` fails `ship config check` with line and column. The resize mode's hints end with `shift-h sidebar left  shift-l sidebar right`. With the sidebar hidden, the keys do nothing and showing it keeps the width.
6. **Tab drops.** Files: `sidebar.rs` (`drawn`, `gap`, `movable`, the drop line), `lane.rs` (`TabMove`, `Item::Move`), `mouse.rs` (`Row`, `gap`), `mod.rs` (the `Move` turn), `draw.rs` (passing the gap to the sidebar). Delivers FR-020 to FR-026 and P2. Checks: P2's scenarios, checking `ship tab list` against the line shown before each release. The depth range holds at a gap between a deep row and a shallower one. Dropping into the dragged tab's own subtree shows no line and does nothing. A drop whose target a script closes before its turn does nothing and shows no error. A folded parent unfolds after a child drop. Selection doesn't change, and a moved selected tab stays selected.
7. **Menus.** Files: `menu.rs`, `keymap.rs` (`Menus`, `MenuItem`, `MenusFile`), `defaults.toml` (`[client.menus]`), `keys.rs` (`handled`), `mouse.rs` (right-click routing, `owner`, button), `sidebar.rs` (`Heading`, `Button`, `Below`, button drawing), `mod.rs` (`Local.menu`, menu-first routing, select then open), `draw.rs` (menu last). Delivers FR-027 to FR-036 (FR-038's passthrough branch comes in slice 8) and P4. Checks: P4's scenarios by hand, including the replaced and emptied config. A menu near the bottom-right corner fits on screen. A right-click on a border opens the menu for the pane FR-030 names, at a junction too. An item that fails shows its error on the status line. "hide sidebar" and "reload config" act at once, and a menu action after a pending create acts on the clicked target (SC-004). `[client.menus] pain = []` and an item with `lable` fail `ship config check` with line and column. With `mouse = false`, nothing opens (FR-003). `menu.rs` is under about 250 lines.
8. **Passthrough at the router.** Files: `mouse.rs` (`passes`, the passthrough branch). Delivers FR-038 and P5. Checks: P5's scenarios with Neovim `mouse=a` in two clients. A right drag in Neovim's visual mode follows past the pane's edge. A passthrough pane's label and border still open the menu, and the shell prompt in a passthrough pane opens the menu. The right-clicked pane isn't selected when the click passes through.
9. **Docs, specs and size.** Files: this change's `proposal.md`, `tasks.md` and spec deltas (`mouse-input`, `terminal-client`, `keymap`, `config-file`, `session-structure`, `session-observation`, and any other spec naming `rename`); `docs/guide.md` (drags, menus, `set`, `--to`, passthrough); `README.md`'s "What works today" and keys if mouse resizing is listed as missing; the product brief's milestone-1 item. Checks: `openspec validate mouse`, every link resolves, and the guide's examples run as written. Report implementation Rust, vendored Rust and tests separately against `dac695b` (SC-006). SC-001 runs by hand in two terminals on macOS; Linux is reported unverified unless Cyan runs it there.

## Deviation log

- **Consolidated slice 2, patch-name serialization.** `OptionalName` serializes a cleared name as null, but the planned PATCH bodies treat null as absent. A shared field serializer therefore writes a present cleared name as `""`. Both CLI clearing workflows passed. This enforces the planned wire semantics without changing user-visible requirements.

- **Checkpoint consolidation, approved by Cyan after slice 1.** Six review checkpoints replace nine: original slice 1; slices 2–3 together; slices 4–5 together; slice 6; slices 7–8 together; slice 9. Scope, task numbers and verification requirements are unchanged. This is execution grouping, not user-visible behavior.

- **Lock, architecture decision 5 amended by G1.** The drop item unfolds the new parent at its turn, after the re-check passes and before `tab move` is sent, not after the move succeeds. Approved with this paper ("LGTM"). The architecture paper's text is left as locked; this entry is the amendment. The mouse-input delta now states that timing and that a subsequent failed move leaves the parent unfolded.

- **Slice 7 alone, requested by Cyan.** Cyan requested original slice 7 independently of slice 8 despite the earlier checkpoint consolidation. Both were completed and verified separately. This changes execution grouping only, not user-visible behavior.

- **Slice 7 menu follow-up, requested by Cyan.** Pointer movement highlights a visible item without activating it, and border/outside movement preserves the last highlight. The menu uses terminal-default row colors, ANSI cyan borders and a bold ANSI black-on-cyan highlight instead of reverse video or fixed RGB colors. Ratatui ListState provides the viewport offset used for hit-testing a clipped list. The context-menus delta records the hover, viewport and palette behavior.

- **Slice 7 naming amendment, approved by Cyan.** Both bindings and menu items accept `label`; the action fallback is `Action::label()`. A composed chord plus label remains a status-line hint. Binding `hint` fields are rejected without an alias. The keymap delta records the binding syntax and obsolete-field rejection, and the terminal-client delta uses `label` for status-line overrides. The published keymap and terminal-client specs had already been updated during the follow-up.

- **Slice 9 accounting.** Against `dac695b`, `cloc` measures net growth of +820 client, +113 server/core and +5 vendored implementation Rust code lines, with no authored tests. This is within SC-006's +650 to +900 and +100 to +160 estimates. Physical growth including comments and blanks is +871 client, +138 server/core and +6 vendored. The complete counts, verification and SC-001 coverage limitations are recorded in [tasks.md](../tasks.md). The planning estimates above remain historical.
