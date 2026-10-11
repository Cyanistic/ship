# Spec Delta

## MODIFIED Requirements

### Requirement: Exited programs
When a pane's program exits, the pane SHALL remain with its last screen and exit status until it is removed, and SHALL ignore keys, pastes and resizes. Its history SHALL still scroll and be readable, and mouse selection in it SHALL still work.

#### Scenario: Program exits with a code
- **WHEN** a pane's program runs `exit 3`
- **THEN** the pane remains, `ship pane get` reports it exited with code 3, and attached clients still show its last screen

#### Scenario: Read back a crash
- **WHEN** a pane's program prints a long error and exits
- **THEN** a user can scroll its history and `ship pane read --lines 100` prints the end of the error
