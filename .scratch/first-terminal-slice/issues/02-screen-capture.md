# Can the supplied session expose a faithful transferable screen?

ID: 02
Parent: [Plan the first client/server terminal slice](../map.md)
Labels: wayfinder:prototype
Type: prototype
Mode: HITL
Status: resolved
Assignee: Cyan
Blocked by: None

## Question

Can ratatui-ghostty's supplied session provide complete screen and cursor data for owned Ship screen/cell types and faithful Ratatui reconstruction, without reimplementing Ghostty extraction? Which conversion fields and minimum integration changes are necessary?

After claiming, use the prototype skill for a temporary capture/reconstruction experiment and review the result with Cyan. Include graphemes, colors/reset, modifiers, underline configuration, wide-cell/skip behavior and cursor state. Inspect lifecycle/error limitations that could block first-slice use. Keep permanent tests and production implementation out of this ticket. Record actual build/platform evidence and missing evidence. Link any experiment asset rather than pasting it into the map.

## Comments

Resolution records Cyan's prior live review: the patched example worked, the parent-rerun capture/JSON probe passed, and Cyan accepted calling feasibility complete. This update records that concluded discussion rather than adopting a new schema.

## Answer

Use the supplied ratatui-ghostty session and owned screen/cell conversions as the first-slice starting approach. No low-level Ghostty extraction rewrite is needed for demonstrated behavior. The final owned schema remains to be specified; scratch Ratatui color/modifier coupling is not automatic approval of that wire schema.

The published wrapper needs the demonstrated small dependency/API update to use the inspected libghostty-rs 0.2.1 pin with Zig 0.16.0. How Ship retains that patch remains open.

## Evidence

[Terminal feasibility evidence](../../../docs/research/terminal-feasibility.md) records dependency pins, exact compatibility changes, 80 passing enabled upstream tests (one ignored), Cyan's interactive example confirmation, parent-rerun real-shell buffer/cursor conversion assertions and graceful cleanup. It links the disposable asset locations; no production implementation exists.

## Uncertain decisions

Atomic/coherent capture under continuous output, final independent cell schema, nondefault skip/diff options, alternate cursor variants, forced shutdown behavior and Linux compatibility remain unverified. The contract ticket must account for coherent publication and lifecycle. These limits do not invalidate the accepted first-slice reuse direction.
