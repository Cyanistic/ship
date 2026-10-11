# Slice 3: tab sidebar, navigation and client settings

Tasks 3.1–3.6 are implemented and verified on macOS. The release client passed 65 temporary workflow checks against owned servers. Linux and Windows are unverified. Slice 4 has not started.

## Implementation

- `crates/ship-client/src/keymap.rs`: `ClientSettings` owns `Keymap` and the fixed `sidebar_width` (default 26). Startup, reload and `ship config check` use its loading path. Mode chords use `IndexMap`; untouched defaults retain their order, followed by explicitly configured chords in file order, including replacements. Empty bindings still unbind. Each binding captures its first action's value-or-name hint while deserializing, then adds its source chord spelling during normalization. This avoids a parallel action-label match tree and preserves `esc` rather than crokey's displayed `Esc`.
- `Cargo.toml`, client manifest and Cargo.lock: `tui-tree-widget` 0.24.1 uses the same ratatui-core 0.1.2 and ratatui-widgets 0.3.2 already used by Ship. TOML's `preserve_order` feature carries file order into the chord map.
- `crates/ship-client/src/ui/sidebar.rs` (new): owns the folded-ID set, visibility and width. New tabs are expanded. Reconciliation prunes removed tabs and opens selected ancestors, excluding the selected row itself. The visible-row walk ignores sidebar visibility. Rendering rebuilds `TreeItem`s and `TreeState` from those folds and the selection, with `tabs`, depth indentation, `▾`/`▸` markers and a cyan/reversed selected row. The widget handles scrolling to the selected row; there is no separately stored sidebar cursor.
- `ui/mod.rs`: loads settings before the initial attach so the first reported area already excludes the configured width. Next/previous navigate visible rows and use existing `Memory::landing` for own-pane selection. Expand/collapse, toggle, terminal resize and save/explicit reload are wired into the existing action queue and view reporting. Bad reloads keep the running width and keymap. Folds survive hiding, reloading and reconnecting.
- `ui/draw.rs`: excludes the shown sidebar and bottom bar from pane drawing, with saturating area arithmetic. Ordinary status content stays first; non-normal-mode hints are appended and clipped by the existing one-line renderer.
- `keymap/defaults.toml`: removes only `alt-1..9`. An explicitly bound row-selection action remains unavailable. Resize hints work, but resize actions remain unavailable until slice 4.
- `crates/ship-core/src/config.rs` and core manifest: the existing shared config loader now wraps TOML deserialization with `serde_path_to_error` (already a transitive dependency). TOML alone gave the line/column but not the required `client.sidebar_width` path. The adapter preserves TOML's source excerpt and adds the key path. Server/client config ownership is unchanged.
- `README.md`: current keys, tree navigation, two-client behavior, fixed width/reload, hint order and a runnable three-level-tree recipe.

No server layout, protocol, route, zoom, swap, resize or directory behavior was added. Protocol remains 8. No permanent authored test code was added.

## Evidence

Temporary harness and logs: `/tmp/ship-slice3-C4Dt/`. `harness.py` launches only owned servers on random ports and full-screen clients in real PTYs. A TCP proxy logs client-to-server traffic and can temporarily cut connections for reconnect checks. pyte renders actual client terminal output. The original slice-2 harness was copied into this temporary directory, not into the repository.

### Main two-client workflow: 43/43

`check.py`, debug and final release runs; final log `check-release.log`:

