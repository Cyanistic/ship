# Session restore architecture

Status: Locked on 2026-10-16, approved by Cyan after Plannotator review ("LGTM"). The approval doesn't authorize implementation; Cyan requests that separately. Written on 2026-10-15 from the locked [product paper](product.md) and a design conversation with Cyan, against the crates at `fc2b5a7`. Cyan picked shape A in chat ("i think i agree with the A though for the task thing"), moved restore from a message into the state actor's constructor ("isn't the smarter design to populate the start args of the actor instead of sending a restore message?"), and had restore go through the same path that enforces the tree's invariants for every edit ("could we make it go through somewhere that enforces the invariants all the time"; "that sounds right to me!"). Diagrams are text and diffs, not Mermaid, at Cyan's request.

Amended on 2026-10-16 during program review, and accepted by Cyan without a second review ("yes please. go ahead and do that for me. i don't have to check architecture.md again."): decision 2 no longer splits `apply` out of `commit`. In a brand-new state, `apply` would have no viewers to repair and no runtimes to drop, and restore can't fail, so the constructor sets the restored tree directly and stays infallible. The invariants Cyan asked restore to keep are still enforced: `load`'s check rejects what a hand-edited file can break (decision 9), and restore removes failed panes with `layout::close`, the same call commands use (decision 8). `start_pane` now takes its ID from the caller instead of an optional one, and `--no-restore` reaches `serve()` as its own flag next to the save path.

Amended again on 2026-10-16 during slice 1, at Cyan's direction ("if a user does that, and the program itself doesn't, why should i give a shit?"; then "let's go for that!"): decision 9 no longer checks the file's structure. A file that parses is restored as written, the way `commit` trusts a command's edit. Hand-edited duplicates or a foreign zoom are the editor's problem, the same as deleting their own files. Decision 8 now resolves the saved directory through `start_pane`'s own `pane_cwd` before starting, since `start_pane`'s `Validation` error can't tell a bad directory from a shell that won't start. The loaded and projected tree is a `Saved` newtype, so only `load` and `project` make one and only `ServerState::new` restores from one.

Timing clarified during slice 3 on 2026-10-16 after Cyan accepted keeping the planned five-second trigger. Decision 5 schedules the write; serialization and disk I/O take additional time. The exact-five-second SIGKILL probe exposed a wording mismatch, not evidence of a non-atomic write. See the program paper's deviation log.

## Summary

A new saver task watches the replica the server already publishes after every change. It strips each replica down to the saved tabs, and schedules a save 5 seconds after the first unsaved change, plus a final flush when the server stops. Serialization and disk I/O add time after the deadline. Restore happens when the state actor is built: `serve()` reads and parses the file, and `ServerState::new` builds the state with the restored tree, starting each saved pane with its saved ID before the actor ever runs. Restore can't happen into an existing tree. The saved file is Ship's existing `Tabs` type serialized as JSON with the live fields blanked. The 60-second exit window (FR-006) is left to #16, because today a program exiting doesn't change anything that gets saved.

About 200 lines of implementation Rust. Depends on the diagnostics change for the state directory lookup and the log path.

## Context and constraints

