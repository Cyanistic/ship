# Can the supplied session expose a faithful transferable screen?

ID: 02
Parent: [Plan the first client/server terminal slice](../map.md)
Labels: wayfinder:prototype
Type: prototype
Mode: HITL
Status: open
Assignee: unassigned
Blocked by: None

## Question

Can ratatui-ghostty's supplied session provide complete screen and cursor data for owned Ship screen/cell types and faithful Ratatui reconstruction, without reimplementing Ghostty extraction? Which conversion fields and minimum integration changes are necessary?

After claiming, use the prototype skill for a temporary capture/reconstruction experiment and review the result with Cyan. Include graphemes, colors/reset, modifiers, underline configuration, wide-cell/skip behavior and cursor state. Inspect lifecycle/error limitations that could block first-slice use. Keep permanent tests and production implementation out of this ticket. Record actual build/platform evidence and missing evidence. Link any experiment asset rather than pasting it into the map.
