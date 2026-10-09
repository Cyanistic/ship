# config-file Specification

## Purpose

Let users configure Ship in one TOML file that the client and the server each read for their own part, with errors that name their location and client changes that apply on save.

## Requirements

### Requirement: Config file location
Ship SHALL read its config from `--config <file>` if given, else from the file named by `SHIP_CONFIG`, else from `ship/config.toml` in the platform's config directory. No other environment variable SHALL set config values. With no config file, Ship SHALL run on the defaults. A server that `ship` starts in the background SHALL read the same file as the command that started it.

#### Scenario: No config file
- **WHEN** no config file exists and a user runs `ship`
- **THEN** the client opens with the default keys and new panes run the login shell

#### Scenario: File from the environment
- **WHEN** no server is running and a user runs `SHIP_CONFIG=/tmp/ship.toml ship` with `shell = "/bin/sh"` under `[server]` in that file
- **THEN** a pane created without a command runs `/bin/sh`

### Requirement: Client and server parts
The file's top level SHALL hold only `[server]` and `[client]`. The server SHALL read only `[server]`, and the client SHALL read only `[client]`. An error in one part SHALL NOT affect the process that reads the other. Unknown top-level tables, unknown fields, unknown actions and unknown action arguments SHALL be errors. Relative paths in the file SHALL resolve from the file's folder.

#### Scenario: Typo in a top-level table
- **WHEN** the file has a `[srever]` table
- **THEN** the client reports it as an unknown table and `ship config check` exits non-zero

#### Scenario: Client error leaves the server alone
- **WHEN** the file has a valid `[server]` shell and an unknown action under `[client]`
- **THEN** new panes still run the configured shell

### Requirement: Server shell setting
`[server] shell` SHALL name the program a pane runs when created without a command, started as a login shell. The server SHALL read it each time it starts such a pane, so a saved change applies to the next pane. When the file is bad, the server SHALL use the login shell and log the error.

#### Scenario: Change applies to the next pane
- **WHEN** a user sets `shell = "/bin/sh"`, saves, and creates a pane without a command on a running server
- **THEN** the new pane runs `/bin/sh` as a login shell, and existing panes are unchanged

#### Scenario: Broken file
- **WHEN** the file has a syntax error and a user creates a pane without a command
- **THEN** the pane runs the login shell and the server log records the error

### Requirement: Client reload
A client SHALL reload `[client]` when the config file is saved, including when an editor replaces the file by rename and when the path is a symlink. The `client.config.reload` action SHALL also reload it.

#### Scenario: Save while attached
- **WHEN** a user adds `"alt-y" = { server.tab.create = {} }` under `[client.modes.normal.keys]` and saves while a client is attached
- **THEN** `alt-y` creates a tab without a restart, and `alt-n` still works

#### Scenario: Rename-save through a symlink
- **WHEN** the config path is a symlink into another directory and the target is saved by an editor that writes a new file and renames it into place
- **THEN** the attached client applies the change

### Requirement: Bad client config
A client that starts with a bad `[client]` SHALL run on the default keys and show the error. A client whose `[client]` becomes bad on reload SHALL keep its running keys and show the error.

#### Scenario: Broken save
- **WHEN** a user saves the file with an unknown action while a client is attached
- **THEN** the client keeps the keys it had and shows the error with its location

#### Scenario: Broken at startup
- **WHEN** a user runs `ship` with an unknown action in the file
- **THEN** the client opens with the default keys and shows the error

### Requirement: Config check
`ship config check [FILE]` SHALL check FILE, or the file chosen as in Config file location, the same way the client and server load it. It SHALL report every error with the file and its location and exit non-zero when there is one, and exit 0 otherwise. It SHALL report no warnings.

#### Scenario: Several mistakes
- **WHEN** a file has a misspelled action, an unknown field and a binding to an undefined mode
- **THEN** `ship config check` names all three with their key paths and exits 1

#### Scenario: Valid file
- **WHEN** a file defines a mode that no binding enters and has no errors
- **THEN** `ship config check` prints nothing and exits 0
