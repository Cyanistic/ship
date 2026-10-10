# Proposal

Planning draft. The papers' superseding amendments and the subsequent conversation confirmations guide this proposal, including Cyan's answers to the planning gates on 2026-10-10 (`design.md`, Settled gates). Product A-2, architecture A-2/A-3 and the program paper were locked on 2026-10-10.

## Why

Ship currently draws one selected pane at a time and cycles top-level tabs, so a shell, editor and agent cannot be used side by side. A shared split layout and a client-local tab sidebar make the existing recursive tab tree usable for daily work.

## What Changes

- Store panes once, as leaves of each tab's nested binary split layout. Publish server-computed frame and content geometry as optional geometry on each recursive tab, present exactly when viewed, and size each terminal from that same published geometry. Keep source layouts geometry-free; remove the parallel replica geometry map (architecture A-5/program P-2, approved by Cyan in the slice-1 correction conversation).
- Honor right/down pane creation, collapse splits on close, repair selection into the space reclaimed by the sibling subtree, and use first-child-before-second tree order throughout.
- Draw simultaneous pane screens with labeled shared borders, directional focus and client-local pane memory. Preserve single-pane borderless rendering and existing input, status and empty-state behavior.
- Show a foldable left sidebar titled `tabs`, without a tab bar. Navigate visible rows even when the sidebar is hidden; configure fixed width through `[client] sidebar_width`, default 26.
- Add shared zoom, explicit same-tab pane swaps, client-resolved directional swaps from keys and working directional edge resize. Resize `amount` remains an action argument and is bindable; omitted, it is 1 cell. New splits and resizes keep at least 1x1 of content: a split below that is refused and a resize is clamped. `ship pane swap` takes an explicit `--other` only.
- Start key-created panes in the selected pane's reported current directory, else its starting directory. Keep CLI creation in the caller's directory. Add the child-tab key and `ship tab create --child`, which reads `--pane` (filled from `SHIP_PANE_ID` inside a pane) as the parent's pane.
- Add `alt-f`, `alt-shift-h/j/k/l` and `alt-shift-n`; remove default `alt-1..9` bindings while retaining the unavailable row-selection action. Add non-normal-mode hints that label each chord with its action's value or name, in file order, such as `h left  j down  k up  l right  esc normal`.
- **BREAKING**: replace the pane-map wire representation and pane-create targeting shape, advancing protocol compatibility from 7 to 8. Existing clients must not attach across that boundary. `CreatePane.parent` becomes `at`, a tab or a pane, beside `direction`.
- **BREAKING**: replace top-level-only tab cycling and creation-order pane semantics with visible sidebar rows and layout order. Permit same-tab swaps where independent pane reordering was previously forbidden.

Mouse support, scrollback, floating panes, UI renaming/prompts, agent markers, persistence, themes and new resize modes are out of scope. Small windows squeeze existing layouts without deleting panes or changing proportions. A comfortable content minimum beyond 1x1 and a CLI directional swap are also out of scope.

## Capabilities

### New Capabilities

None. The existing specs already own tab/pane structure, observation, rendering, keys, programs and configuration.

### Modified Capabilities

- `session-structure`: shared pane layout, anchor-based creation, pane order, close collapse, same-tab ID-pair swaps and directional edge resize.
- `terminal-client`: simultaneous screens, pane geometry and borders, sidebar presentation, area reporting, mode hints and layout-derived labels.
- `session-observation`: visible-row navigation, per-tab pane memory, close/zoom selection repair, simultaneous screen observation and shared zoom.
- `pane-programs`: anchor targeting, key-created pane directories and inspection of the shell-reported directory.
- `keymap`: working layout actions, child-tab key behavior, changed defaults and directional-swap resolution as a client action that submits a server edit.
- `config-file`: fixed sidebar-width setting under the existing client config contract, owned with the keymap by one client-settings type.
- `health-exchange`: protocol version 8 for the incompatible layout wire change.
- `observer-recovery`: existing selection retention subject to the shared zoom repair rule on reconnect.

The deltas specify settled behavior, including the gate answers. Architecture A-4 and program P-1 record Cyan's approved zero-content handling: a 1x1 internal PTY/emulator floor, actual published geometry, and cells/cursors clipped to actual content. The slice 1 probe verifies that policy.

## Impact

- `ship-core`: `model.rs`, `tree.rs`, `protocol.rs`, `command.rs` and new layout/geometry descriptions and shared neighbor search.
- `ship-server`: state actor commit/repair/sizing, pane events, attach stream membership, pane-operation routes and OpenAPI registration.
- `ship-client`: shared CLI/key execution, ordinary API methods, observer visibility, pane drawing/cursor placement, local sidebar/memory, config loading/reloading and key defaults.
- Existing ratatui 0.30 and vendored terminal wrapper are relevant mechanisms. The papers propose ratatui-core serde and tui-tree-widget dependencies; dependency compatibility and geometry rounding require temporary probes, not assumed success.
- Preserve the existing health/startup, revision ordering, process lifetime, transport and OpenAPI maintenance contracts. `client-maintenance` and `local-foundation` remain applicable without changed requirements.
- Later implementation verification uses real commands, rendered terminal workflows and temporary probes only. No permanent tests, implementation edits, paper lock changes or commits are authorized by this planning run.