- **One commit point.** Every change to the tree goes through `ServerState::commit` (`crates/ship-server/src/state.rs:97`). It runs the edit on a copy, repairs each viewer's selection, swaps the copy in, drops the running program of every pane no longer in the tree (`retain_runtimes`), bumps the revision, and publishes an `Arc<Replica>` on the `RelayBus`. It trusts each edit to leave the tree itself valid, such as unique IDs and a zoomed pane inside its tab. `serve()` already subscribes a `watch` channel to it for attach streams (`crates/ship-server/src/lib.rs:175`).
- **The bus keeps nothing.** `Publish` clones into the sinks subscribed at that moment and drops the value when there are none (`crates/ship-core/src/relay.rs:45`).
- **Pane events subscribe after spawn today.** `forward_pane_events` subscribes an unbounded channel to the bus after `ServerState::spawn`, then forwards into the actor (`crates/ship-server/src/lib.rs:234`).
- **The directory is already tracked.** `Pane::dir` holds the OSC 7 directory, else the process working directory on Linux and macOS. `Pane::cwd` holds the starting directory (`crates/ship-core/src/model.rs`).
- **Program exits only set `status`.** `PaneChange::Exited` writes `pane.status` (`crates/ship-server/src/state.rs:880`). `status` isn't saved (FR-002), so no exit changes the saved tabs until #16.
- **IDs are random per creation.** `start_pane` calls `Id::new()`, a UUID v4 (`crates/ship-server/src/state.rs:262`).
- **Reconnects resend the selection.** A reconnecting client sends its last selection (`crates/ship-client/src/ui/observer.rs:86`), and `Attach` keeps it if the node exists. Keeping IDs is enough for P1's reconnect story.
- **The starter runs before accept.** `serve()` binds, spawns the actors, asks for the starter tab, then serves. A health probe waits in the listen backlog meanwhile (`crates/ship-server/src/lib.rs:183`).
- **Stopping is ordered.** `stop_actors` stops the state actor, then the bus. Dropping the bus drops the replica `watch` sender, which ends every stream (`crates/ship-server/src/lib.rs:278`).
- **Diagnostics owns the state directory.** The diagnostics worktree resolves `$XDG_STATE_HOME/ship` with `etcetera` (XDG on macOS too) in `crates/ship/src/runlog.rs`, and has `0700` and `0600` helpers. Not merged yet.
- **Dependencies on hand.** `serde_json`, `tempfile` and `tokio-util` are workspace dependencies. `tempfile` isn't yet a dependency of `ship-server`.

## Candidate shapes

### A: A saver task watches the replica (picked)

The saver gets its own clone of the replica `watch` receiver, like an attach stream. On each change it projects the replica to saved tabs and compares the result with the last projection it wrote. It writes when the deadline passes, and flushes when the stream ends. The state actor only gains constructor restore. Cost: the replica doesn't say why it changed, so #16 has to add a cause before exits can be held back 60 seconds. Failure mode: a write that fails is logged and retried, and the server keeps running.

```diff
 serve
   spawn RelayBus, ServerState
   subscribe replica → watch                  # attach streams read this
+  spawn saver(replicas.clone(), path)       # project, compare, 5 s deadline, write
```

### B: Saving inside the state actor

`commit` marks the state dirty and schedules a `SaveNow` message, and the actor does the `.prev` copy and the write itself. Every cause is known where it happens. Cost: the actor that owns the tree also owns timers and disk writes. Failure mode: a slow disk delays every queued command, keys and clicks included.

```diff
 ServerState.commit(edit)
   clone, edit, swap, publish
+  if project(tabs) != saved: schedule SaveNow in 5 s
+impl Message<SaveNow>: .prev copy, temp write, rename
```

### C: A, plus a cause on every commit

`commit` takes a `Cause`, about 15 call sites pass it, and the state actor publishes `SavedTabs { tabs, cause }` for a saver that keeps a separate 60-second window for exits. Cost: the exit branch can't run until #16, so it's untested code (`docs/agents/code.md`: verify with real workflows). Failure mode: the window is wrong in a way nobody notices until #16 lands.

```diff
-async fn commit(&mut self, edit)
+async fn commit(&mut self, cause: Cause, edit)
+  publish(SavedTabs { tabs: project(tabs), cause })
```

## Decision

Shape A, with restore as constructor input. The outcome that matters most is the smallest change that meets every requirement today. A adds one task and changes the state actor only at birth. B puts disk latency on the command path. C builds exit handling before anything can trigger it.

## Structure

```text
ship binary (crates/ship)
  cli         ship server --no-restore
  paths       <state>/ship/tabs/<port>.json          # shared state-dir lookup with diagnostics
     │ save path, restore flag
     ▼
ship-server
  serve()
    bind
    spawn RelayBus
    events = subscribe pane events                    # moved before the constructor
    saved  = saver::load(file)                        # unless --no-restore; parse; problems → warn!
    state  = ServerState::new(…, saved)               # restored tree set directly: saved IDs, failed panes closed
    replicas = watch(state.replica())
    spawn ServerState; forward(events, state)
    if starter and no tabs: CreateTab starter
    saver = spawn saver::run(replicas.clone(), file)
    accept … shutdown
    stop_actors(state, bus)                           # bus drop closes the replica watch
    await saver                                       # final flush happens on close
       │
       ▼
  <state>/ship/tabs/43179.json        written by the saver only
  <state>/ship/tabs/43179.json.prev   first write of each run
```

