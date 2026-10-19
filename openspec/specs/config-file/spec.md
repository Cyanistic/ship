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
- **WHEN** a user adds `"alt-y" = { action = { server.tab.create = {} } }` under `[client.modes.normal.keys]` and saves while a client is attached
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
`ship config check [FILE]` SHALL check FILE, or the file chosen as in Config file location, the same way the client and server load it. It SHALL report the first error in each part, server and client, with the file, line and column, and exit non-zero when there is one, and exit 0 otherwise. It SHALL report no warnings.

#### Scenario: A mistake in each part
- **WHEN** a file has an unknown field under `[server]` and a misspelled action under `[client]`
- **THEN** `ship config check` prints both with their lines and columns and exits 1

#### Scenario: Valid file
- **WHEN** a file defines a mode that no binding enters and has no errors
- **THEN** `ship config check` prints nothing and exits 0

### Requirement: Sidebar width setting
`[client] sidebar_width` SHALL set the sidebar's configured width in columns, defaulting to 26. Labels SHALL NOT resize the sidebar. A client MAY replace the configured width with its own through a divider drag or `client.sidebar.resize`. That width SHALL apply only to that client, SHALL NOT be written to the file, and SHALL last until the client exits or the file's `sidebar_width` value changes; reloading a file whose `sidebar_width` is unchanged SHALL keep it. Either width SHALL stay between 10 columns and the width that leaves the pane area 20 columns, and when the terminal is too narrow for both limits the pane area's minimum SHALL win. The setting SHALL use the existing config selection, client reload and error-reporting contract; the server SHALL NOT read it as server configuration.

#### Scenario: Default width
- **WHEN** a client starts without a sidebar-width setting
- **THEN** its shown sidebar uses 26 columns when space permits

#### Scenario: Explicit width
- **WHEN** the config sets `[client] sidebar_width = 40` and is loaded by a client
- **THEN** the sidebar uses the configured 40 columns when space permits and the client reports the remaining pane area

#### Scenario: Label changes do not resize
- **WHEN** a pane title changes the derived label of a sidebar row
- **THEN** the sidebar width remains configured rather than fitting the new label

#### Scenario: Invalid type
- **WHEN** a file sets `[client] sidebar_width = "wide"` and the user runs `ship config check` on it
- **THEN** the command rejects it with the file and `client.sidebar_width` location

#### Scenario: Width reload
- **WHEN** a user changes the width from 26 to 40 and saves a valid config while attached
- **THEN** the client applies the new width without restart and reports its changed pane area

#### Scenario: Dragged width survives an unrelated save
- **WHEN** a user drags the sidebar wider and then saves a config change that leaves `sidebar_width` alone
- **THEN** the dragged width stays, and a later save that changes `sidebar_width` replaces it

#### Scenario: Narrow terminal
- **WHEN** a user drags the divider right on a 40-column terminal
- **THEN** the sidebar stops where the pane area keeps 20 columns

### Requirement: Server scrollback setting
`[server] scrollback_bytes` SHALL limit each pane's history in bytes, with a default of 10,000,000. The server SHALL read it when it starts a pane, so a saved change applies to panes created afterwards. When the file is bad, the server SHALL use the default and log the error.

#### Scenario: Smaller history for new panes
- **WHEN** a user sets `scrollback_bytes = 20000`, saves, and creates a pane on a running server
- **THEN** the new pane keeps about 20 KB of history and existing panes keep theirs

### Requirement: Client mouse setting
`[client] mouse` SHALL turn the client's mouse capture on or off, defaulting to `true`, and SHALL reload like other `[client]` settings.

#### Scenario: Default capture
- **WHEN** a user with no config runs `ship` and clicks a pane
- **THEN** the click selects the pane

### Requirement: Agent settings
`[client.agents]` SHALL configure agent-attention. `rows` SHALL be a list of lines, each a list of the tokens `icon`, `app`, `state`, `tab`, `pane`, `title` and `msg`, defaulting to `[["icon", "app", "state"], ["tab"]]`. `delay_ms` SHALL be the notification delay in milliseconds, defaulting to 1000. `popups` SHALL turn popups on or off, defaulting to off. An unknown token or key SHALL fail loading with its line, column and the accepted values, like every other config error.

#### Scenario: Misspelled token
- **WHEN** the file sets `rows = [["icon", "apps"]]` and the user runs `ship config check`
- **THEN** it reports `apps` with its line and column, lists the accepted tokens and exits 1

#### Scenario: No agents table
- **WHEN** the file has no `[client.agents]`
- **THEN** entries use the default rows, notifications wait 1 second, and no popups are written

### Requirement: Agent settings reload
The agent settings, sound settings included, SHALL reload with the rest of `[client]`, reading sound files again. A `[client]` that fails to load SHALL leave the running agent settings in place.

#### Scenario: Custom rows on reload
- **WHEN** a user saves `rows = [["icon", "app"], ["title"]]` while a client is attached
- **THEN** agent entries show those lines without a restart

#### Scenario: Broken save
- **WHEN** a user saves `delay_ms = "soon"` while a client is attached
- **THEN** the client keeps its running agent settings and shows the error with its location

### Requirement: Agent sound settings
`[client.agents.sounds.default]` and each `[client.agents.sounds.apps.<app>]`, keyed by the reported `app`, SHALL each accept `done` and `attention`, set to `"off"`, `{ builtin = "done" }`, `{ builtin = "attention" }` or `{ path = "<file>" }`. An unset slot SHALL fall back to the default table's, then to the bundled sound for its notification. An unknown builtin name, sound form or key SHALL fail loading with its line and column.

#### Scenario: Explicit defaults
- **WHEN** the file sets `done = { builtin = "done" }` and `attention = { builtin = "attention" }` under `[client.agents.sounds.default]`
- **THEN** the client plays the same sounds as with no sound settings

#### Scenario: App override falls through
- **WHEN** the file sets only `done = "off"` under `[client.agents.sounds.apps.droid]`
- **THEN** Droid finishing plays nothing and Droid asking for approval plays the default attention sound

### Requirement: Sound paths
A sound `path` SHALL resolve relative to the config file's directory, with a leading `~` expanded to the home directory. A sound file that is missing or can't be decoded SHALL NOT fail loading, and `ship config check` SHALL NOT report it; agent-attention's Sound failures covers it.

#### Scenario: Sound file relative to the config
- **WHEN** the config file is `~/.config/ship/config.toml` and sets `done = { path = "sounds/ding.mp3" }` under `[client.agents.sounds.default]`
- **THEN** finishing agents play `~/.config/ship/sounds/ding.mp3`

#### Scenario: Missing file passes the check
- **WHEN** the file sets `done = { path = "nope.mp3" }` and the file doesn't exist
- **THEN** `ship config check` exits 0
