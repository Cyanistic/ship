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
On reconnecting to its session, a client SHALL keep its previous selection if that entity still belongs to the session, and otherwise SHALL select the session's first pane in tree order, or the session itself when it has no panes. This SHALL hold even when the selected entity moved to another session with its owning tab. Retention SHALL NOT depend on disk persistence. A newly started client SHALL begin at the session's first pane in tree order, or the session itself when it has no panes.

#### Scenario: Selection still present
- **WHEN** a client reconnects and its selected pane is still in its session
- **THEN** the pane remains selected

#### Scenario: Selection removed or moved away
- **WHEN** a client reconnects after its selected pane was removed, or moved to another session with its tab
- **THEN** the client selects its session's first pane in tree order, or the session itself when no panes remain

### Requirement: Session removed during an outage
Reconnection SHALL target the client's session by ID and SHALL never recreate a removed session. If the session no longer exists when the client reconnects, the client SHALL restore the terminal, report that the session was removed and exit with status 0.

#### Scenario: Session removed while disconnected
- **WHEN** a client's session is removed during an outage and the connection is restored
- **THEN** the client restores the terminal, reports the removal and exits, and no session with that name is recreated

### Requirement: Keys during an outage
While disconnected, the client SHALL discard keys and pastes the user produces. They SHALL NOT be delivered after the connection returns.

#### Scenario: Typing while disconnected
- **WHEN** a user types `ls` and Enter while the client is disconnected, and the connection is then restored
- **THEN** the pane never receives those keys