| Box | Owns |
| --- | --- |
| `cli` and paths in `crates/ship` | The `--no-restore` flag and its help text (FR-014). Resolving `<state>/ship/tabs/<port>.json` and creating `tabs/` with mode `0700`. Passing both to `serve()` the way the config path goes today. The background launcher never passes `--no-restore`. |
| `serve()` | Loading before the actor exists. Subscribing pane events before the constructor. The "starter only when there are no tabs" check (FR-019). Starting the saver and awaiting it after the actors stop. |
| `ServerState::new` | Building the state with the restored tree: starting each pane with its saved ID through `start_pane`, the directory fallback (FR-012), and leaving out panes whose shell fails (FR-015). No publish, since the replica `watch` is seeded from the constructed state. Infallible, since every restore problem is a warning. `commit` is unchanged. |
| `saver` (new module in `ship-server`) | `load`: missing file, unreadable file (FR-013), and parse. `project`: replica to saved tabs. `run`: comparing, the deadline, the `.prev` copy, atomic writes and the final flush. Nothing else reads or writes the file. |
| `start_pane` | Taking the pane's ID from its caller (a new one on creation, the saved one on restore), so restore and creation share one path. |

## Key decisions

1. **A saver task, not the state actor.** It consumes the replica `watch` that already exists, so what `commit` does for a command doesn't change. Attaches, selections, titles and status change the replica but not the projection, so they never cause a write. Rejected: B (disk writes on the command path) and C (an exit window nothing can exercise yet).

2. **Restore is constructor input.** `ServerState::new` takes the saved tabs, so a server is born restored and the code has no way to restore into a tree that already has tabs. The constructor starts every saved pane, which records its runtime, and sets the resulting tree, so tree and runtimes match by construction: a pane that fails to start never gets a runtime and is closed out of the tree. The first replica every stream sees is seeded from the constructed state, so nothing needs publishing. Rejected: a `Restore` message after spawn, which needs a rule for a non-empty tree and costs a commit; and, in an amendment, running the restore through an `apply` split out of `commit`, because in a new state it has no viewers to repair and no runtimes to drop, so the split would only forward.