- Two clients, 100x25 and 140x30, see the same three-level tree. Initial rows have the `tabs` title, correct depth indentation and unfolded markers; nothing is selected. The selected tab's row has the highlight.
- Default width 26 leaves a 74x24 tab area on the smaller client. Both clients initially land on the first own pane. One cycles to the second while the other independently remembers the first.
- `alt-g`, `j/k`, `h/l` and `esc` navigate and fold using the selection itself. An empty parent with a populated child shows its no-pane hint instead of the child's screen. Next skips folded descendants; next/previous wrap. Normal-mode Alt arrows use the same rows. Returning to a worked-in tab restores each client's own remembered pane.
- One client's folds do not change the other client's tree. Hiding/showing restores the same folds. Hidden-sidebar navigation visits the same rows.
- Creating and selecting a tab under a folded descendant from a bound server action expands its ancestors automatically. The creation request uses the existing explicit destination; no slice-5 child action was added.
- Hiding the smaller client's sidebar grows the tab from 74 to 100 columns. Showing it restores 74. Width save, rename-save and explicit reload apply 40, 30 and 35 columns respectively. An invalid type keeps the previous width and reports the file/key location. Resizing to 90x23 reports 64x22 after the default sidebar and bottom bar.
- `ship config check` accepts server shell configuration alongside client width, and rejects `sidebar_width = "wide"` with the file, `client.sidebar_width`, line 2, column 17 and the offending text.
- A pane running a raw two-byte reader receives `alt-1` as bytes `1b31`. An explicitly bound `client.tab.select` reports `tab select is not available yet`.
- Default hints are exactly `j next  k prev  h collapse  l expand  esc normal` and `h left  j down  k up  l right  esc normal`. A custom binding and a two-action list display the first action's label, in file order. At 55 columns the status reads `[tabs] deep › deep-shell  j next  k prev  h collapse`, cutting later hints. At 90 columns, mode, labels, `exited (3)` and the latest action error remain ahead of partial hints.
- Removing the selected ancestor repairs selection to nothing; previous from nothing opens the last row. The wire log contains no `collapsed` or `recent` state.

Actual program sizes from `stty size`, matched against `ship tab get` content rectangles:

| Case | Tab width | Selected split content | Observed stty rows/cols |
| --- | ---: | --- | --- |
| Smaller viewer hides sidebar | 100 | x=51, y=1, width=48, height=22 | `22 48` |
| Width reload to 40 | 60 | x=31, y=1, width=28, height=22 | `22 28` |

Rendered records include `initial.txt`, `selected-descendant.txt`, `width40.txt`, `resize-hints.txt` and `narrow-error.txt`.

### Additional workflow and README examples: 18/18

`extra.py`, debug and final release runs; final log `extra-release.log`:

- Runs the README's literal shell recipe with `ship` and `jq`, creates `project → review → checks`, renders it in two clients and executes its ancestor-close cleanup. At startup, width 40 in a 100x25 client produces a borderless 60x24 pane; `stty size` prints `24 60`.
- Replacements follow untouched defaults in file order, and an unbound default disappears from the hints. The replacement action executes. A bad save retains both the running bindings and folds. A fresh client with invalid width falls back to width 26 and reports the full key path.
- Shrinking a client to 10x2 reports zero columns and one row of pane area. A 0x0 terminal survives and publishes a 0x0 area. Enlarging to 120x25 restores a running pane at 94x24; `stty size` prints `24 94`. No new tiny-window policy was needed.
- Cutting only the owned proxy connections shows mode, tab/pane labels, exit status, reconnect state and latest error before clipped hints: `[tabs] w › a  exited (3)  disconnected, reconnecting  · tab select is not available yet  j next  k prev  h collapse  l`. Reconnection retains selection and mode. Rendered records: `startup40.txt` and `disconnected.txt`.

### Derived labels and scrolling: 4/4

`labels.py`, release run; log `labels.log`:

- Empty unnamed tabs display their 1-based sibling positions at two depths.
- An unnamed tab follows its first own pane's explicit label.
- Clearing that pane's name changes the row to its OSC title, `derived-title`, without changing the configured sidebar or the 74-column reported area.
- With 25 additional rows below the viewport, previous-wrap selects the last row and the widget scrolls it into view with the highlight. Record: `scrolled.txt`.

All harness servers and clients were shut down. Existing user servers were left untouched.

### Checks and size

Passed on this macOS host:

