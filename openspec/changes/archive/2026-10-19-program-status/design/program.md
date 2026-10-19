# Program status: program

Status: Locked on 2026-10-18 with Cyan's approval (Plannotator "LGTM" after review rounds that reshaped G1, made G2's sound tables explicit, split the sidebar into sections, moved sound loading to config load, and checked the plan against `docs/agents/code.md`). Corrected the same day at Cyan's request to name `ship tab list`, which prints every pane, in place of `ship pane list`, which doesn't exist. Written from the locked [product](product.md) and [architecture](architecture.md) papers against `origin/main` at `0a6bd77`. Approval doesn't start implementation; Cyan requests each slice separately.

Gates, all settled in review. G6 took its recommendation when the paper was approved without a note on it.

- **G1, the `Pane` field.** Settled: `program`, a map from record id to record that mirrors the specification's model of one record per id. The root record's key is the empty string. That is how Ghostty's API names the root (`id`: "Empty for the root record"), and no program can claim it, because the specification's id grammar needs at least one character per segment and drops a report whose id fails it. A clear with an empty id removes every key, so the clear rule has no special case. The field is omitted when the pane has no records.

  ```json
  "program": {
    "": { "state": "working", "app": "deploy", "msg": "Deploying v2.4.1" },
    "eu-west": { "state": "blocked", "kind": "permission", "title": "EU West" }
  }
  ```

  Rejected: a list of records carrying an optional `id` (doesn't read as one record per id), a `root` key beside the others (`root` is a valid id), and root fields flattened onto `program` with the others under `records` (reshapes the model and splits the server code into two cases).
- **G2, config keys.** Settled:

  ```toml
  [client.agents]
  rows = [["icon", "app", "state"], ["tab"]]
  delay_ms = 1000
  popups = false

  [client.agents.sounds.default]
  done = { builtin = "done" }
  attention = { builtin = "attention" }

  [client.agents.sounds.apps.droid]
  done = "off"
  ```

  The defaults and each app's overrides are separate tables of the same shape, so one type reads both and a default never hides among the app names. Rejected: `done` and `attention` directly under `sounds` beside `apps`, which needs `#[serde(flatten)]` and gives up `deny_unknown_fields` there. `delay_ms` follows the integer style of `sidebar_width`; Herdr's `delay_seconds` was the alternative.
- **G3, the jump action's name.** Settled: `client.agents.next = {}`, bound to `alt-a` (settled in product review).
- **G4, the keys that move the split.** Settled: `client.sidebar.resize` takes a `Direction` instead of a `Side`, so `left` and `right` keep resizing the width and `up` and `down` move the divider. `shift-j` and `shift-k` in resize mode bind `down` and `up`. Existing configs still parse. Rejected: a separate `client.sidebar.split` action.
- **G5, icons and colors** (FR-010: distinct by both shape and color). Settled: blocked `▲` red, error `✖` magenta, unseen `◆` cyan, working `●` yellow, idle `○` green. The `state` token takes its icon's color.
- **G6, how many records a pane keeps.** The specification requires a terminal to hold at least 64 records, allows up to 256, and evicts the one updated longest ago when full. Settled: 64. The four agents only use the root record, and 64 bounds the replica if a program reports many workers. Rejected: 256. Not a choice: the text limits are the specification's (`app` 32 bytes, `title` 192, `msg` 2048, `id` 128), enforced only where reports are parsed (see "Reports are checked once").

## Rationale

This is a document-only plan. The skeleton below describes proposed source, not created or compiled code.

The skeleton follows the architecture's structure box by box:

- **`vendor/ratatui-ghostty/src/program_status.rs`** is the scanner, the only temporary file. It finds OSC 7501, OSC 133 prompt starts and RIS in each batch, carrying partial sequences across batches. `session.rs` runs it just before `vt_write`, writes the support reply through the session's writer, and pushes `SessionEvent::ProgramStatus` or `SessionEvent::PromptStart`. RIS becomes a `clear` report with an empty id, which is exactly what Ghostty's callback delivers. The report type mirrors upstream's `GhosttyTerminalProgramStatus`, so the swap replaces the scanner with a callback registration and leaves the event untouched.
- **`ship-core` model** gains `StatusRecord` and its enums, and a `program` field on `Pane` (G1). Serde and utoipa derives put it in the replica and in `ship pane get` and `ship tab list` without further code (FR-005).
- **`crates/ship-server/src/records.rs`** holds the revision 0.4 rules: put, clear a subtree, evict, drop working and blocked on a prompt, and strip bidirectional formatting characters. The pane task owns one `Records` value and publishes the whole set when it changes, which follows the "model state as state" rule in `docs/agents/code.md` and matches how the task already treats `title` and `dir`.
- **`state.rs`** replaces the pane's records on `PaneChange::Program` and clears them on `PaneChange::Exited`. **`saver.rs`**'s `blank` clears them too.
- **`crates/ship-client/src/ui/attention.rs`** is the notifier. Its inputs are replicas, focus and the panes in view. It owns the previous root state per pane, pending notifications with their due time, the unseen set and the attach baseline, and it emits `Notice`s when they're due.
- **`crates/ship-client/src/ui/agents.rs`** derives display states, the tab rollup, the ordered agent list and the next pane for the jump action. It has no rendering, so the jump action, the rollup and the notifier use it without ratatui.
- **`crates/ship-client/src/sound.rs`** holds `Clip`, audio known to decode, and the speaker that plays clips on the audio thread. It sits beside `keymap.rs` rather than under `ui/`, because loading the config turns sound settings into clips.
- **`sidebar.rs`** becomes `sidebar/`, one file per section under a frame. `sidebar/mod.rs` is the frame: shown, width, the client-local split, the right-hand divider, the divider row between the sections, and `hit`, which hands a position to the section under it. `sidebar/tabs.rs` is today's tree code, moved, and now keeps its tree state between frames so it can scroll. `sidebar/agents.rs` is the agents section: its offset, its drawn rows for hit-testing, row layout from tokens and the icons. Each section owns its own scrolling and hits. **`mouse.rs`** routes the nested hits. **`terminal.rs`** turns focus reporting on and writes popups. **`keymap.rs`** parses the new settings and actions. **`mod.rs`** wires focus events, the notifier's timer and the speaker into the drive loop.

Choices made here, inside what the architecture fixed:

- **The report type lives in the vendored crate.** `ratatui-ghostty` can't depend on `ship-core`, so `ProgramStatus` mirrors the upstream report there, and the pane task converts it to `StatusRecord`. The two types differ in purpose: one is what libghostty hands over, the other is what Ship publishes.
- **Unknown values stay out of `StatusRecord`.** The scanner drops reports with an unknown state and treats unknown `kind` values as absent, as Ghostty does (FR-002). So the record's enums are closed and need no catch-all variant.
- **The record map is an `IndexMap` kept in update order.** A put removes the id and reinserts it at the end, so eviction removes the first entry and the JSON lists records oldest update first. `indexmap` is already a dependency of `ship-core` with serde and utoipa support. `StatusRecord` has no `id` field, since the key holds it.
- **Reports are checked once, where they're parsed.** The scanner applies the specification's checks: it drops a report that breaks a limit, holds a control character in decoded text, or has an `app` outside its character set. Ghostty's parser does the same after the swap, so `records.rs` repeats none of it. Dropping the whole report satisfies FR-003: nothing with control characters or past the limits is kept. The one thing no parser does is the specification's advice to disarm bidirectional formatting characters when text is shown outside the terminal grid, so `records.rs` strips those from `title` and `msg` before keeping a record, and every client gets clean text.
- **The root key is a named constant.** `ship_core::model::ROOT` is `""`, and every lookup of the root record uses it. The record rules mention it once, in `within`, because the root is everyone's ancestor.
- **A component owns a setting only when a change needs a reaction.** Today's settings each do: mouse capture sends a terminal command, a new sidebar width drops a dragged one, and the keymap moves into `Keys`. The agent settings don't; each is read where it's used, and the next read sees a reload. So `Local` keeps `agents: AgentSettings`, which `reload` replaces whole, and each reader takes only the piece it needs as an argument: `delay` by value, `rows` as a slice, a resolved `Clip`, `popups` read in place. Rejected: a `configure` method on `Attention`, `Speaker` and the agents section, each of which would only assign a field.
- **"In view" is one `Observer` question.** `Observer::in_view(pane)` answers it from the record's selection and the tab's geometry. `Observer::apply`'s `Screen` arm already computes the same thing inline and switches to it. `Attention` and the sidebar take `&Observer` and ask it, instead of callers assembling tabs, selections and in-view lists for them.
- **`Attention` keeps one map of panes.** Each value holds the pane's last root state, its pending notice and its unseen mark, so `observe` forgets a closed pane or a lost root record in one place and `attached` clears one map. The value is a struct, not an enum, because a pending notice and an unseen mark coexist. Rejected: separate maps for states, pending notices and unseen marks, which must each be pruned.
- **Display states are derived on every draw, not stored.** `agents::display` reads the root record and the notifier's unseen set. Only the unseen set is state (`docs/agents/code.md`: store what can't be derived).
- **"In view" is the selected tab's `geometry.panes`.** The server already narrows that map to the zoomed pane when the tab is zoomed (`state/geometry.rs`), so zoom needs no client logic.
- **The notifier's timer is one `sleep_until` arm in the drive loop**, armed at the earliest pending due time. No task or interval is added.
- **The speaker only plays.** It is a plain thread with a `std::sync::mpsc` channel of `Clip`s. rodio's output stream isn't `Send`. A send to a dead thread is ignored, so a missing device costs nothing after startup (FR-023). It doesn't know about paths, notices or fallbacks.
- **Sound files become clips when the config loads.** Loading reads each `path`, probes it by decoding its header, and keeps the bytes. A missing or undecodable file is replaced by the bundled clip for the slot it was set for (`default.done`, `apps.droid.attention`), with a warning naming the file (FR-023). Loading is where the slot is known, so the fallback needs no extra data downstream. A bad path is reported when the config is saved rather than at the first notification, and a file deleted after loading still plays. Gap: the probe reads the header, so a file corrupt past it fails during playback, and the thread logs it and plays nothing. Rejected: decoding on the audio thread with the notice passed along for the fallback, which made playing also resolve and recover.
- **Popups are a raw OSC 9 write** through `TerminalGuard`, because crossterm has no OSC 9 command. Record text arrives clean, but the line also carries tab and pane labels, which can come from a program's window title, so the assembled line has control characters removed before it's written. `;` stays, since OSC 9's body runs to the terminator.
- **Sound paths resolve at load time** against the config file's directory, with `~` expanded, before they're read.
- **One `Sounds` shape for config and clips.** `Sounds<S>` and `NoticeSounds<S>` are generic over what fills a slot: `Sound` as written in the file, `Cue` once loaded. Loading maps one into the other, so the defaults-and-apps structure is written once.
- **The agents section scrolls with the wheel only.** The tabs section keeps tui-tree-widget's selection-follow scrolling and also gains wheel scrolling. Neither gets keys, since the product only requires overflow to scroll (FR-014).
- **The tabs section keeps its `TreeState`.** `draw` builds a fresh `TreeState` every frame today (`sidebar.rs` `draw`), so no scroll offset survives a frame. `sidebar::Tabs` keeps the state across frames, reselects the viewed tab on each draw, and scrolls it on wheel events. Selection-follow scrolling then comes from tui-tree-widget as it does now.
- **The sections are reached through `Sidebar`'s public fields.** Callers write `sidebar.tabs.reveal(..)` instead of `Sidebar` forwarding each tree method, which would add lines without adding a boundary.

