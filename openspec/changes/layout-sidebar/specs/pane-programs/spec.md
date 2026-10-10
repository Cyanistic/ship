# Spec Delta

## MODIFIED Requirements

### Requirement: Program and starting directory
`ship pane create` SHALL start the configured shell, else the user's login shell, by default, in the CLI's current directory. `--tab ID` SHALL name a tab and `--pane ID` SHALL name a split anchor in its owning tab, defaulting to the pane the command runs in. `--cwd DIR` SHALL set the starting directory, and SHALL fail creation without changing anything when `DIR` is not an existing directory. `-- COMMAND ARGS...` SHALL run that command instead of the shell. `--name NAME` SHALL be optional.

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

### Requirement: Pane inspection
`ship pane get` SHALL report the pane's command, starting directory, current title, shell-reported current directory when available, and status, which is running or exited with its exit code. The title SHALL follow the title the program sets, and SHALL be absent when the program has set none or has cleared it. The reported current directory SHALL remain distinct from the starting directory and SHALL be absent until the shell reports one.

#### Scenario: Program sets a title
- **WHEN** a pane runs `sh -c 'printf "\033]0;hello\007"; sleep 100'`
- **THEN** `ship pane get` reports the title `hello` and the status running

#### Scenario: Shell reports a changed directory
- **WHEN** a pane starts in home, changes to `/tmp`, and its shell reports that directory through OSC 7
- **THEN** inspection shows home as the starting directory and `/tmp` as the reported current directory

#### Scenario: No current-directory report
- **WHEN** the shell has not reported its directory
- **THEN** inspection has no reported current directory and retains the starting directory

## ADDED Requirements

### Requirement: Key-created pane directory
A pane created from keys SHALL start in the selected anchor pane's shell-reported current directory when available, else its starting directory. Without an explicit command it SHALL run the configured shell, else the login shell. The new pane SHALL be selected. CLI creation SHALL continue using the caller's directory unless `--cwd` overrides it.

#### Scenario: Split after changing directory
- **WHEN** a pane's shell reports `/tmp` and a client presses `alt--`
- **THEN** a new shell starts below it in `/tmp` and is selected

#### Scenario: Starting-directory fallback
- **WHEN** the anchor has never reported a directory and a client creates a pane from keys
- **THEN** the new pane starts in the anchor's starting directory

#### Scenario: Caller overrides anchor directory
- **WHEN** a CLI caller in `/var` creates a pane anchored to a pane reporting `/tmp`, without `--cwd`
- **THEN** the new pane starts in `/var`
