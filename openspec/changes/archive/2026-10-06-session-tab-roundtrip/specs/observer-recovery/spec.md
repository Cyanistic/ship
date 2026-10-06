# Spec Delta

## Purpose

Keep a running text observer useful across temporary connection loss by reconnecting automatically and resuming from current server state.

## ADDED Requirements

### Requirement: Report and reconnect
When a running observer loses its connection, it SHALL report the disconnection on stderr and automatically retry, waiting between attempts with increasing delays of at most five seconds. Reconnecting SHALL NOT replay or repeat any mutation.

#### Scenario: Connection drops
- **WHEN** the network path between an attached observer and a running server is cut
- **THEN** the observer reports disconnection and keeps retrying at intervals no longer than five seconds

### Requirement: Resume with current state
After reconnecting, the observer SHALL replace its view with the server's current state, including every change made while it was disconnected, without requiring another edit.

#### Scenario: Edits during an outage
- **WHEN** edits are made while an observer is disconnected and the connection is then restored
- **THEN** the observer shows the current structure without any further edit

### Requirement: Selection after reconnecting
On reconnecting to its session, an observer SHALL keep its previous selection if that entity still belongs to the session, and otherwise SHALL select the session itself. This SHALL hold even when the selected entity moved to another session with its owning tab. Retention SHALL NOT depend on disk persistence. A newly started observer SHALL begin at the session itself.

#### Scenario: Selection still present
- **WHEN** an observer reconnects and its selected pane is still in its session
- **THEN** the pane remains selected

#### Scenario: Selection removed or moved away
- **WHEN** an observer reconnects after its selected pane was removed, or moved to another session with its tab
- **THEN** the observer selects its session

### Requirement: Session removed during an outage
Reconnection SHALL target the observer's session by ID and SHALL never recreate a removed session. If the session no longer exists when the observer reconnects, the observer SHALL report that the session was removed and exit with status 0.

#### Scenario: Session removed while disconnected
- **WHEN** an observer's session is removed during an outage and the connection is restored
- **THEN** the observer reports the removal and exits, and no session with that name is recreated