Estimated change: about 920 lines of implementation Rust, unmeasured. No test code (`docs/agents/code.md`).

| Area | Lines |
| --- | --- |
| Scanner and session wiring (temporary) | about +90 |
| `ship-core` model | about +45 |
| `records.rs`, pane task, state, saver | about +100 |
| `agents.rs`: display states, rollup, list, rows, jump | about +160 |
| `attention.rs`: transitions, delay, unseen, focus | about +130 |
| `sidebar/` and `mouse.rs`: frame, sections, split, scrolling, hits (the tree's move is not counted) | about +190 |
| `sound.rs` | about +70 |
| `keymap.rs`: settings and actions | about +80 |
| `mod.rs`, `terminal.rs`, `draw.rs` | about +60 |

## Skeleton map

### Proposed settings

```diff
 # Cargo.toml [workspace.dependencies]
+base64 = "0.23"
+rodio = { version = "0.23", default-features = false, features = ["playback", "mp3", "wav", "vorbis", "flac"] }
```

```diff
 # vendor/ratatui-ghostty/Cargo.toml
+base64.workspace = true
```

```diff
 # crates/ship-client/Cargo.toml
+rodio.workspace = true
```

rodio's default `recording`, `mp4` and `dither` features are left out. `wav`, `vorbis` and `flac` stay so that custom sounds aren't limited to MP3. On Linux, `playback` pulls in cpal's ALSA backend, which needs `libasound2-dev` (or the distro's equivalent) at build time (slice 4).

```diff
 # crates/ship-client/src/keymap/defaults.toml
 [client.modes.normal.keys]
+"alt-a"       = { action = { client.agents.next = {} } }

 [client.modes.resize.keys]
+"shift-j" = { action = { client.sidebar.resize = { direction = "down" } }, label = "split down" }
+"shift-k" = { action = { client.sidebar.resize = { direction = "up" } }, label = "split up" }
```

