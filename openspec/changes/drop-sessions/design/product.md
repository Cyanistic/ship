# Drop sessions

Status: Locked again on 2026-10-08 with amendment A-1, which Cyan approved in Plannotator with "LGTM". First locked on 2026-10-08: Cyan approved the first draft in Plannotator with "LGTM" and resolved U-1 in conversation: drop the word "session" from user-facing text. During architecture design Cyan decided that the first tab comes from server startup, not from opening an empty server; the text that decision replaces is struck through and marked superseded by A-1. This paper defines observable behavior, not architecture or implementation. The decisions behind it are recorded in [notes.md](../notes.md). The locked [session/tab roundtrip](../../archive/2026-10-06-session-tab-roundtrip/design/product.md) and [pane terminals](../../archive/2026-10-07-pane-terminals/design/product.md) papers stay in force except where this paper amends them.

## Summary

A Ship server stops holding sessions. It holds tabs directly, and those tabs hold child tabs and panes, like folders on a disk. Running `ship` opens the client on the whole server instead of attaching to one named session. This prepares for the overview planned in the layout change, where the sidebar shows everything on a server, and later on several machines.

Today a session is a tab that can't hold panes but must have a unique name. Once a client sees the whole server, choosing one session to attach to does nothing, and the name adds a lookup the server never needed.

Decided so far, in conversation:

- The session concept is removed, not renamed. Top-level tabs take its place.
- A server holds only tabs at the top. Every pane belongs to a tab.
- `ship` opens the client with nothing selected. The command line never selects anything for the client.
- Names stay display labels. Every command that targets a tab or pane takes its ID.
- ~~When `ship` opens a server with no tabs, the server creates one with a shell in its home directory, and the client selects it.~~ Superseded by A-1.
- A-1: A server creates tabs on its own only at startup, from a starter it is given; `ship server` alone starts empty. When `ship` starts the server itself, it asks for one tab with a shell in the server user's home directory and selects it. A running server with no tabs is valid, and opening it shows the empty hint.
- The sidebar, its folding and the overview belong to the layout change. Several machines belong to a later change.
- "Session" leaves user-facing text too. Top-level tabs are called tabs, so there is one word for one thing.

## Goal

Make a Ship server one tree of tabs and panes that a client opens as a whole, so the coming overview can show everything on a server and so the model has one container type instead of two. It serves Cyan, who moves between many pieces of work and wants to see them, not attach to them one at a time.

## Interface

The command line, before and after:

```text
before                                   after
ship                    server health    ship                    open the client on the server
ship attach <name|id>   open a session   (removed)
ship session list|create|get|rename|rm   (removed)
ship tab create <parent>                 ship tab create [<tab-id>]       no parent: top level
ship tab move <id> <parent>              ship tab move <id> [<tab-id>]    no parent: top level
                                         ship tab list                   top-level tabs and descendants, JSON
ship server                              ship server [--starter]         empty, or one shell tab in ~ (A-1)
ship server stop                         ship server stop
                                         ship server status              what bare `ship` printed before
```

Until the layout change adds the sidebar, the client keeps today's single-pane view. With nothing selected, the pane area shows a hint instead:

```text
3 tabs · C-b ) to open one
no tabs · ship tab create
```

## User stories

### P1: Open the server, not a session

**Given** a server with several top-level tabs, **when** I run `ship`, **then** the client opens with nothing selected, shows how many top-level tabs exist and how to open one, and `C-b )` opens the first top-level tab.

**Given** a top-level tab is selected, **when** I press `C-b )` or `C-b (`, **then** the next or previous top-level tab opens, wrapping at the ends.

Independent verification: create three top-level tabs with the CLI, run `ship`, check the hint, and cycle through all three in both directions.

### P1: Start on an empty server

**Given** no server is running, **when** I run `ship`, **then** a server starts, a top-level tab with a shell in the server user's home directory exists, and the client opens with that tab selected.

~~**Given** a running server with no tabs, **when** two clients run `ship` at the same moment, **then** exactly one tab is created.~~ Superseded by A-1.

**Given** a running server with no tabs, **when** I run `ship`, **then** the client opens with nothing selected, shows `no tabs · ship tab create`, and no tab is created. (A-1)

**Given** no server is running, **when** I run `ship server`, **then** it starts with no tabs; with `--starter`, it starts with one tab holding a shell in home. (A-1)

Independent verification: stop the server, run `ship` from a project directory, and check the shell's directory is home. ~~Remove every tab, open two clients together, and count the tabs.~~ Remove every tab, run `ship`, and check that the hint shows and the tab count stays zero. Start `ship server` with and without `--starter` and list the tabs. (A-1)

### P1: Script tabs by ID

**Given** a server, **when** I run `ship tab create` with no parent, **then** a top-level tab is created and printed as JSON with its ID, and `ship tab list` includes it.

**Given** a nested tab, **when** I run `ship tab move <id>` with no parent, **then** it becomes the last top-level tab with its panes and child tabs intact and still running.

Independent verification: build a two-level tree with the CLI, move a nested tab to the top level and back, and check that its programs keep running.

