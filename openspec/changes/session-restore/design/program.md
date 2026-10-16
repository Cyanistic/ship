# Session restore program

Status: Locked on 2026-10-16, approved by Cyan ("it LGTM!"). Approval doesn't start implementation; Cyan requests each slice separately. Written on 2026-10-16 from the locked [product](product.md) and [architecture](architecture.md) papers, against the crates at `4c3d2c8`, after diagnostics merged. Revised the same day after Cyan flagged code smells in the skeleton: the saver is a struct instead of a flag argument, the change check compares `Tabs` through derived `PartialEq` instead of JSON bytes, `start_pane` takes its ID, `--no-restore` is its own `serve` parameter, path resolution has no side effect, and the `apply` split is gone (with architecture decision 2 amended to match).

Gates:

- **G1, building before diagnostics merges.** Settled: diagnostics merged into `main` (`4c3d2c8`) and this branch was rebased onto it, so the skeleton uses its `runlog.rs` (state directory lookup, the `0700` helper), its `log` parameter on `serve` and `#[traced]` directly. Cyan: "diagnostics are merging soon", then "just merged it!"

Settled at lock, as recommended (Cyan's approval carried no notes on either):

- **G2, the starter tab counts as a change.** Architecture decision 5 makes the restored tree the saver's baseline, and the starter tab is added after it. So a first run writes a file holding the starter tab 5 seconds after start, and `ship server --no-restore --starter` replaces the saved tabs 5 seconds after start even if the user does nothing (the old file survives as `.prev`). Accepted: it follows decision 5 and `.prev` covers the mistake. Rejected: taking the baseline after the starter, which reopens decision 5.
- **G3, two sibling tabs with the same ID in a hand-edited file.** `Tabs` is an `IndexMap`, and serde keeps the last of two equal keys in one JSON object, so `check` never sees the first one and that tab is dropped without a warning. Duplicates anywhere else (a pane ID twice, the same tab ID under two parents, a key that isn't its tab's `id`) are caught as decision 9 says. Accepted: no command can produce it. Rejected: a custom `Tabs` deserializer that rejects repeated keys, about 20 lines kept by hand.

## Rationale

This is a document-only plan. Nothing below exists as source yet, and none of it has been compiled.

The skeleton follows the architecture's structure box by box:

- **`saver.rs`** is the only code that touches the file: `load` at startup, `project` for the change check, and the `Saver` task, which owns what it last wrote and whether it has kept `.prev`.
- **`state.rs`** gives `start_pane` an ID parameter and makes `ServerState::new` take the saved tabs and build its tree from them. `commit` is unchanged.
- **`lib.rs`** loads before building the state unless `restore` is false, subscribes pane events before the constructor, starts the starter only when the restored tree is empty, starts the saver, and awaits it after the actors stop.
- **`crates/ship`** adds `--no-restore`, creates `<state>/ship/tabs/` with mode `0700` and passes `<port>.json` inside it.
- **`ship-core`** derives `PartialEq` on `Tab`, `Pane`, `Layout`, `Split` and `TabGeometry`.

Choices made here, inside what the architecture fixed:

- **The change check compares `Tabs`.** `project` returns the projected tree, and the saver compares it with `==` against the tree it last wrote; serializing happens only in a write. Every field type already derives `PartialEq` except the five named above, so it costs one word on each derive line. `Ratio` is `PartialEq` without `Eq`, so none of them get `Eq`.
- **The saver's state is a struct.** `Saver` holds the path, the last written projection and whether `.prev` has been kept this run, so `save` takes no flag and `run` takes `self`.
- **Restored panes start at `Size::FALLBACK`**, like the starter pane. The first attach resizes them through `apply_sizes`, as it does for every pane today.
- **Restore builds the tree, then the constructor sets it** (amended decision 2). `restore` starts every pane through `start_pane`, which records each runtime, and returns the finished tree; a pane that fails never gets a runtime, so tree and runtimes match without `retain_runtimes`. `new` stays infallible, since every restore problem is a warning.
- **`start_pane` takes its ID.** `CreateTab` and `CreatePane` pass `Id::new()`, restore passes the saved one. No `Option` standing for "make one up".
- **The saver task gets its own root span**, `saver`, like `pane_events`. `spawn_traced` is for short work inside the caller's span; long-lived tasks get a root span (`ship_core::spawn_traced`'s doc).
- **The final flush runs inside the graceful-shutdown future**, right after `stop_actors`, so it's covered by the existing 5-second bound without a second timeout.
- **`--no-restore` reaches `serve` as `restore: bool`, next to the path.** Whether to read the file is the caller's decision, so `lib.rs` skips `load` instead of `load` checking a flag riding on a path type.
- **`main.rs` creates `tabs/` explicitly**, through a `runlog::create_private_dir` function extracted from `RunLog::start`, then passes `tabs/<port>.json`. No accessor creates directories, and `runlog`'s `platform` module stays private.
- **Two constants for two meanings.** `DEADLINE` is FR-005's 5 seconds and `RETRY` is decision 11's delay after a failed write. They're equal today, not by definition.