Bundled assets: Kenney's Interface Sounds `confirmation_002.wav` and `error_007.wav` are copied to `crates/ship-client/assets/sounds/done.wav` and `attention.wav`, with an `ATTRIBUTION.md` beside them naming the pack and its CC0 1.0 dedication (FR-028).

### Added

`vendor/ratatui-ghostty/src/program_status.rs` (slice 1; temporary)

```rust
//! OSC 7501 reports, OSC 133 prompt starts and RIS, found ahead of libghostty
//! until libghostty-rs binds `GHOSTTY_TERMINAL_OPT_PROGRAM_STATUS`.

use base64::Engine;

/// Upstream's support reply body (`program_status.query_reply`).
pub(crate) const QUERY_REPLY: &str =
    "?:states=idle,working,done,blocked,error:kinds=permission,question,auth";

/// The longest sequence kept while waiting for its terminator.
const MAX_SEQUENCE: usize = 4096;

/// Shaped like `GhosttyTerminalProgramStatus`.
#[derive(Clone, Debug)]
pub struct ProgramStatus {
    pub state: ProgramState,
    pub kind: Option<ProgramKind>,
    pub progress: Option<u8>,
    /// Empty for the root record.
    pub id: String,
    pub app: Option<String>,
    pub title: Option<String>,
    pub message: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramState { Idle, Working, Done, Blocked, Error, Clear }

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProgramKind { Permission, Question, Auth }

/// What a batch held, in byte order.
pub(crate) enum Found {
    /// The support query and the terminator it used, which the reply repeats.
    Query(Terminator),
    Report(ProgramStatus),
    PromptStart,
    /// RIS. The session reports it as a `clear` with an empty id.
    Reset,
}

#[derive(Clone, Copy)]
pub(crate) enum Terminator { Bel, St }

/// Carries a sequence split across batches.
#[derive(Default)]
pub(crate) struct Scanner {
    state: Scan,
    sequence: Vec<u8>,
}

#[derive(Default)]
enum Scan { #[default] Ground, Escape, Osc, OscEscape }

impl Scanner {
    pub fn scan(&mut self, bytes: &[u8], found: impl FnMut(Found));
}

/// Upstream's validation: `state` required and known, limits enforced,
/// `title` and `msg` valid base64 of UTF-8 without control characters,
/// unknown keys and malformed pairs skipped.
fn report(body: &[u8]) -> Option<ProgramStatus>;
```

`crates/ship-server/src/records.rs` (slice 1)

```rust
//! A pane's status records under revision 0.4's rules.

use indexmap::IndexMap;
use ratatui_ghostty::program_status::{ProgramState, ProgramStatus};
use ship_core::model::StatusRecord;

/// G6.
const MAX_RECORDS: usize = 64;

/// Keyed by id, `ROOT` for the root, oldest update first.
#[derive(Default)]
pub(crate) struct Records(IndexMap<String, StatusRecord>);

impl Records {
    /// Put replaces the record under the report's id completely and moves it
    /// to the end, evicting the first record past `MAX_RECORDS`; `clear`
    /// removes every id `within` the report's. Returns whether anything
    /// changed.
    pub fn apply(&mut self, report: ProgramStatus) -> bool;

    /// Drops working and blocked records (FR-004).
    pub fn prompt(&mut self) -> bool;

    pub fn snapshot(&self) -> IndexMap<String, StatusRecord>;
}

/// Whether `id` is `ancestor` or beneath it. The root is every id's ancestor.
fn within(id: &str, ancestor: &str) -> bool;

/// Bidirectional formatting characters removed; the scanner already refused
/// control characters and overlong text.
fn disarm(text: String) -> String;
```

`crates/ship-client/src/ui/agents.rs` (slices 2 and 3)

```rust
//! Display states, the tab rollup, the agent list and the jump target.
//! No rendering; `sidebar::agents` draws them.

use ship_core::{
    id::IdOf,
    model::{NodeId, Pane, StatusRecord, Tab},
    tree::Tabs,
};

use super::attention::Attention;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Display { Blocked, Error, Unseen, Working, Idle }

impl Display {
    pub fn label(self) -> &'static str;
    /// FR-011's order; blocked and error share the top rank.
    fn rank(self) -> u8;
}

/// One listed pane, in agent-list order.
pub(super) struct Agent<'a> {
    pub pane: &'a Pane,
    pub root: &'a StatusRecord,
    pub display: Display,
    /// Tab labels from the top level (FR-016).
    pub path: Vec<String>,
}

/// The pane's root record and its display state, if it has one.
pub(super) fn display(pane: &Pane, attention: &Attention) -> Option<Display>;

/// Highest-ranked display among the tab's panes and descendants (FR-011).
pub(super) fn rollup(tab: &Tab, attention: &Attention) -> Option<Display>;

/// By rank, then depth-first tree order (FR-015).
pub(super) fn list<'a>(tabs: &'a Tabs, attention: &Attention) -> Vec<Agent<'a>>;

/// Blocked and error first, then unseen, else every agent, starting after
/// `selection` (FR-019).
pub(super) fn next(agents: &[Agent], selection: Option<NodeId>) -> Option<IdOf<Pane>>;
```

`crates/ship-client/src/ui/sidebar/agents.rs` (slice 2)

```rust
//! The sidebar's agents section.

use ratatui::{layout::{Position, Rect}, text::{Line, Span}, Frame};
use ship_core::{id::IdOf, model::Pane};

use super::Scroll;
use crate::keymap::Token;
use crate::ui::{agents::{self, Agent, Display}, attention::Attention};

/// G5; also used for the tabs' rollup icons.
pub(super) fn icon(display: Display) -> Span<'static>;

#[derive(Default)]
pub(in crate::ui) struct Agents {
    /// The first drawn agent.
    offset: usize,
    /// Agent rows as last drawn, for hit-testing.
    drawn: Vec<Drawn>,
    area: Rect,
}

/// One agent's rows on screen.
struct Drawn {
    pane: IdOf<Pane>,
    area: Rect,
}

impl Agents {
    /// The agent whose rows contain `at`.
    pub fn hit(&self, at: Position) -> Option<IdOf<Pane>>;
    pub fn scroll(&mut self, scroll: Scroll);

    /// The rows of `agents::list`, or the "no agents" placeholder (FR-012).
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        tabs: &ship_core::tree::Tabs,
        attention: &Attention,
        rows: &[Vec<Token>],
    );
}

/// The row's lines, dropping empty tokens and empty lines; `tab` shortened
/// from the left to `width` (FR-016, FR-017).
fn row(agent: &Agent, rows: &[Vec<Token>], width: u16) -> Vec<Line<'static>>;
```

`crates/ship-client/src/ui/attention.rs` (slice 3)

