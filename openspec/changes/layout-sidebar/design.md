# Design

Planning artifact for a locked plan. See `proposal.md` for motivation and capability mapping. Cyan answered the seven planning gates (G1 to G7) in chat on 2026-10-10; **Settled gates** records the answers. Product A-2, architecture A-2/A-3 and the program paper were locked on 2026-10-10. Each slice still runs on Cyan's request and stops for review before the next.

## Context

Read in full: `design/product.md`, `design/architecture.md`, `design/program.md` and all ten existing main specs. Product A-2 and architecture A-2/A-3 supersede their originals where indicated. Later conversation confirms a floating-point first-child share, squeeze-to-fit shrink, bindable resize amount and directional edge movement with opposite-edge fallback, and then settles G1 to G7. It does not authorize implementation.

The current code explains why the change crosses several layers:

- `ship-core/src/model.rs` stores `Tab.panes` as an ordered map. `tree.rs` traverses that map for membership and descends into child tabs in `first_pane`. These choices drive today's label, cycling and selection behavior, not just serialization.
- `ship-client/src/execute.rs` discards create `direction` and converts a pane target into its owning tab. `ship-core/src/command.rs` already accepts split direction and resize amount, but `execute` returns unavailable for resize. Preserving the anchor must therefore start in shared command execution and continue through the request and state edit.
- `ship-server/src/state.rs` computes the smallest viewer area in `tab_sizes`, clamping each dimension to at least one, then `apply_sizes` sends that whole area to every pane. `commit` repairs selections, publishes a replica and resizes programs. The layout must be introduced at this ownership boundary rather than divided independently by the renderer.
- `ship-server/src/attach.rs` already watches all panes of the viewed tab. Conversely, `ui/observer.rs` only marks screen updates visible for the selected pane, and `ui/draw.rs` draws only that pane. Fixing just the draw function would leave neighboring panes stale.
- `ui/mod.rs` has a serialized action queue and revision catch-up, but top-level tab cycling and `tree::first_pane` can land in a child's pane. The sidebar's visible-row list and own-pane landing replace these callers.
- `keymap.rs` currently loads only modes into `Keymap`; `RawClient` rejects other fields. Mode bindings are held in unordered maps after Figment merging. `Keymap::load` is called by `ship config check` in `ship/src/commands.rs` and by the client's startup and reload in `ui/mod.rs`; those calls move to `ClientSettings::load` (G4).
- `protocol.rs` is at version 7 and has no geometry in `Replica`; `api.rs` has tab inspection and attach, but no standalone replica/geometry read method. G7 drops the CLI directional swap, so no CLI geometry read is needed.
- `pane.rs` forwards titles and exits, but not the wrapper's existing OSC 7 `CwdChanged`. It starts PTYs at positive fallback/tab sizes today. The renderer's `paint` divides by screen columns; zero content and zero terminal sizes need a probe before that logic is reused.

The main specs' obsolete `ship pane rm`/positional-create examples in `pane-programs` are unrelated legacy inconsistencies. This plan uses the actual `close` and flag-based commands; it does not use those examples as permission to revive old commands or rewrite unrelated requirements.

## Goals / Non-Goals

**Goals:**

- Keep one owner for layout geometry and one stored record of pane membership/order.
- Deliver runnable checkpoints from real CLI/key input to inspection, terminal size or rendered output.
- Preserve process lifetime, faithful screens, ordered input, revision ordering, config reload, startup policy and ordinary shared-DTO API methods.
- Identify which specification clauses are settled and which acceptance cases remain blocked, rather than converting recommendations into requirements.

**Non-Goals:**

- No duplicate layout computation in clients, pane map alongside the layout, server-owned recency or directional server swap mode.
- No standalone geometry endpoint or directional-swap CLI geometry read, no comfortable pane minimum, and no per-binding hint override.
- No implementation, dependency/config edits, main-spec synchronization, permanent tests or paper lock changes during planning. The proposal lists the product exclusions.