Estimated change: about 205 lines of implementation Rust, unmeasured. No test code (`docs/agents/code.md`).

| Area | Lines |
| --- | --- |
| `saver.rs`: load, check, project, `Saver` | about +125 |
| `state.rs`: `new` with restore, `start_pane` ID | about +35 |
| `lib.rs`: event subscription split, load, starter check, saver spawn and await | about +20 |
| `main.rs`, `cli.rs`, `runlog.rs`: flag, help, directory | about +25 |
| `ship-core`: `PartialEq` derives | 0 (five derive lines change) |

## Skeleton map

### Proposed settings

`tempfile` becomes a dependency of `ship-server`. It is already a workspace dependency.

```diff
 # crates/ship-server/Cargo.toml
 serde_json.workspace = true
+tempfile.workspace = true
 tokio.workspace = true
```

`ship-core` derives `PartialEq` on the tree types that lack it (`Tab`, `Pane` in `model.rs`; `Layout`, `Split` in `layout.rs`; `TabGeometry` in `geometry.rs`):

```diff
-#[derive(Clone, Debug, Serialize, Deserialize, utoipa::ToSchema)]
+#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, utoipa::ToSchema)]
 pub struct Tab {
```

No new configuration keys. The saved file's shape is `Tabs` as `ship tab list` prints it, with the projection's placeholders:

```json
{
  "tab:0193…": {
    "id": "tab:0193…",
    "name": "work",
    "tabs": {},
    "layout": {
      "kind": "split",
      "axis": "horizontal",
      "ratio": 0.62,
      "first": { "kind": "pane", "id": "pane:7f2c…", "name": null, "command": [], "cwd": "/Users/cyan/projects/ship", "rightClickPassthrough": false, "status": { "state": "running" } },
      "second": { "kind": "pane", "id": "pane:41ab…", "name": null, "command": [], "cwd": "/Users/cyan", "rightClickPassthrough": true, "status": { "state": "running" } }
    },
    "zoomed": "pane:7f2c…"
  }
}
```

### Added

#### `crates/ship-server/src/saver.rs` (slices 1 and 2)

```rust
//! The saved tabs: read once at startup, written as they change. Nothing else
//! reads or writes the file.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use ship_core::{
    model::{Pane, Tab},
    prelude::*,
    protocol::Replica,
    tree::Tabs,
};
use ship_macros::traced;
use tokio::sync::watch;

/// The longest a change waits before it is written (FR-005).
const DEADLINE: Duration = Duration::from_secs(5);
/// The wait after a failed write before trying again (decision 11).
const RETRY: Duration = Duration::from_secs(5);

// Slice 1

/// The tabs to restore, else empty. Never fails startup. A missing file is a
/// first run (info). An unreadable, unparsable or `check`-failing file is a
/// warning naming the file and its `.prev` path, and is left in place (FR-013).
#[traced]
pub(crate) fn load(path: &Path) -> Tabs;

/// What a hand-edited file can break and no command can (decision 9): a tab
/// or pane ID used twice anywhere in the tree, a map key that isn't its tab's
/// `id`, a zoom on a pane outside its tab. Sibling keys repeated in one JSON
/// object never reach here (G3).
fn check(tabs: &Tabs) -> Result<()>;

/// The saved form of a published tree (decision 4): per pane `cwd` = `dir`
/// else `cwd`; `dir`, `title` cleared; `command` empty; `status` running; per
/// tab `geometry` cleared. Recursive over child tabs.
pub(crate) fn project(tabs: &Tabs) -> Tabs;

fn project_tab(tab: &Tab) -> Tab;
fn project_pane(pane: &Pane) -> Pane;

/// `<port>.json.prev` beside `path`, the one kept copy (FR-009).
fn prev(path: &Path) -> PathBuf;

// Slice 2

/// The saver task's state.
pub(crate) struct Saver {
    path: PathBuf,
    /// The projection last written, else the restored one, so a server that
    /// changes nothing never writes (decision 5, FR-007).
    written: Tabs,
    /// Whether this run has copied the old file to `.prev` (FR-009).
    kept_prev: bool,
}

impl Saver {
    pub(crate) fn new(path: PathBuf, baseline: Tabs) -> Self;

    /// On a replica whose projection differs from `written`, sets the deadline
    /// if unset (never pushed back, decision 5); on one that matches again,
    /// clears it. At the deadline, saves; on failure logs an error and sets the
    /// deadline `RETRY` out. When the sender drops (decision 6), saves once
    /// more if a deadline is set, then returns.
    pub(crate) async fn run(mut self, replicas: watch::Receiver<Arc<Replica>>);

    /// One save: pretty JSON, then on `spawn_blocking` the `.prev` copy if not
    /// yet kept and a file exists (parsable or not), then `replace`. Updates
    /// `written` and `kept_prev` only on success. `no_err`, because `run` logs
    /// a failure as an error (decision 11) and `#[traced]` would also warn.
    #[traced(no_err)]
    async fn save(&mut self, tabs: Tabs) -> Result<()>;
}

