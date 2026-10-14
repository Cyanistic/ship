# Spec Delta

## ADDED Requirements

### Requirement: Published mouse tracking
Each published pane screen SHALL carry `mouse: true` while the pane's program has asked for mouse input, and SHALL omit `mouse` otherwise. The flag SHALL follow the program's mouse mode as of its latest output and SHALL be terminal state, not shared tab or pane state: a change to it SHALL NOT change the shared state's revision. Clients SHALL use it to decide where a right-click goes, as mouse-input's Right-click passthrough describes.

#### Scenario: Neovim turns the mouse on and off
- **WHEN** an observer watches a pane that starts Neovim with `mouse=a`, then quits it
- **THEN** the pane's screens carry `mouse: true` while Neovim runs, and omit `mouse` at the shell prompt before and after

#### Scenario: Pager without the mouse
- **WHEN** a pane runs `less` on a long file, then `less --mouse`
- **THEN** the first omits `mouse`, and the second carries `mouse: true`

## MODIFIED Requirements

### Requirement: Published pane geometry
The server SHALL publish each viewed tab's size and visible pane frame/content rectangles in layout order, relative to the tab area, as optional `geometry` on that tab recursively. Unviewed tabs SHALL omit geometry; a viewed empty tab SHALL include its size and an empty pane map. `Replica` SHALL NOT contain a parallel geometry map. Geometry SHALL be derived from source layouts and current viewers when publishing, not independently mutable or cached server state. Tab list/get/create/set/move responses SHALL use the same published-tab semantics. PTY sizing SHALL consume the committed published tab geometry. Clients SHALL render and resolve neighbors from that geometry rather than compute a second layout. Geometry SHALL change with shared layout edits and reported viewing areas.

#### Scenario: Inspect a split tab through observation
- **WHEN** a caller observes a viewed tab after splitting it
- **THEN** the published geometry describes the same content sizes that its visible programs receive

#### Scenario: Viewed child under unviewed ancestors
- **WHEN** a viewer selects a pane in the grandchild of an unviewed tab
- **THEN** the nested grandchild carries its own geometry in the same replica as its layout and viewer record, and its unviewed ancestors omit geometry

#### Scenario: Last viewer leaves a tab
- **WHEN** the last viewer of a tab selects another tab or detaches
- **THEN** subsequent publications and tab inspection omit that tab's geometry while its programs retain their last terminal sizes

