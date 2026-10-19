# Spec Delta

## ADDED Requirements

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