/// A `tempfile` in `target`'s directory (mode 0600 on Unix, FR-003),
/// `sync_all`, then `persist` over `target` (FR-008).
fn replace(target: &Path, bytes: &[u8]) -> Result<()>;
```

### Changed

#### `crates/ship-server/src/state.rs` (slice 1)

```diff
 impl ServerState {
-    pub(crate) fn new(
-        bus: ActorRef<RelayBus>,
-        live: watch::Sender<LivePanes>,
-        pane_env: PaneEnv,
-    ) -> Self
+    /// An empty state whose tree is then set to `restore(saved)` (decision 2).
+    /// Restored panes start here, before the actor runs; nothing is published,
+    /// since the replica watch is seeded from the result.
+    pub(crate) fn new(
+        bus: ActorRef<RelayBus>,
+        live: watch::Sender<LivePanes>,
+        pane_env: PaneEnv,
+        saved: Tabs,
+    ) -> Self

+    /// `saved` with every pane started under its saved ID and directory.
+    /// A `Validation` error from `start_pane` warns with the pane and
+    /// directory and retries in home (FR-012). Any other failure warns and
+    /// leaves the pane out with `layout::close`, putting the tab's zoom back
+    /// if the zoomed pane is still there (FR-015).
+    #[traced]
+    fn restore(&mut self, saved: Tabs) -> Tabs
+    fn restore_tab(&mut self, tab: Tab) -> Tab

-    fn start_pane(&mut self, size: Size, input: PaneInput) -> Result<Pane>
+    fn start_pane(
+        &mut self,
+        id: IdOf<Pane>,
+        size: Size,
+        input: PaneInput,
+    ) -> Result<Pane>
 }
```

The two existing callers, `CreateTab` and `CreatePane`, pass `Id::new()`, which `start_pane` calls today. Restore builds each pane's input from the saved pane:

```rust
PaneInput {
    name: saved.name,
    right_click_passthrough: saved.right_click_passthrough,
    spec: PaneSpec { command: None, cwd: Some(saved.cwd) }, // None on the home retry
}
```

#### `crates/ship-server/src/lib.rs` (slices 1 and 2)

```diff
 mod routes;
