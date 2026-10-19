# Spec Delta

## MODIFIED Requirements

### Requirement: Pane inspection
`ship pane get` SHALL report the pane's command, starting directory, current title, current directory when known, status, which is running or exited with its exit code, and its status records as program-status describes. The title SHALL follow the title the program sets, and SHALL be absent when the program has set none or has cleared it. The current directory SHALL be the directory the shell last reported through OSC 7, else, on Linux and macOS, the program's working directory as of its latest output. It SHALL remain distinct from the starting directory and SHALL be absent until known.

#### Scenario: Program sets a title
- **WHEN** a pane runs `sh -c 'printf "\033]0;hello\007"; sleep 100'`
- **THEN** `ship pane get` reports the title `hello` and the status running

#### Scenario: Shell reports a changed directory
- **WHEN** a pane starts in home, changes to `/tmp`, and its shell reports that directory through OSC 7
- **THEN** inspection shows home as the starting directory and `/tmp` as the current directory

#### Scenario: Shell that doesn't report its directory
- **WHEN** a pane's shell on Linux or macOS never reports OSC 7 and the user runs `cd /usr/share`
- **THEN** inspection shows `/usr/share` as the current directory once the next prompt is drawn, and retains the starting directory

#### Scenario: No current directory known
- **WHEN** the shell has not reported its directory and the platform offers no working-directory lookup
- **THEN** inspection has no current directory and retains the starting directory

#### Scenario: Program reports its status
- **WHEN** a pane's program reports `working` through OSC 7501
- **THEN** `ship pane get` shows the record under `program` with the key `""`
