# Spec Delta

## ADDED Requirements

### Requirement: Scroll indicator
A pane scrolled up from the bottom SHALL show how far it is from the bottom, on its border, or at its content's top right when it has no border. A pane at the bottom SHALL show no indicator. Its exact wording is settled in rendering review.

#### Scenario: Indicator appears and goes
- **WHEN** a user scrolls a bordered shell pane up one page, then types a key
- **THEN** the pane's border shows its distance from the bottom, and the indicator disappears once input returns it to the bottom

## MODIFIED Requirements

### Requirement: Terminal restoration
The client SHALL restore the user's terminal to its state before opening on every exit path, including detach, server shutdown, panics and signals it can handle. Restoring SHALL include turning mouse capture off.

#### Scenario: Client killed
- **WHEN** the client receives SIGTERM, or panics in a temporary probe build
- **THEN** the outer terminal is usable without running `reset`

#### Scenario: Mouse released on detach
- **WHEN** a user detaches with `alt-q` and drags in the outer terminal
- **THEN** the outer terminal selects text and no mouse escape sequences appear at the shell prompt
