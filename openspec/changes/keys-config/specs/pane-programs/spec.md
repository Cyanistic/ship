# Spec Delta

## MODIFIED Requirements

### Requirement: Program and starting directory
`ship pane create` SHALL start the configured shell, else the user's login shell, by default, in the CLI's current directory. `--tab ID` SHALL name the tab, defaulting to the tab holding the pane the command runs in. `--cwd DIR` SHALL set the starting directory, and SHALL fail creation without changing anything when `DIR` is not an existing directory. `-- COMMAND ARGS...` SHALL run that command instead of the shell. `--name NAME` SHALL be optional.

#### Scenario: Default shell in the current directory
- **WHEN** a user runs `ship pane create --tab <tab>` from `/tmp` with no shell configured
- **THEN** the pane runs the user's login shell, and `ship pane get --pane <id>` reports `/tmp` as its starting directory

#### Scenario: Configured shell
- **WHEN** `[server] shell` is `/bin/sh` and a user runs `ship pane create --tab <tab>`
- **THEN** the pane runs `/bin/sh` as a login shell

#### Scenario: Explicit command and directory
- **WHEN** a user runs `ship pane create --tab <tab> --cwd /var -- sh -c 'sleep 100'`
- **THEN** the pane runs `sh -c 'sleep 100'` starting in `/var`

#### Scenario: Missing directory
- **WHEN** a user passes `--cwd` naming a directory that does not exist
- **THEN** pane creation fails and nothing changes