```rust
//! When a change in a root record becomes a notification or an unseen mark.

use std::{
    collections::HashMap,
    time::{Duration, Instant},
};

use ship_core::{id::IdOf, model::{Pane, ReportedState}};

use super::observer::Observer;
use crate::keymap::Notice;

/// A notification waiting out the delay (FR-021).
struct Pending {
    notice: Notice,
    /// The state that must still hold when it's due.
    state: ReportedState,
    due: Instant,
}

/// A fired notification, carrying what the sound and popup need.
pub(super) struct Fired {
    pub pane: IdOf<Pane>,
    pub notice: Notice,
    /// The record's own `app`, which picks the app's sounds.
    pub app: Option<String>,
    /// The app, else the pane's label (FR-017), for the popup.
    pub name: String,
    pub path: String,
    pub message: Option<String>,
}

pub(super) struct Attention {
    /// Panes with a root record in the last replica seen.
    panes: HashMap<IdOf<Pane>, Watched>,
    /// Whether the client's window has focus. Starts true, so a terminal
    /// that never reports focus counts as focused (FR-008).
    focused: bool,
}

/// What the notifier knows about one pane. A pane can have a pending notice
/// and an unseen mark at once: it finished unseen, then finished again.
struct Watched {
    /// Its root state in the last replica seen.
    state: ReportedState,
    pending: Option<Pending>,
    unseen: bool,
}

/// No panes, focused.
impl Default for Attention;

impl Attention {
    /// Forget everything and take the replica as the baseline, so nothing
    /// that happened before the attach notifies (FR-009).
    pub fn attached(&mut self, observer: &Observer);

    /// Turn root-state changes into notices due after `delay` (FR-007,
    /// FR-020, FR-021).
    pub fn observe(&mut self, observer: &Observer, now: Instant, delay: Duration);

    pub fn set_focused(&mut self, focused: bool);

    /// Clear unseen marks for panes in view while focused (FR-008).
    pub fn viewed(&mut self, observer: &Observer);

    pub fn due(&self) -> Option<Instant>;

    /// Notices whose state held, minus those in view while focused; done
    /// notices that fire mark their pane unseen.
    pub fn fire(&mut self, observer: &Observer, now: Instant) -> Vec<Fired>;

    pub fn unseen(&self, pane: IdOf<Pane>) -> bool;
}
```

`crates/ship-client/src/sound.rs` (slice 4)

```rust
//! Notification sounds: clips loaded with the config, played on a thread
//! that owns the audio device.

use std::{path::Path, sync::{mpsc, Arc}, thread};

use crate::keymap::Notice;

/// Audio bytes whose header decodes; cheap to clone.
#[derive(Clone)]
pub struct Clip(Arc<[u8]>);

impl Clip {
    /// The notice's bundled sound, from `include_bytes!`.
    pub fn bundled(notice: Notice) -> Self;

    /// Reads `path` and probes it with rodio's decoder.
    pub fn load(path: &Path) -> Result<Self>;
}

/// A loaded slot: silence or a clip.
#[derive(Clone)]
pub enum Cue {
    Off,
    Play(Clip),
}

/// Dropping it ends the thread.
pub struct Speaker(mpsc::Sender<Clip>);

impl Speaker {
    /// Opens the default output on the thread; with no device, the thread
    /// logs once and exits, and later plays are ignored.
    pub fn start() -> Self;

    pub fn play(&self, clip: Clip);
}
```

`crates/ship-client/assets/sounds/` (slice 4): `done.wav`, `attention.wav` and `ATTRIBUTION.md`, as in Proposed settings.

### Changed

`vendor/ratatui-ghostty/src/lib.rs` (slice 1)

```diff
 pub mod input;
+pub mod program_status;
 pub mod session;
```

`vendor/ratatui-ghostty/src/session.rs` (slice 1)

```diff
 pub enum SessionEvent {
     ...
     ClipboardWrite(String),
+    /// An OSC 7501 report; RIS arrives as `clear` with an empty id.
+    ProgramStatus(ProgramStatus),
+    /// OSC 133 `A`: the shell started a new prompt.
+    PromptStart,
 }
```

```diff
     loop {
         pending.take(&mut batch);
         if !batch.is_empty() {
+            // Ahead of vt_write, so the support reply precedes libghostty's
+            // answer to a DA1 in the same batch (FR-001).
+            scanner.scan(&batch, |found| state.found(found));
             state.terminal.vt_write(&batch);
```

```rust
impl SessionState<'_> {
    /// Writes the support reply, or queues the event.
    fn found(&mut self, found: Found);
}
```

`crates/ship-core/src/model.rs` (slice 1)

```diff
 pub struct Pane {
     ...
     pub status: PaneStatus,
+    /// What the pane's program last reported through OSC 7501, keyed by
+    /// record id with `ROOT` for the root, oldest update first; absent when
+    /// none (G1).
+    #[serde(default, skip_serializing_if = "IndexMap::is_empty")]
+    pub program: IndexMap<String, StatusRecord>,
 }
```

```rust
/// The root record's id: the empty name, which the id grammar can't produce.
pub const ROOT: &str = "";

/// One record of a pane's program status (OSC 7501, revision 0.4). Its id
/// is the key in `Pane::program`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct StatusRecord {
    pub state: ReportedState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<BlockedKind>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum ReportedState { Idle, Working, Done, Blocked, Error }

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub enum BlockedKind { Permission, Question, Auth }

impl Pane {
    /// `self.program.get(ROOT)`.
    pub fn root_status(&self) -> Option<&StatusRecord>;
}
```