### P2: Keep working when tabs go away

**Given** a client viewing a tab, **when** that tab is removed and another tab remains to fall back to, **then** the client moves to it as selection repair does today. **When** nothing is left to fall back to, **then** the client stays attached with nothing selected.

**Given** a client attached to a server, **when** the last tab is removed, **then** the client stays attached and shows the `ship tab create` hint. It ends only when I detach or the server shuts down.

## Non-goals

- The sidebar tree, folding, and the overview of the whole server. The layout change builds them, and starts the tree collapsed.
- Several machines in one client, and which servers get a first tab when one client opens several.
- Selecting a tab from the command line, such as `ship work` or `ship open work`.
- Looking tabs up by name or path, and any rule that tab names be unique.
- Panes held directly by the server, outside any tab.
- Moving tabs between servers.
- Aliases or migration for the removed commands and routes. Ship has no saved state yet, so nothing on disk changes.
- Configurable keys. The keys and config change replaces the `C-b` bindings.

## Functional requirements

- **FR-001:** A server MUST hold an ordered list of top-level tabs. It MUST NOT hold sessions or panes outside a tab.
- **FR-002:** Every tab and pane command and route MUST identify its target by ID. Names MUST NOT be used to look anything up.
- **FR-003:** `ship` with no arguments MUST start a local server when none is reachable at the default address, as it does today, and MUST open the client on the whole server.
- **FR-004:** The client MUST open with nothing selected, except as FR-005 says.
- ~~**FR-005:** When `ship` opens a server with no tabs, the server MUST create one top-level tab containing a shell started in the server user's home directory, and the client MUST select it. Clients opening the same empty server at the same time MUST together cause exactly one tab to be created.~~ Superseded by A-1.
- **FR-005 (A-1):** When `ship` finds no server reachable at the default address, the server it starts MUST begin with one top-level tab containing a shell started in the server user's home directory, and the client MUST select that tab. When `ship` opens a server that was already running, it MUST NOT create a tab, even if the server has none.
- ~~**FR-006:** A server whose tabs are removed MUST NOT create a tab on its own. FR-005 applies only when a client opens it.~~ Superseded by A-1.
- **FR-006 (A-1):** A server MUST create tabs on its own only at startup, from a starter it was given. `ship server` MUST start with no tabs, and `ship server --starter` with one tab as FR-005 describes. A running server with no tabs is valid and MUST keep running.
- **FR-007:** With nothing selected, the pane area MUST show the number of top-level tabs and the key that opens one, or, with no tabs, the CLI command that creates one. The status line MUST NOT show a session.
- **FR-008:** `C-b )` and `C-b (` MUST open the next and previous top-level tab, wrapping. With nothing selected they MUST open the first and last top-level tab.
- **FR-009:** When the viewed tab is removed, the client MUST fall back as selection repair does today. When nothing remains to fall back to, it MUST keep the attachment with nothing selected. An attachment MUST end only on detach or server shutdown.
- **FR-010:** `ship tab create` and `ship tab move` MUST accept an omitted parent, meaning the top level. `ship tab list` MUST print the top-level tabs and their descendants as JSON.
- **FR-011:** `ship server status` MUST print what bare `ship` prints today.
- **FR-012:** `ship attach`, the `ship session` commands, and the session routes MUST be removed.
- **FR-013:** `AGENTS.md` and `GLOSSARY.md` MUST describe the server's tabs as the top-level container. The locked papers' session text MUST be marked superseded by this paper, not rewritten.

## Success criteria

- **SC-001:** On Linux and macOS, every P1 story's verification passes using only the CLI and the client.
- **SC-002:** Nothing in the CLI, the routes, the client or the docs mentions sessions, except superseded text in archived papers.
- ~~**SC-003:** Two clients opened together on an empty server leave exactly one tab, across repeated tries.~~ Superseded by A-1.
- **SC-003 (A-1):** Opening a running server never changes its tabs: `ship tab list` reads the same before and after `ship`, including when it is empty.
- **SC-004:** Opening, cycling and detaching feel as responsive as attaching to a session does today.
- **SC-005:** Implementation Rust shrinks overall. The size report gives implementation and tests separately.
- **SC-006:** Checks not run on a platform are reported as unverified there; Windows is reported as unverified throughout.

Verification is through real workflows and temporary probes; this paper does not authorize permanent test code.

## Open unknowns

None. U-1, whether "session" survives as a user-facing word for a top-level tab, was resolved on 2026-10-08: it does not.

## Amendments

- **A-1 (2026-10-08):** The first tab comes from server startup. Cyan decided during architecture design that `ship server` takes the starting tabs and then never creates tabs on its own, and that an empty running server is a valid state, not one to repair. `ship server` starts empty by default; bare `ship` passes `--starter` when it starts the server. This replaces the open-an-empty-server rule in the Decided list, the second P1 "Start on an empty server" story, FR-005, FR-006 and SC-003. It also removes the need for clients to agree on who creates the first tab. A startup layout from config could later supply the starter; that is not part of this change.
