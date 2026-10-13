# session-observation Specification

## Purpose

Let independently running full-screen clients open the whole server, see its live state and pane screens, and keep their own selections without affecting each other.

## Requirements

### Requirement: Attach a client
Bare `ship` SHALL attach a full-screen client to the whole server, as described by the `terminal-client` capability. On attach it SHALL show the server's current state and, once a tab is viewed, the current screens of all its visible panes, including changes made while it was attaching, without waiting for another change. It SHALL update after each later change. `ship attach` SHALL no longer exist.

#### Scenario: Attach during edits
- **WHEN** a user opens `ship` while another process is creating top-level tabs
- **THEN** the client's state includes every created tab without any further edit

#### Scenario: Quiet pane on attach
- **WHEN** a user opens `ship`, presses `alt-right`, and the opened tab's panes show quiet shell prompts
- **THEN** the client shows all their visible prompts at once

#### Scenario: Two observers converge
- **WHEN** two clients view the same pane and a separate CLI process edits the tree
- **THEN** both clients show the same resulting shared state and screens

### Requirement: First tab from server startup
When bare `ship` starts the local server itself, the server SHALL begin with one top-level tab holding one pane running the user's login shell in the server user's home directory, and the client SHALL open with that tab's pane selected. Opening a server that was already running SHALL NOT create a tab, even when it has none.

#### Scenario: Start on a stopped server
- **WHEN** no server is running and a user runs `ship` from a project directory
- **THEN** a server starts with one tab, and the client opens on its shell, whose working directory is home

#### Scenario: Open an empty running server
- **WHEN** a running server has no tabs and a user runs `ship`
- **THEN** the client shows `no tabs · ship tab create`, and `ship tab list` reads the same before and after

#### Scenario: Two clients start the server together
- **WHEN** two `ship` commands run at the same time with no server running
- **THEN** exactly one tab exists and both clients open on it

### Requirement: Independent optional selection
Each attached client SHALL have its own selection: a tab, a pane, or nothing. A newly opened client SHALL select nothing, except as "First tab from server startup" describes. A client SHALL be able to select an empty tab or a pane. Changing one client's selection SHALL NOT change another's, except for the shared zoom repair rule. Each client SHALL show its own selection. Sidebar folds and pane-selection history SHALL stay client-local.

#### Scenario: Open with nothing selected
- **WHEN** a server has three top-level tabs and a user runs `ship`
- **THEN** the client selects nothing and shows `3 tabs · alt-right to open one`

#### Scenario: Different selections
- **WHEN** two clients are on different panes in an unzoomed tab and one presses `alt-tab`
- **THEN** only that client's selection changes, and both still see all visible panes

### Requirement: Selection repair
When a client's selected pane is removed, its selection SHALL move into the sibling subtree that takes the pane's space, choosing the leaf toward the closed side on same-axis splits and the first child otherwise. With no own panes left it SHALL fall back to the tab. Removed tabs or ancestors SHALL repair to the nearest surviving ancestor, or nothing. Moved surviving selections SHALL stay selected. A selection hidden by shared zoom SHALL repair to the zoomed pane.

#### Scenario: Selected pane removed
- **WHEN** A is selected in `[A | [B / C]]` and A is removed
- **THEN** the client's selection moves to B in the subtree taking A's space

#### Scenario: Last pane in a tab removed
- **WHEN** a client's selected pane is the only pane in its tab and is removed
- **THEN** its selection becomes the tab and the client shows the no-pane hint

#### Scenario: Selected tab's ancestor removed
- **WHEN** an ancestor of a client's selected tab is removed
- **THEN** its selection becomes the nearest surviving ancestor, or nothing if none survives

#### Scenario: Selected tab moved
- **WHEN** a client's selected pane's tab moves to the top level
- **THEN** the client keeps that pane selected

#### Scenario: Shared zoom hides selection
- **WHEN** one client zooms A while another viewing that tab has B selected
- **THEN** the other client's selection repairs to A

### Requirement: Attachments outlive tabs
An attachment SHALL end only when its client detaches or the server shuts down. Removing tabs, including every tab, SHALL NOT end it.

#### Scenario: Remove every tab
- **WHEN** a client is attached and a user removes every top-level tab
- **THEN** the client stays attached and shows `no tabs · ship tab create`

#### Scenario: Tabs return
- **WHEN** a client shows no tabs and a user then runs `ship tab create` twice
- **THEN** the client's hint updates to `2 tabs · alt-right to open one` without reattaching

### Requirement: Latest state for slow observers
A client SHALL never be shown state or a screen older than what it has already shown. A client that stops reading SHALL, once it resumes, receive the latest state and the latest screen of each pane it shows, even if no further change occurs. Intermediate states and screens are not guaranteed to be shown.

#### Scenario: Stalled observer resumes
- **WHEN** a client process is suspended, fifty edits are made and its pane prints more output, and the process is resumed
- **THEN** it shows the final state and the pane's final screen without any further change

### Requirement: Compressed observation stream
The observation stream SHALL be served as server-sent events with negotiated zstd compression, with gzip available, and each event SHALL reach the observer without waiting for the stream to end.

