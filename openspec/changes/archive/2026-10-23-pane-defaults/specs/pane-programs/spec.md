## MODIFIED Requirements

### Requirement: Exited programs
When a pane's program exits, the server SHALL close the pane exactly as `ship pane close` would if its exit behavior is `close`, or if it is `keep_failed` and the program exited with code 0 and no signal. Otherwise the pane SHALL remain with its last screen and exit status until it is removed, and SHALL ignore keys, pastes and resizes. Its history SHALL still scroll and be readable, and mouse selection in it SHALL still work.

#### Scenario: Shell exits in a split
- **WHEN** a default shell pane beside another pane runs `exit`
- **THEN** the pane closes, its neighbor takes the space, and the selection moves there

#### Scenario: Lone pane exits
- **WHEN** a default shell pane that is the only pane in its tab runs `exit`
- **THEN** the pane closes and the tab stays, empty

#### Scenario: Program exits with a code
- **WHEN** a pane with exit behavior `keep` runs `exit 3`
- **THEN** the pane remains, `ship pane get` reports it exited with code 3, and attached clients still show its last screen

#### Scenario: Failures stay
- **WHEN** panes with exit behavior `keep_failed` run `sh -c 'exit 0'`, `sh -c 'exit 3'` and a `sleep` that is killed by a signal
- **THEN** the first pane closes and the other two remain with their exit status

#### Scenario: Read back a crash
- **WHEN** a pane with exit behavior `keep` prints a long error and exits
- **THEN** a user can scroll its history and `ship pane read --lines 100` prints the end of the error
