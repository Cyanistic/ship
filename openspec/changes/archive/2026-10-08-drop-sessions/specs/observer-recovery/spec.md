# Spec Delta

## MODIFIED Requirements

### Requirement: Selection after reconnecting
On reconnecting, a client SHALL keep its previous selection if that tab or pane still exists anywhere on the server, and otherwise SHALL select nothing. Retention SHALL NOT depend on disk persistence. Reconnecting SHALL NOT create a tab.

#### Scenario: Selection still present
- **WHEN** a client reconnects and its selected pane still exists
- **THEN** the pane remains selected

#### Scenario: Selection removed or moved away
- **WHEN** a client reconnects after its selected pane's tab moved to the top level, and another client reconnects after its selected pane's tab was removed
- **THEN** the first keeps the pane selected, and the second selects nothing and shows the nothing-selected hint

## REMOVED Requirements

### Requirement: Session removed during an outage
**Reason**: There are no sessions, and reconnecting never fails because something was removed.
**Migration**: See "Selection after reconnecting": a removed selection becomes nothing selected.