#### Scenario: zstd negotiated
- **WHEN** a client requests the observation stream accepting zstd
- **THEN** the response is zstd-encoded and individual events arrive while the stream stays open

### Requirement: Shutdown ends observation
Server shutdown by SIGINT or SIGTERM SHALL end open attach streams so that the server still exits within its five-second shutdown bound. Attached clients SHALL restore the terminal, report that the server stopped and exit.

#### Scenario: Shutdown with observers attached
- **WHEN** the server receives SIGTERM while two clients are attached
- **THEN** it exits within five seconds, and both clients restore their terminals, report that the server stopped and exit

### Requirement: Cycle top-level tabs
`client.tab.next` and `client.tab.prev` SHALL select the next/previous visible sidebar row in tree order, wrapping around; from nothing they SHALL select the first/last visible row. Rows under expanded ancestors SHALL count even with the sidebar hidden. Landing SHALL use the tab's zoomed pane, else the client's last selected own pane there, else its first own pane in layout order, else the tab. The server SHALL accept selection of any existing tab, pane or nothing.

#### Scenario: Cycle tabs
- **WHEN** three visible top-level rows have no children and a client on the first presses `alt-right` three times
- **THEN** the client visits each row's landing selection and returns to the first

#### Scenario: From nothing selected
- **WHEN** a client has nothing selected and presses `alt-left`
- **THEN** the last visible row is opened using its landing selection

#### Scenario: Tab without panes
- **WHEN** a client cycles to a tab with no own panes but a child with panes
- **THEN** the parent tab itself is selected and the no-pane hint is shown, rather than a child's pane

#### Scenario: Return to a worked-in tab
- **WHEN** a client leaves a tab after selecting its second pane and later cycles back
- **THEN** it selects that surviving pane again, independently of other clients' histories

#### Scenario: Hidden sidebar navigation
- **WHEN** a client hides its sidebar and invokes tab-next in normal mode
- **THEN** it moves through the same expanded rows as when the sidebar was visible

### Requirement: Local folding and selected-row visibility
Fold state SHALL be independent for each client. `client.tab.collapse` and `client.tab.expand` SHALL fold/unfold the selected tab's row in any mode where bound. Selecting a descendant by any means SHALL expand its ancestors, so the selected row is visible. Folding SHALL NOT introduce a separate cursor or change the selected tab to a child.

#### Scenario: Two clients fold differently
- **WHEN** two clients view the same tree and one folds a tab
- **THEN** the other client's folds and selection are unchanged

#### Scenario: Open a folded descendant
- **WHEN** a client selects a tab beneath a folded ancestor
- **THEN** that ancestor expands and the selected tab's row becomes visible

#### Scenario: Tabs mode uses the selection
- **WHEN** a client enters default `tabs` mode and presses `j`, `h`, `l` and `esc`
- **THEN** `j` moves the selection to the next visible row, `h` and `l` fold/unfold that row, and `esc` returns to normal on the same selection without Enter

### Requirement: Published pane geometry
The server SHALL publish each viewed tab's size and visible pane frame/content rectangles in layout order, relative to the tab area, as optional `geometry` on that tab recursively. Unviewed tabs SHALL omit geometry; a viewed empty tab SHALL include its size and an empty pane map. `Replica` SHALL NOT contain a parallel geometry map. Geometry SHALL be derived from source layouts and current viewers when publishing, not independently mutable or cached server state. Tab list/get/create/rename/move responses SHALL use the same published-tab semantics. PTY sizing SHALL consume the committed published tab geometry. Clients SHALL render and resolve neighbors from that geometry rather than compute a second layout. Geometry SHALL change with shared layout edits and reported viewing areas.

#### Scenario: Inspect a split tab through observation
- **WHEN** a caller observes a viewed tab after splitting it
- **THEN** the published geometry describes the same content sizes that its visible programs receive

#### Scenario: Viewed child under unviewed ancestors
- **WHEN** a viewer selects a pane in the grandchild of an unviewed tab
- **THEN** the nested grandchild carries its own geometry in the same replica as its layout and viewer record, and its unviewed ancestors omit geometry

#### Scenario: Last viewer leaves a tab
- **WHEN** the last viewer of a tab selects another tab or detaches
- **THEN** subsequent publications and tab inspection omit that tab's geometry while its programs retain their last terminal sizes

### Requirement: Directional pane focus
`client.pane.focus` SHALL select a pane sharing the requested side of the selected pane's frame. Multiple candidates SHALL resolve to the one this client selected most recently, else the nearest center, with remaining ties resolved by layout order. Pane-selection history SHALL NOT be sent to the server.

#### Scenario: Adjacent candidates
- **WHEN** A is left of B above C, the client last selected C, and it focuses right from A
- **THEN** C is selected and subsequent input goes to C

#### Scenario: No recency for a tie
- **WHEN** a client has no selection history among equally adjacent candidates
- **THEN** the nearest center wins, with equal distances resolved by layout order