+mod saver;
 mod settings;

 pub async fn serve(
     address: SocketAddr,
     starter: bool,
     config: PathBuf,
+    save: PathBuf,
+    restore: bool,
     shutdown: impl Future<Output = Result<()>> + Send + 'static,
     log: impl Fn(Option<LogChange>) -> Result<LogFilter> + Send + Sync + 'static,
 ) -> Result<()> {
     …bind (decision 13: before load)…
     let bus = relay::RelayBus::spawn_with_mailbox(…);
+    let events = subscribe_pane_events(&bus).await?;             // slice 1, decision 3
+    let saved = if restore {
+        saver::load(&save)
+    } else {
+        tracing::info!("not restoring, --no-restore");
+        Tabs::new()
+    };
     let (live_tx, live) = watch::channel(pane::LivePanes::new());
-    let state = state::ServerState::new(bus.clone(), live_tx, pane::PaneEnv::new(address, config)?);
+    let state = state::ServerState::new(
+        bus.clone(),
+        live_tx,
+        pane::PaneEnv::new(address, config)?,
+        saved,
+    );
     let (replica_tx, replicas) = watch::channel(state.replica());
+    let baseline = saver::project(&replicas.borrow().tabs);      // decision 5
+    let restored = !baseline.is_empty();
     …subscribe replica_tx…
     let state = state::ServerState::spawn(state);
-    forward_pane_events(&bus, state.clone()).await?;
+    forward_pane_events(events, state.clone());
     …
-    if starter && let Err(error) = state.ask(starter_tab).await {
+    if starter && !restored && let Err(error) = state.ask(starter_tab).await {   // FR-019
         …
     }
+    let saver = tokio::spawn(                                     // slice 2
+        saver::Saver::new(save, baseline)
+            .run(replicas.clone())
+            .instrument(tracing::info_span!(parent: None, "saver")),
+    );
     …
         .with_graceful_shutdown(async move {
             …
             stop_actors(&state, &bus).await;
+            saver.await.ok();                                     // final flush, decision 6
         })
 }

-async fn forward_pane_events(bus: &ActorRef<relay::RelayBus>, state: ActorRef<state::ServerState>) -> Result<()>
+/// Subscribed before the state is built, so restored panes' first events wait
+/// in the channel until forwarding starts.
+async fn subscribe_pane_events(
+    bus: &ActorRef<relay::RelayBus>,
+) -> Result<mpsc::UnboundedReceiver<pane::PaneEvent>>
+fn forward_pane_events(
+    events: mpsc::UnboundedReceiver<pane::PaneEvent>,
+    state: ActorRef<state::ServerState>,
+)
```

In slice 1, before the saver exists, `baseline` is only used for `restored`.

The doc comment on `serve` gains "restores the saved tabs" before "creates the starter tab", and "only when nothing was restored".

#### `crates/ship/src/cli.rs` (slice 1)

```diff
 pub struct ServerArgs {
     …
-    /// Start with one tab holding a shell in your home directory
+    /// Start with one tab holding a shell in your home directory, only when
+    /// there are no tabs after restoring
     #[arg(long)]
     pub starter: bool,
+    /// Start without restoring this port's saved tabs. The first save
+    /// replaces them and keeps the old file beside it as <port>.json.prev
+    #[arg(long)]
+    pub no_restore: bool,
     #[arg(long, hide = true)]
     pub background_child: bool,
 }
```

#### `crates/ship/src/main.rs` (slice 1)

```diff
         Some(Command::Server(args)) => {
+            let tabs = runlog::state_dir()?.join("tabs");
+            runlog::create_private_dir(&tabs)?;
             ship_server::serve(
                 ship_server::loopback_addr(args.port)?,
                 args.starter,
                 cli.config_path()?,
+                tabs.join(format!("{}.json", args.port)),
+                !args.no_restore,
                 diagnostics::shutdown()?,
                 diagnostics.server_log(),
             )
             .await
         }
         …
         None => {
             let client = client(&cli.server_url)?;
-            // A full-screen client must not misread another protocol. Only a
-            // server this call launched holds a starter tab to open.
+            // A full-screen client must not misread another protocol. Only a
+            // server this call launched opens on its first tab, restored or
+            // the starter (FR-020).
```

#### `crates/ship/src/runlog.rs` (slice 1; the file comes from diagnostics)

```diff
-pub fn logs_dir() -> Result<PathBuf> {
+/// `<state>/ship`. XDG state paths on macOS too, not Application Support.
+pub fn state_dir() -> Result<PathBuf> {
     …
-    path::absolute(directory.join("ship/logs"))
-        .map_err(|error| err!(Io, "cannot resolve log directory", @external: error))
+    path::absolute(directory.join("ship"))
+        .map_err(|error| err!(Io, "cannot resolve the state directory", @external: error))
 }
+
+pub fn logs_dir() -> Result<PathBuf> {
+    state_dir().map(|directory| directory.join("logs"))
+}
+
+/// `dir` and its parents, mode 0700 on Unix.
+pub fn create_private_dir(dir: &Path) -> Result<()>

 impl RunLog {
     pub fn start(dir: &Path, stem: String) -> Result<Self> {
-        let mut builder = fs::DirBuilder::new();
-        builder.recursive(true);
-        platform::private_dir(&mut builder);
-        builder.create(dir)
-            .map_err(|error| err!(Io, "cannot create private log directory {}; no file was created", dir.display(), @external: error))?;
+        create_private_dir(dir).context("no log file was created")?;
```

`create_private_dir`'s error names the directory; `RunLog::start` keeps its "no file was created" detail through `.context`.

### Moved

None.

### Replaced

- `forward_pane_events(bus, state)` in `crates/ship-server/src/lib.rs` becomes `subscribe_pane_events(bus)` plus `forward_pane_events(events, state)` (slice 1).
- The directory creation in `RunLog::start` moves into `runlog::create_private_dir` (slice 1).

### Untouched

- `crates/ship/src/local.rs`: the background launcher passes `--starter` and never `--no-restore`, so bare `ship` and tab and pane commands always restore (FR-014) and get a starter only when nothing was restored (FR-019).
- `ship-client`: `open_first` already selects the first top-level tab's first pane (FR-020), and a reconnecting client already resends its selection, which restored IDs keep valid (P1).
- `crates/ship-server/src/pane.rs`: `Launch` already takes the pane ID.
- `ship-core` beyond the five derives: no new types. `Tabs`, `Tab`, `Pane` and `Layout` serialize and deserialize as they do for `ship tab list`. `PROTOCOL_VERSION` is unchanged; nothing on the wire changes.
- `ServerState::commit`, routes, attach streams, the PaneEvent handler and `stop_actors`.

## Build order

1. **Restore from a file.** Files: `saver.rs` (`load`, `check`, `project`, `prev`), `state.rs`, `lib.rs` (event split, load, starter check), `cli.rs`, `main.rs`, `runlog.rs`, and the `ship-core` derives (`model.rs`, `layout.rs`, `geometry.rs`). Delivers FR-004, FR-010 to FR-015, FR-016's restore half, FR-019, FR-020 and P4. No saving yet, so the file is written by hand: `ship tab list` output is a valid saved file, because load ignores the live fields. Checks, on a spare port: build a tree with nested tabs, a three-way split with custom ratios, a zoom, names and passthrough on one pane; `ship tab list > <state>/ship/tabs/<port>.json`; stop and start `ship server --port <port>`; `ship tab list` matches except command, title, status and geometry, and each pane's `cwd` is its old directory (SC-001 by hand). An open client reconnects to the same pane. A deleted directory starts in home and the log names the pane and directory. A pane whose shell can't start (a configured shell that doesn't exist) is left out with a warning, and its tab's zoom survives if the zoomed pane did. Garbage, a duplicated pane ID and a zoom on a foreign pane each start empty with a warning naming the file and `.prev`, and leave the file untouched. `--no-restore` starts empty. `--starter` adds a tab only when nothing was restored, including the empty-file and `--no-restore` cases. Bare `ship` opens on the first restored tab's first pane. `ship server --help` shows both help texts. Without a file, every existing workflow behaves as before.
2. **Save as things change.** Files: `saver.rs` (`Saver`, `replace`), `lib.rs` (spawn, await), `crates/ship-server/Cargo.toml`. Delivers FR-001 to FR-003, FR-005, FR-007 to FR-009, FR-016 and P2, P3's first two stories as far as they're observable before #16, and P5. Checks: a split, a rename, a passthrough toggle and a `cd` each reach the file within 5 seconds, and a stream of `cd`s every second still saves within 5 seconds. SIGKILL 5 seconds after a change restores it, 20 out of 20 (SC-002). Killing the server during saves in a loop of 200 or more iterations never leaves a file the next start rejects (SC-003). Start and stop with no change leaves the file's mtime alone (FR-007); a change then `ship server stop` within 5 seconds is saved. The first save of a run writes `.prev` with the old bytes, including a garbage file's, and later saves leave `.prev` alone. The file is `0600` and `tabs/` is `0700` (`stat`). `grep` finds no pane arguments in the file after running `sleep 1234` in a pane (SC-006). A read-only `tabs/` logs an error every 5 seconds and the server keeps working. Ten restarts through bare `ship` keep the top-level tab count (SC-007).
3. **Docs, specs and size.** Files: `docs/guide.md` (saved tabs, `--no-restore`, `.prev` recovery, where warnings go), `README.md` "What works today", `GLOSSARY.md` "Saved tabs" (FR-018), the product brief's milestone 2 (FR-017), this change's spec deltas and `proposal.md`/`tasks.md`, and a `design.md` pointing at the three papers. When the PR opens, record the exit-window requirement on #16 (decision 12) and file issues for anything left open. Checks: `openspec validate session-restore`, every link resolves, the guide's commands run as written. Cyan's feel check for SC-005 with 10 tabs and 30 panes. Report implementation Rust and tests separately against today's count.

## Deviation log

Empty.
