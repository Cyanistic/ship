# Ship

Ship keeps terminal work in a server-owned tree of tabs and panes while clients maintain independent views of that work.

## Language

**Tab**:
A container at the server's top level or within another tab, with its own pane layout and ordered child tabs. A tab can be empty and selected without a pane.
_Avoid_: Session (the former top-level container, replaced by top-level tabs), Workspace (as an entity)

**Pane**:
A terminal location within a tab's pane layout.

**Selection**:
A client's most specific chosen tab or pane, or nothing. It is distinct from sidebar expansion and remembered focus history.