### Requirement: Shared zoom
`server.pane.zoom` and `ship pane zoom` SHALL toggle the targeted pane to fill its tab for all viewers while other programs keep running. Pane focus or next from the zoomed pane SHALL unzoom before moving, and a directional client swap SHALL unzoom before resolving its neighbor, even when none is found. Creating, closing, swapping or resizing a pane SHALL unzoom that tab before the edit. Selecting another tab SHALL leave zoom unchanged.

#### Scenario: Two viewers zoom together
- **WHEN** one client toggles zoom on its selected pane
- **THEN** both viewers see that pane alone and hidden selected panes repair to it

#### Scenario: Move out of zoom
- **WHEN** two clients concurrently request directional focus out of a zoomed pane
- **THEN** the tab becomes unzoomed, does not toggle back into zoom, and each client moves using unzoomed geometry

#### Scenario: Layout edit ends zoom
- **WHEN** a valid create, close, swap or resize operation is performed in a zoomed tab
- **THEN** the tab unzooms and all viewers see the edited layout

#### Scenario: Visit another tab
- **WHEN** a client leaves a zoomed tab to view a different tab
- **THEN** the original tab stays zoomed for its other viewers and for a later return

### Requirement: Who changes a selection
The server SHALL hold each attachment's selection. It SHALL change only through its own client's navigation and clicks, a select naming that attachment, a create that selects for it, and Selection repair. A client SHALL show a selection set by someone else as it shows its own, within a frame of the server publishing it.

#### Scenario: A script steers a client
- **WHEN** a client is attached and a second terminal runs `ship view select --attachment <id> <pane>`
- **THEN** the client shows `<pane>` selected on its next frame, and typing goes to it

#### Scenario: Another client's create
- **WHEN** one client presses `alt-n` while a second client has a pane selected
- **THEN** the second client's selection doesn't move

### Requirement: Area-only view updates
`PUT /api/v0/attach/view` SHALL carry only the attachment's terminal `area`, with the attachment named by `x-ship-attachment-id`. Updating the area SHALL NOT replace its selection; selection changes SHALL use the selection route or a selecting create, apart from attachment initialization and server repair.

#### Scenario: Resize after an external selection
- **WHEN** a script selects a pane for an attached client and that client resizes its terminal
- **THEN** the reported area changes and the scripted selection remains selected

### Requirement: Views routes
`GET /api/v0/views` SHALL return every attachment's viewing record as a JSON object keyed by attachment ID, in attach order, with the same records the observation stream publishes under `viewers`. `GET /api/v0/views/{attachment}` SHALL return one record. `PUT /api/v0/views/{attachment}/selection` with `{"selection": <tab or pane ID>}` or `{"selection": null}` SHALL set that attachment's selection and return the record as stored. An unknown attachment or node SHALL get 404 and change nothing.

#### Scenario: Read every view
- **WHEN** two clients are attached and a caller sends `GET /api/v0/views`
- **THEN** it receives an object with both attachment IDs, each holding that client's selection, when it has one, and area

#### Scenario: Select nothing
- **WHEN** a caller sends `PUT /api/v0/views/<id>/selection` with `{"selection": null}`
- **THEN** it receives the record without a selection, and that client shows the nothing-selected hint

#### Scenario: Unknown node
- **WHEN** a caller selects a pane ID that doesn't exist for an attached client
- **THEN** it receives 404 and the client's selection is unchanged

#### Scenario: Unknown attachment
- **WHEN** a caller sends `GET /api/v0/views/<id>` for an attachment that has ended
- **THEN** it receives 404

### Requirement: View commands
`ship view list` SHALL print every attachment's view as JSON keyed by attachment ID. `ship view selected --attachment ID` SHALL print that attachment's selection as a JSON string, or `null`. `ship view select --attachment ID NODE` SHALL select a tab or pane for it, and `--none` instead of `NODE` SHALL select nothing. `--attachment` SHALL default to `SHIP_ATTACHMENT_ID`.

#### Scenario: Find and steer a client
- **WHEN** a user runs `ship view list`, takes an attachment ID from it, and runs `ship view select --attachment <id> <tab>`, then `ship view selected --attachment <id>`
- **THEN** that client has the tab itself selected, and the last command prints the tab's ID

#### Scenario: Neither a node nor --none
- **WHEN** a user runs `ship view select --attachment <id>` with no node and no `--none`
- **THEN** the CLI rejects the arguments before any request, exits 2, and the selection is unchanged

#### Scenario: Unknown attachment or node
- **WHEN** a user runs `ship view select` naming an attachment or a node that doesn't exist
- **THEN** the command fails with a not-found error, exits 1 and nothing changes

### Requirement: Select behind a zoom
Selecting a pane hidden by its tab's zoom, through the views route or `ship view select`, SHALL unzoom that tab for every viewer and select the pane, in one edit.

#### Scenario: Script selects a hidden pane
- **WHEN** A is zoomed in a tab with B, two clients view the tab, and a script selects B for one of them
- **THEN** both clients see the tab unzoomed, and the script's client has B selected
