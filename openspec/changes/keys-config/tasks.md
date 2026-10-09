# Tasks

Checkboxes record completed implementation checkpoints. Writing this file does not authorize source changes. Follow the locked papers indexed by design.md; the program paper's skeleton map and build order are authoritative for files and interfaces. Under an implementation request, finish and verify one slice, record actual evidence and any surprise in the program paper's Deviation log, then stop; do not begin the next slice in the same run. A surprise that contradicts a locked paper reopens it under the design-rules ripple rule before work continues. Unavailable exercises stay explicitly unverified.

Every slice runs workflows against a foreground `ship server --port <p>` with `--server-url http://127.0.0.1:<p>` on each client unless a task says otherwise. Every slice ends with `cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D warnings`, and debug and release builds passing, and adds no permanent tests. Slices that touch routes or protocol types rerun a disposable `openapi()` consumer outside the repo. Every slice runs on macOS. Slices 2 to 4 also run on Linux or are recorded as unverified there. Windows is unverified throughout. Probes and scratch config files live outside the repo.

## 1. One command tree: CLI commands to the same results through `execute`

- [x] 1.1 Add `ship-core/src/command.rs` with `Command`, the `tab` and `pane` structs, `PaneTarget`, `TabTarget`, `Destination` (moved from `cli.rs`, with `resolve`) and `Direction`, and add the `clap` feature to `ship-core/Cargo.toml`. In `protocol.rs`, give `CreateTab` `at: MoveTab` and `starter: Option<Starter>`, add `Starter` and `MoveTab::top`, and set `PROTOCOL_VERSION` to 6. Verify `cargo build -p ship-core` succeeds both with and without `--features clap`.
- [x] 1.2 In `ship-server`, make the `CreateTab` handler start a shell pane for `starter: Some(Shell)` and insert the tab with `tree::place`. Delete the `Starter` message and send `CreateTab` from `serve` for `--starter`. Update the `create_tab` route docs. Verify with `curl` that `POST /api/v0/tabs` with `{"at":{"after":"<id>"},"starter":"shell"}` places a tab holding one pane after `<id>`, that `{}` appends to the top level, and that `ship server --starter` still lists one tab with one pane.
- [x] 1.3 In `ship-client`, add `execute.rs` with `Scope`, `Outcome`, `Client::execute` and the target helpers, move `current_dir` there, and make `api::create_tab` take a `&CreateTab`. Verify the workspace builds.
- [x] 1.4 In `ship`, delete the argument structs from `cli.rs`, flatten `ship_core::command::Command` into `Command`, enable the `clap` feature, and replace `commands::tab` and `pane` with `commands::action`, which builds `Scope::Cli` from `SHIP_PANE_ID`. Verify `ship --help` lists `tab close` and `pane close` and no `rm`, and that `ship tab rm` exits 2.
- [x] 1.5 Verify slice 1 end to end, recording evidence:
  - Inside a Ship pane, `ship pane close` closes that pane and `ship tab close` closes its tab.
  - Outside a pane, `ship pane close` fails with the "pass `--pane` or run inside a pane" message and exits 1.
  - `ship tab create --starter --after <id>` puts a tab with a shell after `<id>`.
  - `ship pane create --tab <tab> --cwd /var -- sh -c 'sleep 100'` works as before.
  - `ship pane resize` with a direction reports "not available yet".
  - `ship tab move --tab <id> <parent> --before <sib>` exits 2.
  - A protocol-5 client fails health.
  - The OpenAPI consumer shows `CreateTab` with `at` and `starter`.
  - fmt, Clippy and both builds pass, with and without the `clap` feature.
  - Stop.

## 2. Config path and server settings: a `[server] shell` line to the shell new panes run

- [x] 2.1 Add `etcetera` and `figment` to the workspace and the `ship`/`ship-server` manifests. Add the global `--config` (with `env = "SHIP_CONFIG"`) and `Cli::config_path` to `cli.rs`, and pass the path to `serve`, `ui::run` and `local::default_health`, which forwards `--config` to the background server. Verify `ship --help` shows `--config` and `[env: SHIP_CONFIG]`.
- [x] 2.2 Add `ship-server/src/settings.rs` with `ServerSettings::load`, give `PaneEnv` the config path, and in `pane::spawn` set `SHELL` on the default-program builder from the setting, logging and falling back when the file is bad. Verify that `shell = "/bin/sh"` makes `ship pane create --tab <t>` run `/bin/sh` with `-sh` in `ps`, and that a relative `shell` resolves from the file's folder.
- [x] 2.3 Add `ship config check [FILE]` (server half), printing each error with its location and exiting 1 on any. Verify that a bad `shell` value (`shell = 3`) is named as `server.shell` with exit 1, and that a valid file exits 0 with no output.
- [x] 2.4 Verify slice 2 end to end, recording evidence:
  - Saving a new `shell` applies to the next pane and leaves running panes alone.
  - A syntax error gives the login shell and a warning in the server log.
  - With no server running, `SHIP_CONFIG=/elsewhere.toml ship` starts a server that reads `/elsewhere.toml`.
  - `ship server --starter` uses the configured shell.
  - Repeat on Linux or record it as unverified.
  - fmt, Clippy and both builds pass.
  - Stop.

## 3. Keymap and modes: Alt chords to actions, with `C-b` gone

