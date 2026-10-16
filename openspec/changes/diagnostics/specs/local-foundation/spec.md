# Spec Delta

## MODIFIED Requirements

### Requirement: End-user documentation
The README SHALL explain what Ship does at this checkpoint, prerequisites, build/run commands, target selection, background startup, normal shutdown, the default keys and the config file. For chord spelling it SHALL point to crokey instead of repeating it. The guide SHALL explain where server and client logs are, how to set and change the log level, and how to record a timeline. Internal implementation/update instructions and slice evidence SHALL remain in development planning rather than the README.

#### Scenario: Run from documented instructions
- **WHEN** a user follows the README for the completed checkpoint
- **THEN** they can build Ship, run the supported workflows and stop the server normally

#### Scenario: Find a log from the guide
- **WHEN** a user follows the guide after a background server misbehaves
- **THEN** they can open that server's current log, raise its level without a restart, and record a timeline

#### Scenario: Change a key from the README
- **WHEN** a user follows the README's config section to rebind a key
- **THEN** the binding works as described and `ship config check` accepts the file
