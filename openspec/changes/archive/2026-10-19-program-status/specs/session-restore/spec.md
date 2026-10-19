# Spec Delta

## MODIFIED Requirements

### Requirement: Nothing live is saved
The saved tabs SHALL NOT hold a pane's program or arguments, its title, its exit status, its status records, its geometry, or its screen and scrollback.

#### Scenario: Program arguments stay off disk
- **WHEN** a pane runs `sleep 1234` and the saved tabs are written
- **THEN** the saved file does not contain `1234`

#### Scenario: Status records stay off disk
- **WHEN** a pane's program has reported `working` and the saved tabs are written
- **THEN** the saved file has no `program` field, and after a restart the restored pane has no records
