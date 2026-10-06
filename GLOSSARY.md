# Ship

Ship keeps terminal work in server-owned sessions while clients maintain independent views of that work.

## Language

**Session**:
A server-owned collection of root tabs and their descendants. It is the top-level container for terminal work.
_Avoid_: Workspace (as an entity)

**Tab**:
A container within a session or another tab, with its own pane layout and ordered child tabs. A tab can be empty and selected without a pane.

**Pane**:
A terminal location within a tab's pane layout.

**Selection**:
A client's most specific chosen session, tab or pane. It is distinct from sidebar expansion and remembered focus history.
