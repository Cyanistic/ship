# Spec Delta

## MODIFIED Requirements

### Requirement: Cycle top-level tabs
`client.tab.next` (by default `alt-right`) SHALL select the next top-level tab and `client.tab.prev` (by default `alt-left`) the previous one, in order, wrapping around. With nothing selected they SHALL select the first and last top-level tab. Selecting a tab this way SHALL select its first pane in tree order, or the tab itself when it has no panes. The server SHALL accept a client's change of selection to any existing tab or pane, or to nothing.

#### Scenario: Cycle tabs
- **WHEN** three top-level tabs exist and a client on the first presses `alt-right` three times
- **THEN** the client shows each tab's first pane in turn and returns to the first

#### Scenario: From nothing selected
- **WHEN** a client has nothing selected and presses `alt-left`
- **THEN** the last top-level tab's first pane is selected

#### Scenario: Tab without panes
- **WHEN** a client cycles to a top-level tab with no panes
- **THEN** the tab itself is selected and the client shows the no-pane hint