## Decisions

### Settled ownership and representation

The architecture selects server-published geometry over a shared function independently run by server and client: the server already needs rectangles to size PTYs, and publishing that result prevents rounding disagreement. Architecture A-5 and program P-2 supersede the earlier beside-tabs representation: geometry is optional on each published `Tab`, recursively, derived only for viewed tabs. The state actor retains geometry-free source layouts; publication computes geometry from layouts and viewers. `Replica` has no parallel geometry map, and clients read `tab.geometry` without recomputing layout. Cyan approved and re-approved this correction before commit ("sounds right. let's go for it!").

A tab holds an optional binary layout whose leaves contain `Pane`, plus an optional zoomed pane ID. Leaves traversed first child before second are the pane order. The rejected pane-map-plus-ID-tree shape would duplicate membership and order. Tab lookups, attach membership, labels, pane cycling, runtime retention and state mutation must all switch together to layout walks.

Split ratios are finite floating-point shares strictly between zero and one. First gets `ratio`, second gets `1 - ratio`; first is left or top. The current skeleton uses an `f32` wrapper. `0.7` means 70% of the total, not first divided by second. The earlier integer ten-thousandths proposal is superseded. Whole-cell rounding remains a verification obligation.

Existing layouts squeeze into the available tab area without changing stored ratios or deleting panes. Enlargement restores proportions subject to rounding. User edits have a mechanical 1x1 content minimum (G2): a split that would leave either half below it is refused with "no space for new pane", and a resize is clamped to it, committing nothing when the edge can't move. Terminal shrink ignores that minimum. Architecture A-4 and program P-1 settle the zero-content gate with Cyan's approval in this implementation conversation: clamp both PTY and Ghostty dimensions to at least 1x1 on startup and resize, while published geometry remains actual and possibly zero. Draw cells and cursors only within actual content, drawing neither when either dimension is zero. Positive terminal sizes match positive content; zero content is the explicit floor exception. This does not permit an offscreen oversized layout.

### Slice 1: CLI split/close to inspected layout and program size

**Input to output:** `ship pane create --pane <id> --direction down`, or a tab target, passes the anchor through execution and the API to a server tree edit; `ship tab get`, the observation geometry and `stty size` show the result. Closing the selected pane collapses the split and repairs the selection into the subtree taking its space.

**Approach:** Add the layout and geometry descriptions in ship-core and server-local geometry/tree edits. Replace map readers throughout core, state, attach and the client consumers needed to compile. `CreatePane.parent` becomes `at`, a tab or a pane, beside `direction`; `PaneInput` is unchanged because it also describes tab starters (G1). The server resolves a tab-only target to its largest content rectangle, ties by layout order. The paper's 80x24 fallback for unviewed tabs is separate from stored last terminal sizes. A new split initially uses the architecture's equal halves.

Compute optional geometry on each tab in the existing commit/replica publication path, including nested viewed children under unviewed parents, and size visible panes recursively from that same committed projection. Tab list/get/create/rename/move responses have the same published-tab semantics. Geometry is omitted when a tab is unviewed and is never cached in the source tree. Check the 1x1 minimum before starting the program, and retain the existing failed-edit runtime cleanup, so refused or failed creation leaves no leaked program. Close repair reads the old tree, descending toward the closed side on the parent axis and taking first child on other axes; ancestor fallback remains intact.

**Verification:** Temporary ratatui probes check fraction constraints, one-cell overlap and merged borders at odd sizes and nested three-way splits. Shrink below border/content capacity and enlarge again, checking geometry containment, unchanged ratios and live programs. Split a pane too small for two 1x1 halves and check the refusal leaves no program behind. Check there is no `Replica.geometry` in implementation/schema, and each tab has geometry exactly when viewed, including at three levels and after viewer/move/close edits. Compare published content sizes with real `stty size`; inspect empty-tab creation, largest-pane ties, close collapse and selection repair. Verify architecture A-4's approved 1x1 internal floor and zero-content cell/cursor clipping. Stop if PTY/rendering needs any additional unapproved fallback. At this checkpoint the client may still draw only its selected pane, now at that pane's size.

