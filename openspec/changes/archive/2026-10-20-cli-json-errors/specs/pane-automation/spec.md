# Spec Delta

## MODIFIED Requirements

### Requirement: Automation errors
Errors from `send` and `wait` SHALL follow the CLI error line in the `health-exchange` capability: one JSON error object as the last line on stderr and exit status 1, with usage errors keeping exit status 2. Each failure SHALL have its own message naming the pane: not found, closed, blocked, in a state `--if` doesn't list, no program status before sending, exited, stalled, the server stopped, and the connection lost. On `send --wait`, the outermost error's `message` SHALL be exactly `nothing was sent` for failures before writing, `the input may have been sent` when writing fails, and `the input was sent` for failures after writing, with the specific failure under `causedBy`.

#### Scenario: Server stops during a wait
- **WHEN** `ship server stop` runs while a script's `wait` is waiting
- **THEN** the wait exits 1 with an error saying the server stopped while waiting on the pane

#### Scenario: Refused before writing
- **WHEN** a script runs `send --wait` on a `blocked` pane without `--if blocked`
- **THEN** the last line on stderr has `.message` equal to `nothing was sent` and the exit status is 1
