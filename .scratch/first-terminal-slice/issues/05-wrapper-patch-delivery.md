# How should Ship retain and upstream the wrapper compatibility patch?

ID: 05
Parent: [Plan the first client/server terminal slice](../map.md)
Labels: wayfinder:grilling
Type: grilling
Mode: HITL
Status: open
Assignee: unassigned
Blocked by: 02

## Question

How should Ship consume the demonstrated ratatui-ghostty compatibility update reproducibly while keeping generic changes easy to contribute upstream: a pinned Git fork, Git subtree, or recorded source import?

The patch worked with libghostty-rs at 8953a740bc378cec3e07e1f6ca949f0595eab19b and Zig 0.16.0. [Feasibility evidence](../../../docs/research/terminal-feasibility.md) records the dependency requirement update and session API edits. No fork, upstream PR or vendored source has been added to Ship.

Cyan knows the fork-plus-Git-dependency workflow; subtree was discussed for self-contained agent checkouts, not selected. Decide delivery and pins with Cyan before implementation. Do not vendor structdiff or add custom derive handling preemptively. Preserve licensing/provenance and report upstream versus Ship-authored code separately. Publishing a fork/PR requires explicit authorization, not merely resolving this planning ticket.