### Slice 2: Focus keys to simultaneous faithful screens

**Input to output:** Split keys show all panes; `alt-h/j/k/l` select the adjacent pane and typing reaches it. A return to a worked-in tab restores this client's own-pane selection once tab navigation is wired in slice 3.

**Approach:** Introduce pane rendering and local selection-memory modules as in the papers. Offset published frames by the client's pane-area origin; draw shared labeled borders only for multiple panes, paint each content rectangle, and place/show the selected pane's cursor using its offset and clipping. Change observer visibility from selected-pane equality to visible-geometry membership. `TerminalGuard` cursor state and `Frame` positioning must agree. Do not predict a tree edit from a key before its replica arrives.

The shared neighbor search in ship-core examines adjacent published frames. Interactive focus uses client-local recent selections, then center distance and layout order. Memory records the last selected own pane per tab and prunes removed IDs. It never crosses the wire. Slice 4's `client.pane.swap` reuses it.

**Verification:** Run an editor, agent and test shell together; exercise wide characters/colors, cursor location, alternate-screen entry/exit, split and close. Check one-pane borderlessness and recency ties. Compare typing with single-pane behavior while another split produces sustained output. Inspect the actual rendered terminal, not only screenshots of a buffer or compilation. Linux is unverified unless run there locally; Windows remains unverified.

### Slice 3: Sidebar/config keys to visible-row navigation and resized panes

**Input to output:** `alt-g`, `j/k`, `h/l`, `esc` navigate/fold the recursive tree, `alt-b` hides/shows it, and saving `[client] sidebar_width = 40` changes the pane area and program sizes.

**Approach:** Keep folds, visibility and pane memory local. Use the papers' tui-tree-widget direction if its ratatui version is compatible; verify rather than assuming the new dependency works. The client-owned folded-ID set is the source of truth; widget state is presentation. New rows are expanded by default, and selected ancestors are expanded on reconciliation. Next/prev use visible rows whether or not the sidebar is shown. Landing prioritizes shared zoom, then a surviving remembered own pane, then first own pane, else the tab.

Exclude the shown sidebar and bottom bar from Attach/View area reports using saturating arithmetic; preserve the smallest-viewer contract. Route reload through the existing loading/checking path, retaining existing error locations and bad-reload behavior. Remove only the row-number defaults; their action stays unavailable. `ClientSettings` owns `[client]`: the `Keymap` and `sidebar_width` (G4). Mode chords move to an `IndexMap` so hints keep file order. Each hint labels a chord from its binding's first action: the leaf's scalar value when it has one, else the leaf's name, so the default resize mode reads `h left  j down  k up  l right  esc normal` (G3). Hints follow the existing status content and are cut from the end when the bar is full. Exact glyphs/colors can be chosen during rendered review without changing behavior.

**Verification:** Build a three-level tree, including an empty parent with a populated child; navigation must never open a child's pane in the parent's area. With two clients, verify different folds, last-pane memories and sidebar visibility. Hide the smallest viewer's sidebar and check `stty size`. Exercise width save, rename-save, invalid type and explicit reload with `ship config check`; ensure changed bindings/unbindings change the hints on reload, and a narrow terminal cuts hints from the end. Check `alt-1` is delivered normally and row-select still reports unavailable.

### Slice 4: Zoom/swap/resize input to shared layout changes

**Input to output:** `alt-f`/`ship pane zoom`, directional swap keys or explicit CLI IDs (`ship pane swap --other`), and resize keys/CLI requests produce observable shared geometry and intact pane programs/screens across viewers.

**Approach:** Zoom uses the tab field; hidden programs keep their last sizes. The architecture's optional desired zoom state supports an explicit internal unzoom before focus/next, while the public key/CLI actions stay toggles. Use the existing action queue's revision wait before resolving/moving, so concurrent unzoom attempts cannot re-toggle zoom. Structural operations clear zoom as part of their successful edit; invalid swap membership must be checked without partially clearing zoom.

