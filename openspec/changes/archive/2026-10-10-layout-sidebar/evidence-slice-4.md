# Slice 4: zoom, swap and resize

Tasks 4.1–4.6 are implemented and verified on macOS. The release build passed 111 temporary workflow checks against owned servers, plus a drift probe and an OpenAPI/ordinary-client contract probe. Linux and Windows are unverified. Slice 5 has not started.

## Implementation

- `crates/ship-core/src/command.rs`: `pane zoom` (`--pane`; an internal, CLI-hidden `zoomed` for explicit unzoom) and `pane swap` (`--pane`, required `--other`, no direction form). `pane resize` help now says it moves an edge. `Direction` serializes and has a schema.
- `crates/ship-core/src/protocol.rs`: `ZoomPane { zoomed: Option<bool> }` (absent toggles, `false` unzooms the pane's tab, `true` zooms the pane), `SwapPane { other }` and `ResizePane { direction, amount }` (absent amount is one cell, G5).
- `crates/ship-server/src/routes.rs`, `lib.rs`: `POST /api/v0/panes/{id}/zoom|swap|resize`, each answering the pane's tab. Swap documents 404 for either missing pane and 422 for cross-tab, self or malformed requests.
- `crates/ship-server/src/state/layout.rs`: `swap` exchanges two leaves of one tab by ID and clears zoom. `resize` finds the deepest split on the requested axis with the pane on the needed side, falls back to the opposite edge at the tab boundary, and binary-searches the furthest move up to the amount that doesn't push any pane's content below 1x1 (G2) unless it was already there. It returns whether the edge moved.
- `crates/ship-server/src/state/geometry.rs`: `ratio_for` finds the ratio giving an exact first-child length under the existing `Fill`/overlap split, so cell steps don't drift.
- `crates/ship-server/src/state.rs`: zoom, swap and resize messages go through the existing `commit`. Zoom and resize commit nothing when nothing changes. They then answer with the published tab, and no revision is created. Selection repair now also moves a selection hidden by its tab's zoom onto the zoomed pane.
- `crates/ship-client/src/api.rs`, `execute.rs`: ordinary `zoom`, `swap` and `resize` methods and CLI/key execution; resize is no longer "not available yet".
- `crates/ship-client/src/ui/mod.rs`, `keymap.rs`, `keymap/defaults.toml`: any `client.pane.*` action on a zoomed tab first queues an explicit `{"zoomed":false}` for the zoomed pane, waits for that revision, then resolves on full geometry (G6). `client.pane.swap` resolves the neighbor with focus's search and sends only `{"other": id}`; with no neighbor it sends nothing. Defaults: `alt-f` zoom, `alt-shift-h/j/k/l` swap. Resize mode's existing `h/j/k/l` now work.
- `crates/ship-client/src/ui/sidebar.rs`: a zoomed tab's row ends in a cyan bold `Z`, tmux's zoom flag.
- `README.md`: keys table, a zoom/swap/resize section with a runnable CLI recipe, and the status paragraph.

Protocol stays 8. The new routes are additions, and the layout shape is unchanged. No permanent authored test code was added.

## Evidence

Temporary harness and logs: `/tmp/ship-slice4/`. `harness.py` (copied from the slice-3 overlay harness) launches owned servers on random ports and full-screen clients in real PTYs, rendered by pyte. A TCP proxy logs one client's requests. Final runs used the release binary (`*-release.log`).

### Zoom and swap: 49/49 (`zoomswap.py`)

- `alt-f` zooms for both viewers: the hidden pane disappears from both, the published geometry holds only the zoomed pane, and the sidebar row reads `work Z`, losing the `Z` on unzoom. The second viewer's hidden selection repairs to the zoomed pane, and its typing reaches it.
- A hidden program keeps running at its last size: `stty size` in hidden B kept printing `14 35` while zoomed A followed the full area.
- Selecting another tab leaves the zoom in place; returning lands on the zoomed pane. `ship pane zoom` toggles.
- Disconnecting and reconnecting a viewer selected on a hidden pane repairs it to the zoomed pane, with no zoom request on the wire.
- Focus and next out of zoom send `{"zoomed":false}` before moving, and both land on full geometry. Two clients pressing focus concurrently leave the tab unzoomed; neither toggles it back. Create and close unzoom.
- `ship pane swap` without `--other` exits 2. A nonadjacent diagonal pair exchanges positions and geometry; both programs keep running and their screens move with them. A pair resolved before a layout change still exchanges those IDs. A missing pane (404), a cross-tab pair and a self swap (422) leave layout and zoom unchanged.
- `alt-shift-*` sends `{"other": …}` only. On a fresh `[X | [Y / Z]]` tab, recency picks Z over Y. With no history the tie goes to Y in layout order. No neighbor sends no swap. A zoomed swap sends the unzoom, then the swap, in that order on the wire. A zoomed swap with no neighbor unzooms and sends no swap. No direction or recency field reaches the server.

### Resize: 22/22 (`resize.py`) and drift probe

- `ship pane resize --amount 5` moves L's right edge 5 cells. An omitted amount moves 1. On R (rightmost) `right 3` moves its left edge right, and on L (leftmost) `left 4` moves its right edge left.
- Resize mode `l`, `l`, `h` moves +2 then −1, and a bound `alt-y` with `amount = 5` works in normal mode.
- Clamp: with R at 3 columns, `--amount 10` moves 2 and leaves R at 1 column. Further requests (`{}` and amount 7) return 200 with the unchanged tab, and the `x-ship-revision` stays at 15 for both.
- Nested: in `[L | [R / D]]`, D `up 2` moves the R/D edge only; D `left 2` moves the outer edge for R and D. A valid resize on a zoomed tab unzooms.
- A missing `--direction` exits 2, a missing pane gives 404, and `"sideways"` gives 422.
- After each step, `stty size` in L, R and D matched the published content rectangles, for example R at `1` column after the clamp.
- `probe/` (temporary Rust crate): 67,051 one-cell steps across widths 3–301 found no drift and no unreachable target. A 30/81 split maps to 59/161 and back to 30/81.

### Real programs, two viewers: 40/40 (`workflow.py`)

zsh, `nvim -u NONE -i NONE` and Pi run in `[shell | [nvim / pi]]`, viewed by a 120x36 client and a 150x40 client through the proxy. Four rounds alternate the acting viewer. Each round zooms the shell, checks the other viewer and `stty size`, and swaps with `alt-shift-l`/`h` out of zoom. It then re-zooms, takes two resize-mode steps and checks again:

- Each zoom hid nvim and Pi in both viewers and marked the row `Z`. The zoomed shell's `stty size` matched its geometry.
- Every swap, resize, create and close unzoomed the tab.
- After every round, Neovim's `editing in nvim` and Pi's footer were on both screens, and the nvim process ID was unchanged.
- At the end the same three pane IDs remained. The proxy log shows swap bodies carrying only `other`. Both clients restored the terminal on exit.

Final rendered screen (`workflow-two.txt`, larger viewer, trimmed): the shell sits left at `34 54`, beside nvim (`editing in nvim`, `[No Name] [+]`) over Pi's prompt and `DeepSeek V4.1 Flash > think:high` footer. Dots fill the space the smaller viewer can't use.

### Wire and OpenAPI agreement

`contract/` (temporary crate) writes `ship_server::openapi()` to `openapi.json`. It asserts that the three paths take `ZoomPane`, `SwapPane` and `ResizePane` and return `Tab`; swap also lists 404 and 422. `Direction` is the `left/down/up/right` enum. Against an owned server, the ordinary client's `zoom`, `swap` and `resize` decoded real `Tab` responses:

- toggle zoom, explicit unzoom, and a repeated unzoom as a no-op;
- swap reversing the leaf order;
- resize changing the layout.

### Documentation

The README's literal recipe ran against an owned server through `ship` and `jq` (`docs.py`). Zoom printed the zoomed pane, swap printed `zoomed: null` and order `right, left`, resize changed the ratio, and the tab closed. `ship pane --help`, `pane zoom --help`, `pane swap --help` (`--other <ID>` required) and `pane resize --help` show the new descriptions.

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

Measured project implementation Rust (all Rust under `crates/`, excluding test paths): HEAD 7,406 lines, then 7,950, +544. The slice-3 figure of 7,386 predates the 3.7 overlay commit. Authored test Rust: 0 lines before and after.

All harness servers and clients were shut down. Existing user servers were left untouched.

## Unverified

- Linux and Windows. No container, VM or remote platform check was attempted.
- A person looking at a GUI terminal. Appearance checks used pyte-rendered output from real client PTYs.

## Uncertain decisions

- Unzoom-before-move applies to every `client.pane.*` action, including `prev`; the spec names focus, next and swap.
- A self swap is rejected with 422 rather than accepted as a no-op.
- A zoom request that changes nothing, and a resize that can't move, commit nothing and answer with the unchanged tab. An immovable resize on a zoomed tab therefore stays zoomed.
- The clamp lets no pane's content drop below 1x1 unless it was already smaller.
- The zoom marker is ` Z` in cyan bold.

```text
reused:   commit/repair/publish, the action queue's revision wait, focus's neighbor search, ratatui Fill/overlap geometry
new code: zoom/swap/resize edits, the exact-length ratio search and the zoom marker; the existing code had none of these edits
proof:    111 release workflow checks, matching stty/published geometry, a 67k-step drift probe, OpenAPI/ordinary-client probe, strict Clippy, fmt, builds and core feature checks on macOS
```
