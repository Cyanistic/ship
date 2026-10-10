# Spec Delta

## MODIFIED Requirements

### Requirement: Selection after reconnecting
On reconnecting, a client SHALL keep its previous selection if that tab or pane still exists anywhere on the server, subject to shared zoom repair onto the zoomed pane when the previous pane is hidden, and otherwise SHALL select nothing. Retention SHALL NOT depend on disk persistence. Reconnecting SHALL NOT create a tab.

#### Scenario: Selection still present
- **WHEN** a client reconnects and its selected pane still exists and is not hidden by shared zoom
- **THEN** the pane remains selected

#### Scenario: Selection removed or moved away
- **WHEN** a client reconnects after its selected pane's tab moved to the top level, and another client reconnects after its selected pane's tab was removed
- **THEN** the first keeps the pane selected unless shared zoom hides it, and the second selects nothing and shows the nothing-selected hint

#### Scenario: Zoom changed while disconnected
- **WHEN** a client reconnects with a surviving pane selection hidden by its tab's shared zoom
- **THEN** the server repairs it to the zoomed pane and the client shows that pane without replaying a mutation