Product A-2 and architecture A-3 override server-direction swaps. Source ID is in the route and the other ID in the body; server validation and leaf exchange occur in one commit. Client-direction swap uses published geometry plus local recency. In a zoomed tab it queues the explicit unzoom first, waits for the revision, and resolves on unzoomed geometry; with no neighbor the tab stays unzoomed and no swap is sent (G6). The CLI has no directional swap: `ship pane swap` requires `--other` and never reads geometry (G7). The explicit-pair request accepts stale resolution if both IDs still belong to the same tab. Nonadjacent pairs are valid. Do not add server re-resolution or revision rejection to make stale geometry appear directional again.

Resize moves the requested edge along the direction, or the opposite edge when the requested side is the tab boundary, by changing the appropriate ancestor split's fraction. Keep direction and amount in the existing shared command; add no grow/shrink flag, separate amount setting or extra modes. An omitted amount is 1 cell, and the default resize chords omit it (G5). Resizes clamp at the 1x1 minimum (G2). A temporary probe must show that converting cells to fractions and back moves the intended number of cells without drift.

**Verification:** Verify zoom repair with two viewers and on reconnect, internal unzoom races, tab switching, and each structural edit while zoomed. Inspect swap wire requests for IDs only, direct nonadjacent swaps, stale geometry pairs, missing/cross-tab rejection, `ship pane swap` without `--other` rejected, and no-neighbor behavior. Verify zoomed resolution: a swap unzooms then exchanges, and with no neighbor leaves the tab unzoomed. Resize a movable edge by five cells and by one default press and compare geometry and `stty size`; test fallback edges and clamping at the 1x1 minimum.

### Slice 5: Directory-aware split and child-tab key to selected shell

**Input to output:** After `cd /tmp` and an OSC 7 report, splitting from keys starts a shell there; `ship pane get` exposes the reported directory. `alt-shift-n` appends and selects a child shell tab.

**Approach:** Forward the wrapper's existing directory event through `PaneChange` and a state commit, like titles; store it separately from the immutable starting `cwd`. CLI execution keeps supplying the caller's absolute directory; keys allow server-side anchor-directory fallback. Do not guess directories by querying live process internals. `tab create --child` flattens a `PaneTarget`, so `ship tab create` gains `--pane`, read only with `--child` as "the tab of this pane" and ignored otherwise; inside a pane `SHIP_PANE_ID` fills it (G1). Resolve current client/CLI targeting to the existing parent destination, not a server-side selected-tab concept.

**Verification:** Check Cyan's actual zsh OSC 7 reporting and a shell with reports disabled. Confirm key splits use reported/starting directory and CLI splits use the caller's directory. Build nested trees with child keys, check last-child placement and selection, and check no-selection top-level fallback. Verify from command help and real requests that `ship tab create --child` inside a pane makes a child of that pane's tab, plain `ship tab create` inside a pane still makes a top-level tab, and `--child --parent X` is rejected.

Update README/help alongside the slice that introduces each user-facing behavior, not as a detached horizontal docs slice. At every checkpoint report applicable checks and actual workflow evidence; measured implementation Rust and test counts are separate. No authored permanent tests are authorized.

## Risks / Trade-offs

