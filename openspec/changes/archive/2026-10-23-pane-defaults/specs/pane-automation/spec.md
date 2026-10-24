## ADDED Requirements

### Requirement: Exit before close
When a pane's program exits and its exit behavior closes the pane, the server SHALL publish a revision with the pane showing its exit status before the revision that removes it. An `every` feed SHALL therefore always carry the exit before the close. This is the only ordering of revisions the server promises.

#### Scenario: Exit then close on an every feed
- **WHEN** a caller attaches with `{"feed": "every"}` and a default shell pane runs `exit 2`
- **THEN** the caller receives a `state` event where the pane has exited with code 2, followed by one where the pane is gone

### Requirement: Wait that falls behind
A `wait`, or the wait in `send --wait`, that falls too far behind the server to see every revision SHALL fail with an error saying it fell behind the server while waiting on the pane. It SHALL NOT report that the pane closed in that case.

#### Scenario: Lost feed
- **WHEN** a `wait`'s stream ends with reason `lost`
- **THEN** the wait exits 1 with an error naming the pane and saying it fell behind the server

## MODIFIED Requirements

### Requirement: Waiting on exit
`exited` SHALL match when the pane's program exits, and the printed pane SHALL carry its exit status, also when the pane's exit behavior then closes it. When the program exits and `exited` isn't in the condition, the wait SHALL fail with an error saying the program exited and giving its exit status, also when the pane then closes.

#### Scenario: Wait for exit
- **WHEN** a pane runs `cargo test` and a script runs `ship pane wait --pane <id> --status exited`
- **THEN** the wait returns when the program exits, and the printed pane shows its exit status

#### Scenario: Pane closes on exit
- **WHEN** a pane with exit behavior `close` runs `sh -c 'sleep 1; exit 4'` and a script runs `ship pane wait --pane <id> --status exited`
- **THEN** the wait prints the pane with exit code 4, and the pane is gone afterwards

#### Scenario: Either outcome
- **WHEN** a script runs `ship pane wait --pane <id> --status done,exited`
- **THEN** the wait returns on whichever happens first

#### Scenario: Unexpected exit
- **WHEN** a script runs `ship pane wait --pane <id>` and the program exits with code 1 while `working`
- **THEN** the wait exits 1 with an error naming the pane and saying the program exited with code 1, whether or not the pane then closes
