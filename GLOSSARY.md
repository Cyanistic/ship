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

**Status record**:
What a pane's program last reported about itself through OSC 7501, the Program Status Protocol: a state (`idle`, `working`, `done`, `blocked` or `error`) with an optional `kind`, `app`, `title`, `msg`, `progress` and `id`. The record under the empty id is the root record, the program's own state. The server keeps records with the pane but doesn't save them.
_Avoid_: Agent state, Status (alone)

**Display state**:
What a client shows for a pane with a root record: `blocked`, `error`, `working`, `unseen` or `idle`. Each client derives its own; `unseen` means that client saw the program finish while the pane wasn't in view in a focused window, and a reported `done` alone displays as `idle`.
_Avoid_: Agent status, Done (as a display state)