- `cargo build -p ship`
- `cargo build --release -p ship`
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo check -p ship-core`
- `cargo check -p ship-core --features clap`
- `openspec validate layout-sidebar --strict`
- `git diff --check`

Logs are in the temporary harness directory. Strict Clippy passes on the current repository state; the older slice-2 lint failure is not carried forward as an exception.

Measured project implementation Rust (all Rust under `crates/`, excluding test paths): HEAD 7,087 lines → 7,386 lines, +299. Authored test Rust: 0 lines before and after. Dependency/vendor implementation and tests are excluded from these project counts.

## Rendered-review follow-up: divider and header gap

Cyan requested a clear sidebar/pane separator and a blank row between `tabs` and the first tab, leaving directory-based labels for later. `Sidebar::draw` now renders a ratatui right-only `Block` inside the configured sidebar width, then the header and tree inside it. The tree begins after the header and one blank row. The divider ends above the existing status bar and disappears when the sidebar is hidden. Labels and pane-area reporting are unchanged.

`/tmp/ship-slice3-C4Dt/chrome.py`, debug client on macOS, passed four rendered PTY checks (`chrome.log`, `chrome.txt`): header gap and depth indentation; a continuous divider at column 25 with the default 26-column sidebar and unchanged 74-column pane area in a 100-column client; hiding removes the divider and restores 100 columns; tiny/zero terminal sizes survive and restore the divider on enlargement. Debug build, formatting, strict workspace Clippy and `git diff --check` passed. The earlier workflow records above describe the original slice checkpoint, before this spacing adjustment.

## Unverified

- Linux and Windows. No container, VM or remote platform check was attempted.
- A person looking at a GUI terminal. Appearance checks used pyte-rendered output from real client PTYs, not a GUI-terminal screenshot.

## Uncertain decisions

None for behavior or architecture. Marker glyphs and highlight colors are the rendering details explicitly left to implementation review by the locked plan.

```text
reused:   tui-tree-widget, ratatui clipping, ordered TOML/IndexMap, crokey normalization, the existing memory/action/view/reload paths, serde_path_to_error
new code: client settings, local folds/visible rows, sidebar rendering and first-action hint presentation; existing code had no sidebar or ordered hints
proof:    65 release workflow checks, matching stty/published geometry, literal README examples, strict Clippy, fmt, debug/release builds and core feature checks on macOS
```

## Task 3.7: status-line overlay (product A-3, architecture A-6, program P-3)

Verified on macOS only; Linux and Windows are unverified.

- `ui/draw.rs`: `STATUS_ROWS` is gone. `tab_area` subtracts only the shown sidebar. `status_line` returns `Option<Line>`, `None` in normal mode with a connected client, a running selected pane and no message. Otherwise it gives a mode-name pill (bold black on cyan), then `exited (n)`, `disconnected, reconnecting` and the first line of the latest error, then hints with keys in bold cyan and labels in gray, the whole row on ANSI black (Herdr's panel background, a step above a Nord terminal's base). A cell dump shows `000000` behind every overlay cell through column 99. Tab and pane labels are dropped, so `position` went with them. `draw` clears and draws the overlay last over the pane area's bottom row, and sets the terminal cursor only when it doesn't fall on that row.
- `ui/panes.rs`: `draw` returns the cursor's frame position with the cursor instead of setting it, so `draw.rs` can drop it under the overlay.
- `keymap.rs`: `Mode::hints` yields each `chord label` hint rather than a joined string, so keys and labels can be styled separately.
- `README.md`: describes the overlay instead of the bottom bar.

Sidebar styling, from the same review: Herdr's `terminal` palette roles. The divider is dark gray, the ` tabs` header is dark gray and bold, unselected rows are gray, and the selected row is bold on a dark-gray band with a one-column lead.

Probe: `/tmp/ship-overlay-YI4F/overlay.py` (copy of the slice 3 harness, owned server, pyte-rendered 100x25 client), 17/18 with the 18th a faulty assertion: pyte reports ANSI cyan as `00cdcd`, not `cyan`. A follow-up cell dump (`peek.py`) showed the pill as `000000` on `00cdcd`, bold.

- The tab area is 74x25 beside the sidebar, and `stty size` in the pane reads `25 74`.
- In normal mode, the last row shows pane output (`seq 30`) and the cursor on it is visible.
- `alt-r` shows ` RESIZE   h left  j down  k up  l right  esc normal` starting right of the divider at column 26. Columns 0 to 24 are blank, the tab geometry is unchanged and the cursor under the overlay is hidden. `esc` removes the overlay and shows the cursor again.
- `alt-g` shows ` TABS ` with `j next  k prev  h collapse  l expand  esc normal`. A bound `client.tab.select` shows `tab select is not available yet`, and `esc` clears it with the overlay.
- `exit 3` shows `exited (3)`. Cutting the proxy adds `disconnected, reconnecting`, and restoring it removes that again. Labels `work` and `›` never appear.
- With the sidebar hidden, the overlay starts at column 0.

`cargo build`, `cargo build --release`, `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all -- --check` pass. No permanent test code was added.

