# observer-recovery Specification

## Purpose

Keep a running client useful across temporary connection loss by keeping its last screens, reconnecting automatically and resuming from current server state.

## Requirements

### Requirement: Report and reconnect
When a running client loses its connection, it SHALL keep showing the last screens, show a disconnected indicator in its status line and automatically retry, waiting between attempts with increasing delays of at most five seconds. Reconnecting SHALL NOT replay or repeat any mutation.

#### Scenario: Connection drops
- **WHEN** the network path between an attached client and a running server is cut
- **THEN** the client keeps its last screen, its status line shows that it is disconnected and reconnecting, and it keeps retrying at intervals no longer than five seconds

### Requirement: Resume with current state
After reconnecting, the client SHALL replace its view with the server's current state and screens, including every change made while it was disconnected, without requiring any user action.

#### Scenario: Edits during an outage
- **WHEN** edits are made and the selected pane prints output while a client is disconnected, and the connection is then restored
- **THEN** the client shows the current structure and the pane's current screen without any further action

### Requirement: Selection after reconnecting
On reconnecting, a client SHALL keep its previous selection if that tab or pane still exists anywhere on the server, and otherwise SHALL select nothing. Retention SHALL NOT depend on disk persistence. Reconnecting SHALL NOT create a tab.

#### Scenario: Selection still present
- **WHEN** a client reconnects and its selected pane still exists
- **THEN** the pane remains selected

#### Scenario: Selection removed or moved away
- **WHEN** a client reconnects after its selected pane's tab moved to the top level, and another client reconnects after its selected pane's tab was removed
- **THEN** the first keeps the pane selected, and the second selects nothing and shows the nothing-selected hint

### Requirement: Keys during an outage
While disconnected, the client SHALL discard keys and pastes the user produces. They SHALL NOT be delivered after the connection returns.

#### Scenario: Typing while disconnected
- **WHEN** a user types `ls` and Enter while the client is disconnected, and the connection is then restored
- **THEN** the pane never receives those keys
