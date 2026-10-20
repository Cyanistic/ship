# Spec Delta

## MODIFIED Requirements

### Requirement: Config check
`ship config check [FILE]` SHALL check FILE, or the file chosen as in Config file location, the same way the client and server load it. It SHALL report the first error in each part, server and client, as text with the file, line and column, and exit 1 when there is one, without printing any further error line. It SHALL exit 0 otherwise. It SHALL report no warnings.

#### Scenario: A mistake in each part
- **WHEN** a file has an unknown field under `[server]` and a misspelled action under `[client]`
- **THEN** `ship config check` prints both with their lines and columns, prints nothing after them, and exits 1

#### Scenario: Valid file
- **WHEN** a file defines a mode that no binding enters and has no errors
- **THEN** `ship config check` prints nothing and exits 0