- [Cell rounding, overlap or border merge disagrees with PTY size] -> Temporary ratatui probe before slice 1 and rendered terminal checks in slice 2; reopen geometry decisions if the selected mechanism fails.
- [Zero-area content reaches code expecting positive dimensions] -> Probe wrapper/PTY/screen handling, clipping and cursor arithmetic. Current positive tab-size clamps do not prove per-pane zero-size safety; stop rather than silently adopt a fallback.
- [Directory events add replica commits] -> Follow title-event deduplication; judge responsiveness through real sustained-output/directory workflows before adding machinery.
- [Replica geometry grows with viewed tabs] -> Accept the papers' derived-on-wire trade-off for one geometry owner; do not silently move it to a new endpoint or stream.
- [CLI users lose directional swap] -> Accepted (G7). Scripts name panes by ID. If it's wanted later, a short-lived attach can read the replica without a new route; it would fail for unviewed tabs.
- [Zoomed swap with no neighbor still unzooms] -> Accepted (G6), matching pane focus out of zoom.
- [Stale swap resolution exchanges a pair no longer adjacent] -> This is the approved explicit-pair contract, not an error to fix. Reject only invalid membership, preserving atomicity.
- [Spec category conflict] -> `keymap` is explicitly amended for `client.pane.swap`; it remains local resolution of a server edit, not client authority over shared state.
- [File-existence status appears complete before decisions are] -> Keep gates in tasks and report draft readiness separately from `openspec status` and schema validation.

## Migration Plan

There is no persisted layout to migrate. Later implementation replaces the model/wire representation and advances protocol 7 to 8 together. Update server and client builds as a compatible pair; mixed versions fail health before attaching. Current application dispatch also validates explicit ordinary commands and `server stop` through its existing health helper, without autostart; endpoint methods remain policy-free. Cyan approved documenting the already-built protocol-7 limitation: explicit ordinary commands in that binary skip health and can reach protocol-8 endpoints, so no compatibility guarantee applies to them. Slice 1 does not add a request marker/server guard or retroactively protect that old path. Acceptance requires current-client rejection before layout/attach/shutdown, old-client rejection on health-checking paths, and explicit evidence of the legacy exception. OpenAPI and ordinary client methods must match the rebuilt server through real requests, not compilation alone.

Restarting the server discards in-memory tabs and programs, so any live rollout or rollback must be separately authorized. This planning run performs neither. A rollback to a version-7 build requires the matching version-7 client/server pair and cannot restore discarded in-memory work. Do not introduce backwards wire translation or disk persistence.

## Settled gates

Cyan answered these in chat on 2026-10-10 ("i agree withll of those! go for it!"). They are folded into the slices above, the program paper, product A-2 and architecture A-2/A-3. Those amendments and the program paper were locked the same day.

| Gate | Question | Decision |
| --- | --- | --- |
| G1 | Where does pane placement go, and what does `tab create --child --pane` mean? | `CreatePane.at` (tab or pane) beside `direction`; `PaneInput` unchanged. `--pane` on `tab create` is read only with `--child`, as "the tab of this pane", and ignored otherwise, because `SHIP_PANE_ID` fills it inside every pane. |
| G2 | What limits apply to new splits and manual resizes? | A mechanical 1x1 content minimum. Splits below it are refused ("no space for new pane"); resizes clamp and commit nothing when the edge can't move. Terminal shrink ignores it. Architecture A-4 settles zero-content handling: both PTY and emulator use a 1x1 internal floor, with actual published geometry and clipped cells/cursor. The slice 1 probe verifies it. |
| G3 | What do mode hints show? | Each chord in file order, labeled by its binding's first action: the leaf's scalar value, else the leaf's name. Cut from the end when the bar is full. No per-binding override. |
| G4 | Where does `sidebar_width` live? | `ClientSettings { keymap, sidebar_width }`, loaded where `[client]` is loaded today. |
| G5 | What is one omitted resize step? | 1 cell. The default resize chords omit `amount`. |
| G6 | Does a zoomed directional swap unzoom first? | Yes: unzoom, wait for the revision, resolve on unzoomed geometry. With no neighbor the tab stays unzoomed and no swap is sent. |
| G7 | How does the CLI get geometry for a directional swap? | It doesn't: `ship pane swap --direction` is dropped. `ship pane swap` requires `--other`. |

## Open Questions

Only deferrable rendering details remain here: the exact glyphs, colors and highlight styles for borders, folded rows and the zoom marker. They must satisfy the specified distinctions and can be reviewed in the real rendered terminal without changing capability ownership or operation semantics.