3. **Pane events subscribe before the constructor.** Restored panes start inside `ServerState::new` and can publish an OSC 7 directory or an exit right away. The bus drops publications nobody is subscribed to, so `forward_pane_events` splits at its existing seam: subscribe the same unbounded sink first, start forwarding after spawn. The channel holds the early events meanwhile. Rejected: giving the bus replay (it is stateless on purpose), and starting restored panes after spawn (that brings back decision 2's message).

4. **The file is the existing `Tabs`, projected.** `project` clones the tree and, for each pane, sets `cwd` to `dir` else `cwd`, clears `dir`, `title` and `geometry`, empties `command` and sets `status` to `running`. Then `serde_json` writes it pretty-printed so it can be fixed by hand. Any field added to `Tab` or `Pane` later is saved without touching the saver, which is FR-002's default. The placeholders `"command": []` and `"status": {"state": "running"}` stay in the file and are ignored on load. Rejected: hand-written `SavedTab` and `SavedPane` mirror types, which can silently miss a new field, and making `Layout` generic over its leaf, which touches every layout call site for a file format.

5. **A deadline counted from the first unsaved change.** When a projection differs from the last one written and no deadline is set, the deadline becomes now plus 5 seconds. New changes never push it back, so a stream of directory changes still triggers the save at that deadline (FR-005). Serialization and disk I/O take additional time; this is not a guarantee that the write completes before an exact-five-second SIGKILL. The baseline at startup is the restored projection, so a server that restores and changes nothing never writes (FR-007). The comparison is against the last projection written, not the file. Rejected: a trailing debounce, which a busy shell could postpone forever.

6. **Final flush when the replica stream closes.** The saver's loop ends when the `watch` sender drops, which happens when `stop_actors` stops the bus after the state actor. By then the bus has delivered every queued commit. If a deadline is set, the saver writes the latest projection, and `serve()` awaits it inside the existing 5-second shutdown bound. This covers `ship server stop`, SIGTERM, SIGINT and an actor stopping on its own. Rejected: an explicit flush call between stopping the state actor and stopping the bus, which can run before the bus delivers the last commit.

7. **Atomic writes, and a `.prev` copy made the same way.** Each write goes to a `tempfile` in `tabs/`, which is mode `0600` on Unix (FR-003), then `sync_all`, then `persist` over the target (FR-008). On the first write of a run, if the target exists, its bytes are first copied to `.prev` through the same temp-and-rename (FR-009). Copying works whether or not the old file parses, which FR-013 needs. Rejected: renaming the old file to `.prev` before writing, because a crash between the two steps leaves no saved file.

8. **Restore problems are warnings, and restore never blocks startup.** A missing file is a first run and logs at info. An unreadable or unparsable file logs a warning naming the file and the `.prev` path, and the server starts empty (FR-013). Restore resolves each saved directory with `pane_cwd`, the function `start_pane` uses for its "is this a real directory" rule. If it rejects the directory, restore logs a warning naming the pane and directory and starts the pane in home (FR-012). Any `start_pane` failure leaves the pane out with `layout::close`, with a warning (FR-015). `close` clears the tab's zoom, so restore puts the zoom back if the zoomed pane survived. Rejected: a second copy of the directory rule in restore; and telling the two failures apart by `start_pane`'s error code, which is `Validation` for both a bad directory and a shell that won't start, and changing it would change what clients see. (Amended in slice 1 with decision 9.)

9. **The loader trusts the file like `commit` trusts a command** (amended). A file that parses is restored as written. A hand-edited file can break rules no command can, such as the same pane ID twice or a zoom on a pane outside its tab, and the result is whatever the tree then does: geometry and restore already ignore a foreign zoom, and a repeated pane ID shares one runtime. Only an unreadable or unparsable file is handled under FR-013. Rejected: checking the structure in `load` (the original decision), which guards against something only the user can do to their own file; repairing the file, which guesses at what it meant; and checking in `commit`, which costs every command.

10. **Starter after restore.** `serve()` checks whether the constructed state has any tabs before spawning, and asks for the starter only when it has none and `--starter` is set (FR-019). The client side needs no change: a client that launched the server already asks for the first tab's pane, restored or not (FR-020).

11. **Logging follows the diagnostics conventions.** `saver::load`, the restore loop in `ServerState::new` and each save are `#[traced]`. The warnings in decisions 8 and 9 are `warn!` inside those spans. A failed write is an error event: the server keeps running, the deadline stays set, and the write is retried 5 seconds later. Pane arguments are never logged, because `#[traced]` skips arguments by default.

12. **The exit window waits for #16.** `status` is projected out, so no current path produces an exit-only change to the saved tabs, and FR-006 and FR-007's "don't save a pending exit" already hold. #16 has to add a cause to commits and a 60-second hold in the saver before an exit can remove a pane. That requirement gets recorded on #16 when this change's PR is opened.

13. **One file per port, guarded by the bind.** `serve()` binds before it loads, so a second server on the same port fails to bind before it touches the file (FR-004).

## Risks and unknowns

- **Diagnostics has to land first.** The state-directory lookup and the log path live in the unmerged diagnostics change. If session restore is built first, it needs a copy of the 10-line lookup that diagnostics later replaces.
- **Projection cost on busy replicas.** Every commit costs one tree clone and one comparison. Commits come from structural edits, attaches, titles and directory changes, not from screen output, so 30 panes should cost nothing noticeable. Profile if typing in a prompt that sets the title every command ever feels slower.
- **Power loss.** `sync_all` covers the file's contents. The directory isn't synced after the rename, so a power cut right after a save could bring back the previous file. FR-008 covers SIGKILL and crashes, and this risk is accepted.
- **Ratios round-trip through `f32` JSON.** `serde_json` writes the shortest representation that reads back to the same `f32`, so proportions should restore exactly. Checked by SC-001's before-and-after comparison.
- **Windows is unverified.** `persist` replaces an existing file on Windows, and the `0600` and `0700` modes do nothing there. Best effort.
- **Restored shells start all at once.** 30 shells starting together in the constructor could make bare `ship` slow to show a screen (SC-005). If it feels slow, panes could start in parallel or in the background, which would revisit decision 2.