Every `Pane { .. }` literal in the server (`state.rs`'s `start_pane`, restore) gains `program: IndexMap::new()`.

`crates/ship-server/src/pane.rs` (slice 1)

```diff
 pub(crate) enum PaneChange {
     Title(Option<String>),
     Dir(String),
     Exited(ExitStatus),
+    /// The pane's whole record set after a change.
+    Program(IndexMap<String, StatusRecord>),
 }

 struct PaneTask {
     ...
+    records: Records,
 }
```

`publish` matches `SessionEvent::ProgramStatus(report)` with `records.apply` and `SessionEvent::PromptStart` with `records.prompt`, in drain order. When either returns true, it sends one `PaneChange::Program(records.snapshot())` after the loop, next to the title change.

`crates/ship-server/src/lib.rs` (slice 1): `mod records;`.

`crates/ship-server/src/state.rs` (slice 1)

```diff
     match change {
         PaneChange::Title(title) => pane.title = title,
         PaneChange::Dir(dir) => pane.dir = Some(dir),
-        PaneChange::Exited(status) => pane.status = PaneStatus::Exited(status),
+        PaneChange::Exited(status) => {
+            pane.status = PaneStatus::Exited(status);
+            pane.program.clear();
+        }
+        PaneChange::Program(records) => pane.program = records,
     }
```

`crates/ship-server/src/saver.rs` (slice 1): `blank` adds `pane.program.clear();`.

`crates/ship-client/src/keymap.rs` (slices 2 to 4)

```diff
 pub struct ClientSettings {
     pub mouse: bool,
     pub keymap: Keymap,
     pub sidebar_width: u16,
+    pub agents: AgentSettings,
 }

 pub enum ClientAction {
     ...
+    Agents(ClientAgents),
 }

 pub enum Sidebar {
     Toggle {},
     Resize {
-        direction: Side,
+        /// Left and right resize the width; up and down move the split (G4).
+        direction: Direction,
         amount: Option<u16>,
     },
 }

 struct Client {
     ...
+    #[serde(default)]
+    agents: AgentsFile,
 }
```

```rust
#[derive(Clone, Debug, Deserialize, strum::IntoStaticStr)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
#[strum(serialize_all = "snake_case")]
pub enum ClientAgents {
    /// FR-019.
    Next {},
}

/// Resolved: defaults applied, sounds loaded into clips.
pub struct AgentSettings {
    pub rows: Vec<Vec<Token>>,
    pub delay: Duration,
    pub popups: bool,
    pub sounds: Sounds<Cue>,
}

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Token { Icon, App, State, Tab, Pane, Title, Msg }

/// The two kinds of notification (FR-020). In config it also names a
/// bundled sound: `{ builtin = "done" }`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Notice { Done, Attention }

/// `"off"`, `{ builtin = "done" }` or `{ path = "~/ding.mp3" }`.
#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Sound {
    Off,
    Builtin(Notice),
    Path(PathBuf),
}

/// `[client.agents.sounds]`: the defaults and each app's overrides (G2).
/// `S` is `Sound` as written, `Cue` once loaded.
#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Sounds<S> {
    #[serde(default)]
    pub default: NoticeSounds<S>,
    /// Keyed by the record's `app`.
    #[serde(default)]
    pub apps: HashMap<String, NoticeSounds<S>>,
}

/// One slot per notice; unset falls through to the next level.
#[derive(Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NoticeSounds<S> {
    pub done: Option<S>,
    pub attention: Option<S>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct AgentsFile {
    rows: Option<Vec<Vec<Token>>>,
    delay_ms: Option<u64>,
    popups: Option<bool>,
    #[serde(default)]
    sounds: Sounds<Sound>,
}

impl Sounds<Sound> {
    /// Each path read into a clip; a bad file becomes the bundled clip for
    /// its slot's notice, with a warning naming it (FR-023).
    fn load(self, base: &Path) -> Sounds<Cue>;
}

impl Sounds<Cue> {
    /// The app's cue, else the default's, else the notice's bundled clip;
    /// `None` for `off`.
    pub fn resolve(&self, app: Option<&str>, notice: Notice) -> Option<Clip>;
}
```

`ClientSettings::load` passes the config path to `over_defaults`, which calls `Sounds::load` with the config's directory: paths resolve against it with `~` expanded, `builtin` becomes `Clip::bundled`, and `off` becomes `Cue::Off`. Unset slots stay unset, and `resolve` falls back to `Clip::bundled(notice)`. A bad file is a warning, not a load error, so the rest of the config still applies. `Sidebar::nudge` takes the `Direction` and handles left and right only; `now` sends up and down to `nudge_split`.

`crates/ship-client/src/ui/sidebar.rs` becomes `crates/ship-client/src/ui/sidebar/mod.rs`, the frame (slice 2; see Moved)

```rust
mod agents;
mod tabs;

pub(super) use self::{agents::Agents, tabs::{movable, Gap, Tabs}};

pub(super) struct Sidebar {
    pub shown: bool,
    configured: u16,
    chosen: Option<u16>,
    /// The dragged split as a share of the content height, until the client
    /// exits (FR-013).
    split: Option<Ratio>,
    area: Rect,
    pub tabs: Tabs,
    pub agents: Agents,
}

/// A wheel step.
#[derive(Clone, Copy)]
pub(super) enum Scroll { Up, Down }

pub(super) enum Hit {
    Heading,
    Button,
    /// The right-hand border, which resizes the width.
    Divider,
    /// The row between the sections, which moves the split.
    Split,
    Tabs(tabs::Hit),
    /// An agent's rows (FR-018).
    Agent(IdOf<Pane>),
}

impl Sidebar {
    // new, width, configure, set_width, nudge and clamp stay as they are.

    /// The divider row between the sections, from `area` and `split`.
    fn split_row(&self) -> u16;
    /// 10–90% of the content height, at least 3 rows per section.
    pub fn set_split(&mut self, y: u16);
    /// Up and down only; `nudge` takes left and right (G4).
    pub fn nudge_split(&mut self, direction: Direction, amount: u16);
    /// Sent to the section under `at`.
    pub fn scroll(&mut self, at: Position, scroll: Scroll);
    pub fn hit(&self, at: Position) -> Option<Hit>;

    /// Heading, tabs on top with rollup icons, the divider row, agents
    /// below (FR-012). The tabs and selection come from `observer`.
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        observer: &Observer,
        attention: &Attention,
        rows: &[Vec<Token>],
        gap: Option<&Gap>,
    );
}
```

`crates/ship-client/src/ui/sidebar/tabs.rs` (slice 2; see Moved)

```rust
//! The sidebar's tabs section: today's tree, moved from `sidebar.rs`.

pub(in crate::ui) struct Tabs {
    collapsed: HashSet<IdOf<Tab>>,
    /// Kept across frames so the offset survives and wheel scrolling works.
    state: TreeState<IdOf<Tab>>,
    area: Rect,
    drawn: Vec<Drawn>,
}

pub(in crate::ui) enum Hit {
    Row(Row),
    /// Below the last row.
    Below,
}

impl Tabs {
    // reconcile, reveal, visible, fold, toggle, gap and the row lookup move
    // unchanged.
    pub fn hit(&self, at: Position) -> Option<Hit>;
    pub fn scroll(&mut self, scroll: Scroll);
    pub fn draw(
        &mut self,
        frame: &mut Frame,
        area: Rect,
        tabs: &ship_core::tree::Tabs,
        selection: Option<NodeId>,
        gap: Option<&Gap>,
        attention: &Attention,
    );
}
```

`items` appends the tab's rollup icon (FR-011) after the zoom flag.

`crates/ship-client/src/ui/mouse.rs` (slice 2)

```diff
 pub(super) enum Clicked {
     ...
     Width(u16),
+    /// The split's new row (FR-013).
+    Split(u16),
+    /// A wheel step over a sidebar section.
+    SidebarScroll { at: Position, scroll: Scroll },
     Menu(Target, Position),
 }

 enum Gesture {
     ...
     Divider { at: Position, moved: bool },
+    Split,
 }
```

The existing arms match `Hit::Tabs(tabs::Hit::Row(row))` and `Hit::Tabs(tabs::Hit::Below)` instead of `Hit::Row` and `Hit::Below`, and `scene.sidebar.gap` becomes `scene.sidebar.tabs.gap`. A left press on `Hit::Agent(pane)` returns `Clicked::Select(NodeId::Pane(pane))` (FR-018). A press on `Hit::Split` starts `Gesture::Split`, and each drag returns `Clicked::Split(y)`. A wheel event over the sidebar returns `Clicked::SidebarScroll`.

`crates/ship-client/src/ui/terminal.rs` (slice 3)

```diff
-        execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste)
+        execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste, EnableFocusChange)
 ...
 fn restore() {
     execute!(
         io::stdout(),
+        DisableFocusChange,
         DisableMouseCapture,
```

```rust
impl TerminalGuard {
    /// One OSC 9 notification to the outer terminal (FR-024).
    pub fn notify(&self, text: &str) -> Result<()>;
}
```

`crates/ship-client/src/ui/mod.rs` (slices 2 to 4)

```diff
 struct Local {
     ...
     message: Option<String>,
+    attention: Attention,
+    speaker: Speaker,
+    /// Replaced whole on reload; readers take the piece they need.
+    agents: AgentSettings,
 }
```

```diff
 // reload
         self.sidebar.configure(settings.sidebar_width);
         self.keys.replace(settings.keymap);
+        self.agents = settings.agents;
```

Drive loop additions:

- On `SseEvent::Attached`, `attention.attached(&observer)` (slice 3).
- On `SseEvent::State`, `attention.observe(&observer, now, agents.delay)` (slice 3).
- `Event::FocusGained` and `Event::FocusLost` set `attention.set_focused` and redraw, instead of reaching `keys.handle` (slice 3).
- A new arm, `sleep_until(attention.due())` if one is pending, calls `attention.fire(&observer, now)`, then plays each `Fired` with `speaker.play` on `agents.sounds.resolve(fired.app.as_deref(), fired.notice)` when it returns a clip (slice 4) and writes the popup if `agents.popups` (slice 3). It needs no extra redraw: a done notice marks its pane unseen, which shows on the next draw.
- After each redraw, `attention.viewed(&observer)` (slice 3).
- `Clicked::Split`, `Clicked::SidebarScroll` (slice 2).
- The tree calls go through the section: `local.sidebar.tabs.reveal`, `.reconcile`, `.toggle`, `.fold` and `.visible` (slice 2).
- `now` handles `Sidebar::Resize` with `up` or `down` through `nudge_split` (slice 2).
- `act` handles `ClientAgents::Next {}` with `agents::next` and `select` (slice 3).
- `reload` replaces `agents` (slice 2).

`crates/ship-client/src/ui/draw.rs` (slice 2)

```diff
         local.sidebar.draw(
             frame,
             sidebar_area,
-            &replica.tabs,
-            local.observer.record().and_then(|record| record.selection),
+            &local.observer,
+            &local.attention,
+            &local.agents.rows,
             local.mouse.gap(),
         );
```

`pane_label` is reused for FR-017.

`crates/ship-client/src/ui/observer.rs` (slice 3)

```rust
impl Observer {
    /// Drawn in the selected tab: a key of its `geometry.panes`, which the
    /// server narrows to the zoomed pane (FR-008).
    pub fn in_view(&self, pane: IdOf<Pane>) -> bool;
}
```

`apply`'s `Screen` arm returns `self.in_view(pane)` in place of its inline check.

`crates/ship-client/src/ui/keys.rs` (slice 3): no change. `ClientAction::Agents` runs in the lane like other selection-changing actions, since it needs the replica at its turn.

Documents (slice 5):

- `docs/product-brief.md`: milestone 3 amended (FR-026).
- `GLOSSARY.md`: **Status record** and **Display state** (FR-027).
- `docs/guide.md`: the agents section, display states, the jump key, notification settings, sounds, popups, focus behavior, the OSC 133 requirement for exit recovery and the tmux limitation.
- `README.md`: "What works today" gains agent status. The default keys list gains `alt-a`. Install notes gain ALSA headers on Linux.
- `docs/agents/codebase.md`: building on Linux needs ALSA headers.
- `openspec/changes/program-status/`: `proposal.md`, `design.md` (a summary pointing at the three papers), `tasks.md` and spec deltas for the pane model (`program`), the attach replica, client display and notifications, and client config.

### Moved

- `crates/ship-client/src/ui/sidebar.rs` → `crates/ship-client/src/ui/sidebar/mod.rs` (the frame: width, divider, heading, hit dispatch) and `crates/ship-client/src/ui/sidebar/tabs.rs` (the tree: folds, `Drawn`, `Gap`, `Row`, `movable`, `items`, row lookup, gap finding and the tree half of `draw`). About 300 lines move with their behavior unchanged, apart from keeping the `TreeState`. Slice 2.

### Replaced

- `keymap::Sidebar::Resize`'s `direction: Side` becomes `direction: Direction` (G4). `Side` is removed once nothing uses it.

### Untouched

- The attach protocol, its events and the protocol version. Records ride in the existing replica.
- libghostty-rs and its pin. The tracking issue opened after merge covers the binding, the bump, adopting the semantic prompt callback that newer libghostty has (`GHOSTTY_TERMINAL_OPT_SEMANTIC_PROMPT`), and deleting the scanner, which ends the nested-sequence risk (architecture decision 3).
- The saved-tabs format (records are blanked).
- `ship-macros`, the HTTP routes, and the CLI's commands. `ship pane get` and `ship tab list` print the new field through serde.

## Build order

Every slice runs the repository's checks: `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, debug and release builds, and `openspec validate program-status --type change --strict` once the change files exist. Exercises use an owned temporary server on a free port with `XDG_STATE_HOME` and the config pointed at temporary directories, run agents in scratch directories with throwaway configs, and leave existing servers alone. Checks run on macOS. Linux is reported unverified unless it is run there.

1. **Records on panes.** Files: `program_status.rs` (new), `session.rs`, the vendored `lib.rs`, `ship-core` model, `records.rs` (new), `pane.rs`, `state.rs`, `saver.rs`, the server's `lib.rs`, workspace and vendored `Cargo.toml`. Delivers: a program's OSC 7501 reports appear in `ship pane get` under `program`, with the root record at `""`. Checks:
   - `printf '\e]7501;?\e\\'` inside a pane gets the reply, before the DA1 reply when both are sent together.
   - Claude Code, Codex, Pi and crush 0.98.1 each show their root record through a turn and a permission dialog.
   - Reports with `id=build` and `id=build/test` sit beside the root record, `id=` and `id=a//b` are dropped, and a 65th id evicts the oldest.
   - `clear`, an `id` subtree clear, `ESC c`, exit, and `kill -9` followed by an OSC 133 prompt each remove what FR-004 says.
   - A report with escape sequences in `app` or a base64 `msg` holding controls doesn't land with controls.
   - A report split across two `printf` writes still parses.
   - The saved-tabs file has no `program`.
2. **Sidebar sections and display states.** Files: `agents.rs` (new, without unseen), `sidebar.rs` moved to `sidebar/mod.rs` and `sidebar/tabs.rs`, `sidebar/agents.rs` (new), `mouse.rs`, `draw.rs`, `mod.rs`, `keymap.rs` (`rows`, `Direction` resize), `defaults.toml` (`shift-j`, `shift-k`). Delivers: P1 and P5. Agent rows, tab rollup icons, the "no agents" placeholder, the split with drag and keys, independent wheel scrolling, clicking a row to select its pane, and token rows with live reload. Checks:
   - The four agents' rows and their tabs' icons match their screens within 2 seconds (SC-001's display part).
   - Thirty tabs and twenty reporting panes in a short terminal scroll independently and feel snappy (SC-008).
   - Drag clamps at 10% and 90% with at least 3 rows each.
   - Folding, tab dragging with its gap underline, the ≡ menu and right-click menus behave as before the move.
   - `rows = [["icon", "app"], ["title"]]` reloads live, and a misspelled token fails with line and column (FR-025).
3. **Unseen, notifications and the jump key.** Files: `attention.rs` (new), `agents.rs` (unseen, `next`), `observer.rs` (`in_view`), `terminal.rs`, `mod.rs`, `keymap.rs` (`ClientAgents`, `delay_ms`, `popups`), `defaults.toml` (`alt-a`). Delivers: P2 without sound, and P3. Checks:
   - Attaching to five finished agents marks nothing unseen (SC-002).
   - A working-to-idle flip shorter than the delay marks nothing in 10 of 10 tries (SC-003's marking part).
   - With the pane in view, focused marks nothing and unfocused marks unseen until focus returns.
   - `alt-a` visits blocked panes, then unseen ones.
   - With `popups = true`, Ghostty shows `<app> needs you · <path>: <msg>`.
   - A report with escapes in `msg` produces a popup without them (SC-005).
4. **Sounds.** Files: `sound.rs` (new), the client's `lib.rs` (`pub mod sound`), assets and `ATTRIBUTION.md`, `keymap.rs` (sounds), `mod.rs`, the client's `Cargo.toml`. Delivers: the sound half of P2, and P4. Checks:
   - Done and attention play the bundled sounds.
   - A `{ path = ... }` sound plays. A missing or non-audio file logs a warning naming it when the config is saved, and the bundled sound plays in its place. `apps.droid.done = "off"` silences only that app.
   - SC-003 and SC-004 with sound.
   - With no output device (or a forced open failure), the client attaches and works, and logs once (SC-007).
   - The Linux build is reported unverified unless built there.
5. **Documents.** Files: those listed under Documents. Delivers: FR-026, FR-027, the guide and README updates, and the openspec change files. Checks: `openspec validate program-status --type change --strict`, and the guide's examples parse through `ship config check`.

## Deviation log

- **2026-10-18, slice 1: two build-driven edits outside the skeleton's file list.** The vendored demo's exhaustive session-event match ignores the two new events. `Layout` keeps its existing inline pane representation with a targeted `clippy::large_enum_variant` expectation: adding the specified inline record map crosses Clippy's size heuristic. No wire shape or layout representation was changed.
- **2026-10-18, slice 1: size and verification checkpoint.** Implementation adds 430 physical Rust lines under crate/vendor `src/`, with zero permanent test lines. The scanner is 248 lines rather than the estimated scanner-and-session total of about 90; report validation, optional base64 padding, every-pair limits and bounded split-sequence scanning account for the difference. macOS builds and 16 temporary end-to-end checks pass; the four live-agent exercises remain unverified. Installed crush is 0.76.0 rather than the required 0.98.1 or later, so task 1.3 stays open. Details and temporary evidence paths are in `tasks.md`. A subsequent isolated Claude smoke attempt stopped at first-run onboarding, before any turn or status report; Cyan had requested stopping early if isolated setup ran into issues. The owned processes were stopped. No later slice was started.
- **2026-10-18, slice 1: live agents accepted with two of four.** Cyan accepted task 1.3 as verified after Codex 0.163.0 and Pi 1.3.0 ran live on a local model. Each showed its root record matching the screen through a turn, and Codex through its approval dialog; both records cleared on exit. Claude Code and crush were not run (crush is installed at 0.76.0, below the 0.98.1 gate), so they are unverified here. Codex reports blocked without `kind` or `msg` and ends a turn as `idle` rather than `done`, which slices 3 and 4 have to handle.
- **2026-10-18, slice 2: small departures from the skeleton.** A click on an agent entry selects on release and not after a drag, through a `Gesture::Agent`, because the mouse-input delta requires that for every sidebar click; the skeleton's "left press returns `Clicked::Select`" contradicted it. The split is stored as an `f32` share rather than a named `Ratio`, since no such type exists and one field doesn't need it. `tabs` is a `pub(super)` module so `mouse.rs` can name `tabs::Hit`. `Tabs` also keeps the last drawn row count so a wheel scroll stops once the last row reaches the bottom; tui-tree-widget alone scrolls until only the last row shows. `Agents` has no `area` field, because each drawn entry keeps its own. The divider row carries an `agents` label, matching the `tabs` heading. As planned, `Display` has no `Unseen` and nothing takes an `Attention` until slice 3. Size came in at +561 lines against roughly 400 estimated for these parts.
- **2026-10-18, after slice 2: G5 amended to follow Herdr.** At Cyan's request, unseen is cyan, not blue, matching the color Herdr's `terminal` theme gives its `done` state (finished, not yet looked at), and the `state` token takes its icon's color, as Herdr's status text does (`src/client/shell.rs` and `agent_sidebar.rs` at `143bf5fd`). Shapes stay Ship's, since Herdr's default dots share `●` across states and FR-010 needs shape too. Idle stays dark gray rather than Herdr's green, because unseen already carries "finished and waiting" and idle is the resting state of most rows. Blocked stays red rather than the theme's light red. The agent-attention spec's Icons requirement changed with it.
- **2026-10-18, slice 3: small departures from the skeleton.**
  - `Notice` arrived in `keymap.rs` in this slice rather than slice 4, because the notifier needs it.
  - `Pending` has no `state`. A due notice fires if the current state still belongs to its kind (blocked or error for attention, idle or done for done), so a pane that goes working, idle, then done inside the delay still finishes once.
  - `Fired` has no `pane` or `app` yet. Nothing reads them until slice 4 resolves sounds by `app`.
  - `viewed` runs at the start of each redraw rather than after it, so the frame that shows a pane already shows it seen.
  - `Agent::name` and `Agent::tab` hold the app-or-label fallback and the path joining, so row tokens and popups share them.
  - A pane that appears with a root record already blocked or error counts as becoming blocked and notifies. Only the attach baseline suppresses that.
  - Size came in at +370 lines against roughly 130 estimated for `attention.rs` plus this slice's parts of `agents.rs`, `keymap.rs`, `mod.rs` and `terminal.rs`.

- **2026-10-18, after slice 3: unseen marks at once, as in Herdr.** At Cyan's request, a working-to-idle or done change marks the pane unseen as soon as the client sees it, unless the pane is in view in a focused window. The delay now holds back only the notification. Before, the pane showed `○` for the delay and then `◆`. Herdr sets its `seen` flag on the completion transition and delays only the toast and sound (`apply_pane_state_change` in `src/app/actions.rs`, Herdr 0.7.5 source). A flip within the delay still changes the icon briefly either way, so holding back the mark kept nothing steadier. FR-007, the quick-flip scenario, the proposal and the agent-attention spec's Unseen marks requirement changed with it. In the code, `observe` sets the mark and `fire` no longer does, so the timer arm no longer redraws and the slice 3 entry's note about that redraw was dropped.

- **2026-10-18, after slice 3: the selected pane's agent entry is highlighted.** At Cyan's request, following Herdr, which gives the focused pane of the active tab a `surface_dim` band with `text`-colored names (`src/ui/sidebar.rs`, Herdr 0.7.5 source); its ANSI `terminal` palette makes those dark gray and the terminal's default. Ship draws the selected pane's entry with the default foreground on dark gray and every other entry gray as before. Only the selected pane counts, not every agent in view, so a tab with several agents highlights one entry and a selected tab highlights none. FR-015 and the agent-attention spec's Agent list requirement gained a sentence and a scenario. `Agents::draw` takes the attachment's selection, for +13 lines.

- **2026-10-18, after slice 3: idle is green.** At Cyan's request, idle moved from dark gray to green, the ANSI color Herdr's `terminal` theme gives a seen idle agent (`agent_icon` in `src/ui/status.rs`, Herdr 0.7.5 source). The selected agent's band is dark gray, so a dark gray idle icon and `state` token vanished on it in every terminal. The earlier amendment kept idle dark gray to keep resting rows quiet; that no longer holds up once rows can sit on the band. G5 and the agent-attention spec's Icons requirement changed with it.

- **2026-10-18, after slice 3: tab icons sit in a left gutter.** At Cyan's request, a tab row's rollup icon moved from after its label to a fixed column at the row's left edge, so nesting depth and label length no longer move it. Herdr puts the `state_icon` ahead of the name on its flat workspace rows. `Tabs::draw` renders the tree three columns in from the section's edge and writes each drawn row's icon into the middle column of that gutter, which lines up with the agents section's icons; the selected row's band is painted across the gutter too. The widget's own highlight symbol is gone. A first attempt reused that symbol's column instead, but tui-tree-widget only draws it while something is selected, and a fresh attachment has no selection until the user picks a tab, so Cyan saw no tab icons until switching tabs. Fold-marker hits, gutter clicks and drop lines now measure from the fixed gutter, so labels also no longer shift one column when a selection first appears. Tab labels lose two columns. `items` no longer takes the attention state. FR-011 and the agent-attention spec's Tab rollup requirement gained a sentence and a scenario; +15 lines.

- **2026-10-18, after slice 3: the jump cycles every agent when nothing needs the user.** At Cyan's request, `alt-a` goes through the whole agent list, in list order, when no pane is blocked, error or unseen, rather than doing nothing. Herdr's `next_agent` always cycles every panel entry (`relative_agent_entry` in `src/app/input/navigate.rs`, Herdr 0.7.5 source); Ship keeps the narrower cycle whenever something needs the user, so the jump still works as a queue then. Both rebuild the list on each press and step past the selected pane, so an agent that changes state and moves mid-cycle can be skipped or visited twice; with nothing waiting, that only affects browsing. FR-019, the proposal and the agent-attention spec's Jump requirement changed, gaining a No agents scenario; +4 lines.

- **2026-10-18, before slice 4: Kenney's sounds replace Herdr's.** At Cyan's request, the bundled sounds are `confirmation_002` for done and `error_007` for attention from Kenney's Interface Sounds 1.0, which is CC0, rather than Herdr's `done.mp3` and `request.mp3`. With Herdr's, a machine running Herdr and one running Ship sound the same, and neither the files (no tags) nor Herdr's commit `0403775f` that added them says where they came from, so FR-028's provenance check had nothing to go on. Cyan picked the two by ear. They are bundled as WAV from Calinou's lossless repackaging of the pack, `done.wav` and `attention.wav`, about 66 KB together. FR-028, Proposed settings, the assets line and task 4.1 changed with it. The same check found that this paper's Herdr citations said 0.9.5, the installed binary's version, while the source read was Herdr's 0.7.5 source; they now say so.

- **2026-10-18, slice 4: small departures from the skeleton.**
  - The speaker opens the default output for each clip and closes it when the clip ends, rather than holding it open from `start`. On macOS an open output holds a `PreventUserIdleSystemSleep` assertion for as long as it's open, so a running client stopped the Mac from idle sleep, and it cost the idle client 0.6–0.8% CPU against 0.0% without it. A failed open logs the first time only and later clips try again, so a device plugged in later works. A clip that can't decode is logged and skipped before the device opens. Herdr avoids the same problem by starting a player process per sound.
  - rodio's `tracing` feature is on. Without it rodio prints stream errors and the drop notice to stderr, which would land on the client's screen.
  - `ship config check` calls a new `ClientSettings::check`, which parses without reading sound files. The command role logs warnings to stderr, so loading clips there would have reported bad sound files, which config-file's Sound paths forbids.
  - `Sounds` and `NoticeSounds` implement `Default` by hand and carry a serde `bound`, because the derives would require `Sound: Default`. `NoticeSounds::get` picks a notice's slot.
  - `~` expands through `std::env::home_dir`.
  - `Fired` gained `app` but not `pane`, which nothing reads.
  - Size came in at +244 lines.

- **2026-10-18, before slice 1: the openspec change files were written early.** Slice 5 listed `proposal.md`, `tasks.md` and the spec deltas as its deliverable. Cyan asked for them once the papers locked, so they exist before any code, as they did for session-restore. Slice 5 still revalidates them.
- **2026-10-18, before slice 1: there is no `ship pane list`.** The product paper's FR-005, the architecture paper's ServerState box and this paper's Rationale and Untouched sections named it, but the CLI has only `ship tab list`, which prints every pane. At Cyan's request all three papers now say `ship tab list`. Nothing in the plan changes: `ship pane get` and `ship tab list` both print `program` through serde.
