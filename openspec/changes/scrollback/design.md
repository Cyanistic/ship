# Design

Summary of the three locked papers in `design/`, which are the source of truth. Where this file and a paper disagree, the paper wins. Product, architecture and program were locked by Cyan on 2026-10-11, after an amendment the same day deferred edit-scrollback. Locking does not start implementation: each slice runs on Cyan's request and stops for review before the next.

- [`design/product.md`](design/product.md): what and why. User stories P1 and P3 to P5, FR-001 to FR-013 and FR-023 to FR-029, success criteria.
- [`design/architecture.md`](design/architecture.md): shape A, typed wrapper commands, and key decisions 1 to 9 and 12 to 14.
- [`design/program.md`](design/program.md): the planned files, types and signatures, the build order and the deviation log.

## Context

Panes keep about 10 KB of history today and have no way to scroll it, read it from a script, or select it with the mouse. Ship doesn't capture the mouse at all. This change adds history scrolling shared by every viewer, `ship pane read`, mouse capture with click-to-select, and drag selection with copy to the client's clipboard over OSC 52. Probe evidence is in `experiments/scrollback-probe/`, and the design conversation's notes are in `notes.md`.

## Goals / Non-Goals

**Goals:** see the product paper's user stories P1 (scroll), P3 (click around), P4 (select and copy) and P5 (read from a script).

**Non-goals:** see the product paper's Non-goals. The main ones:

- Copy mode and search. `ship pane read` stands in for search until it lands.
- Edit-scrollback, deferred on 2026-10-11 (product paper, "Amendment 2026-10-11").
- A bindable run action and `ship clipboard copy`.
- Per-client scroll positions.

## Decisions

- **Terminal mechanics live in the vendored wrapper** (architecture decision 1). Scrolling, reading, mouse routing, gestures, drawing the selection and clipboard events become typed wrapper commands and events, recorded as changes 12 to 16. Ship routes them and decides who gets a copy.
- **Scroll position and selection belong to the pane**, so every viewer sees the same thing (FR-001, decision 3). Two clients dragging at once share one selection.
- **The wrapper outlives its program**, so exited panes still scroll, read and select (decision 2).
- **Mouse events travel on the input stream** as `InputFrame::Mouse` (decision 6). The client hit-tests panes and sidebar rows, and the wrapper decides the rest from the program's live modes (decision 5).
- **Clipboard copies are attach-stream events** (decision 7, refined in the program paper's rationale into a separate bus value). A copy goes to the client that made it. A program's own clipboard write goes to every client viewing its tab.
- **History is capped at 10 MB per pane**, with no line cap (FR-005, decision 12).
- **Settled program gates:**
  - G1: `ship pane scroll --direction up|down|top|bottom [--amount N]`, where the amount is lines and omitting it scrolls a page.
  - G2: the wheel over a pane that isn't selected scrolls that pane without selecting it.
  - G5: selected text shows with foreground and background swapped until rendering review.
- **Protocol version 9.**

## Build order

Program paper, "Build order". Four feature slices, then a slice for docs, issues and size:

1. History and scrolling by keys.
2. `ship pane read`.
3. The mouse.
4. Selection and the clipboard.
5. Docs, issues and size.

## Risks / Trade-offs

Architecture paper, "Risks and unknowns". The ones to watch:

- Memory at the 10 MB cap (U-2), measured in slice 1.
- Long reads stall the pane being read.
- The wrapper reaches about 16 recorded changes.
- OSC 52 size limits in some outer terminals.

Estimated size: 320 to 420 lines of implementation Rust, plus about 300 uncounted wrapper lines.

## Migration Plan

History isn't persisted, so there's nothing to migrate. Protocol 8 goes to 9, so server and client are rebuilt as a pair, and mismatched versions fail the health check as before.

## Open Questions

- U-3: whether `alt-pageup` and `alt-pagedown` reach Ship in Ghostty, Kitty and macOS outer terminals. They work in foot on Linux, and they're rebindable defaults.
- U-4: the scroll indicator's text and the selection's final colors, settled in rendering review.
- Tracking issues drafted in slice 5 carry copy mode, search, the 10 KB default, edit-scrollback and the run action.
