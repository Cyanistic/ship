# Spec Delta

## ADDED Requirements

### Requirement: Shell completion scripts
`ship completions SHELL` SHALL print a tab-completion script for the named shell to stdout and exit 0, for bash, zsh, fish, elvish and PowerShell. The script SHALL be generated from the CLI's own definitions, SHALL need no server and SHALL read no config file. An unsupported shell name SHALL be an argument error.

#### Scenario: Print a completion script
- **WHEN** a user runs `ship completions zsh` with no server running
- **THEN** a zsh completion script for `ship` is printed to stdout and the command exits 0 without starting a server

#### Scenario: Unsupported shell
- **WHEN** a user runs `ship completions` with a shell name Ship doesn't support
- **THEN** the argument parser reports the supported shells and the command exits 2

## MODIFIED Requirements

### Requirement: Buildable application and command help
The project SHALL provide a buildable Ship executable with help and version output that do not require a running server. Help SHALL describe bare `ship` opening the client and `alt-q` detaching, `ship server status`, explicit server startup with `--starter`, custom target selection, `--config`, `ship config check`, `ship config defaults` and `ship completions`, and automatic background-server persistence.

#### Scenario: Foundation help without a server
- **WHEN** a developer builds the foundation and runs `ship --help` or `ship --version` with no server running
- **THEN** the requested output is displayed and the command exits successfully without starting a server

#### Scenario: Command discovery
- **WHEN** a user requests help
- **THEN** help describes `ship`, `ship server status`, `ship --server-url URL`, `ship --config FILE`, `ship config check`, `ship config defaults`, `ship completions SHELL`, `ship server [--port PORT] [--starter]` and that automatic startup leaves the server running