- [x] 3.1 Add `crokey` and `figment` to `ship-client`. Add `keymap.rs` with `Keymap`, `Mode`, `ModeKind`, `ModeName`, `Binding` (a list of `Action`s, A-4), `ClientAction` (argument-free actions as empty struct variants), `Root`/`RawClient`/`RawMode` and the `Defaults` provider, which wraps each default binding in a list. Add `keymap/defaults.toml` from the product Interface, with `alt-1` to `alt-9` written out. `Keymap::load` collects every error with its key path, built from figment's key path rather than its `default.`-prefixed message. Verify `Keymap::defaults()` loads, and that a disposable probe outside the repo shows `"alt-n"` plus `"Alt-N"`, `"ctrl-alt-x"` plus `"alt-ctrl-x"`, an undefined mode, `kind` on `normal`, a `[srever]` table and a typo'd action each failing with its key path.
- [x] 3.2 Rewrite `ui/keys.rs` as the mode machine over `Keymap`, using `KeyCombination::from(KeyEvent)`, and implement `client.send` (check whether crokey converts a `KeyCombination` back to a `KeyEvent`, else build one from the single code and modifiers). Verify that a probe sequence through `Keys::handle` covers normal pass-through, sticky swallow, one-shot exit on bound and unbound keys, and a one-shot binding that enters another mode.
- [x] 3.3 In `ui/mod.rs`, load the keymap in `run` (defaults plus status-line error on failure), add the command queue running `execute` with `Scope::Keys`, select created tabs and starter panes through `chosen`, dispatch client actions with "not available yet" for the layout ones, and replace `navigate`'s `Action` match with `Step`. In `draw.rs`, add `Status` with the mode name and message, and change the empty-state hint to `alt-right to open one`. Update `long_about` to `alt-q detaches`. Verify the workspace builds and `rg 'C-b' crates` finds nothing.
- [x] 3.4 Add the client half of `ship config check`, which runs `Keymap::load`. Verify a file mixing a typo'd action, an unknown field, an undefined mode and a duplicate chord reports all four and exits 1, and a file with an unentered mode and no errors exits 0 silently.
- [ ] 3.5 Verify slice 3 end to end, recording evidence:
  - With no config, run `ship` and exercise every day-one default chord, checking each with `ship tab list`: `alt-n` makes a tab with a shell after the selected one, `alt-|` adds a pane to the selected tab, `alt-x` and `alt-shift-x` close, `alt-left`/`alt-right` and `alt-tab` move, and `alt-q` detaches.
  - `alt-b` shows "not available yet".
  - `ctrl-b` reaches `cat -v` as `^B`.
  - A user-defined sticky mode shows its name, swallows unbound keys and leaves on `esc`.
  - A `prefix` bound to `ctrl-b` with `c` creates a tab and returns to normal, and `client.send = "ctrl-b"` sends one `^B`.
  - `"alt-q" = "none"` keeps the client attached and passes the key through.
  - `clear_defaults` on `resize` empties it.
  - A rebound `alt-n` (`server.tab.create = {}`) makes an empty tab, and `"Alt-N"` in the file overrides the default `alt-n` without an error (A-4).
  - A list binding of a starter tab and a pane makes a tab with two panes, and a list whose first action fails runs nothing after it (A-4).
  - `alt-shift-x` with only a tab selected reports and changes nothing.
  - A bad file at startup runs the defaults and shows the error.
  - A binding and its CLI command give the same result on the same target.
  - Judge key latency by feel against slice 2.
  - Repeat on Linux or record it as unverified.
  - fmt, Clippy and both builds pass.
  - Stop.

## 4. Reload on save: an edited file to new keys in a running client

- [ ] 4.1 Add `notify-debouncer-mini` to `ship-client` and `watch.rs` with `changes(path)`, which watches the file's directory and, for a symlink, its target's directory too. Wire it into `drive` as a `select!` arm, and make `client.config.reload` take the same path, keeping the running keymap and showing the error on failure. Verify the workspace builds.
- [ ] 4.2 Verify slice 4 end to end, recording evidence:
  - With a client attached, adding `"alt-y" = { server.tab.create = {} }` and saving makes `alt-y` work while `alt-n` still does.
  - The same works with vim's rename-save (`:set backupcopy=no`) and with the config path a symlink into another directory.
  - A broken save keeps the old keys and shows the error.
  - A key bound to `client.config.reload` reloads.
  - Removing the active user mode on reload drops the client back to normal.
  - Repeat on Linux or record it as unverified.
  - fmt, Clippy and both builds pass.
  - Stop.

## 5. Docs and the size report: a README that matches, and recorded evidence

- [ ] 5.1 Update `README.md`: the Alt defaults replace the `C-b` keys, `close` replaces `rm`, `--tab`/`--pane` with the in-pane defaults, and a config file section covering location, `[server] shell`, a rebinding example, user modes and `ship config check`, linking crokey for chord spelling. Verify `rg 'C-b|tab rm|pane rm' README.md crates` finds nothing and the README's commands and config example run as written.
- [ ] 5.2 Verify the change end to end:
  - Report implementation Rust before and after, and test lines (zero), separately against the program paper's estimate.
  - Record which checks ran on Linux and macOS, with Windows unverified.
  - Confirm no temporary probes, disposable consumers or scratch config files remain in the repo.
  - Record evidence and stop at the completed change.
