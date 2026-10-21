# Spec Delta

## ADDED Requirements

### Requirement: Printing the defaults
`ship config defaults` SHALL print Ship's built-in defaults to stdout as the TOML file the client and server merge user files over, with its comments, and exit 0. It SHALL NOT read a config file or contact a server.

#### Scenario: Defaults as a starting point
- **WHEN** a user runs `ship config defaults > file.toml` and then `ship config check file.toml`
- **THEN** the check prints nothing and exits 0

#### Scenario: Broken config file
- **WHEN** the config file has errors and the user runs `ship config defaults`
- **THEN** the defaults print and the command exits 0
