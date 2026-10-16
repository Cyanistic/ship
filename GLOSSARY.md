# Ship

Ship keeps terminal work in a server-owned tree of tabs and panes while clients maintain independent views of that work.

## Language

**Tab**:
A container at the server's top level or within another tab, with its own pane layout and ordered child tabs. A tab can be empty and selected without a pane.
_Avoid_: Session (the former top-level container, replaced by top-level tabs), Workspace (as an entity)

**Pane**:
A terminal location within a tab's pane layout.

**Saved tabs**:
The server's tab tree as last saved for its port: every top-level tab and everything under it, without live programs, terminal contents or other live state.
_Avoid_: Workspace, Session, Snapshot, Saved state

**Selection**:
A client's most specific chosen tab or pane, or nothing, held by the server for that attachment. The client, a caller naming the attachment, a selecting create or the server's repair rules may change it; sidebar expansion and remembered focus history remain client-local.
