# session-restore Specification

## Purpose

Bring a server's tabs, splits and directories back after it stops, whether through `ship server stop`, a crash or a reboot. **Saved tabs** are the server's tab tree as last written to its port's saved file: every top-level tab and everything under it, without anything live. Every restored pane is a fresh shell; restore never reruns what a pane was running.

## Requirements

### Requirement: Saved tabs contents
The saved tabs SHALL hold the tab tree with its order and nesting, tab and pane IDs and names, each tab's split layout, proportions and zoomed pane, and each pane's `right_click_passthrough` setting and current directory, else its starting directory. A shared tab or pane attribute added later SHALL be saved unless its change says otherwise.

#### Scenario: Current directory is saved
- **WHEN** a pane's shell started in `~` changes directory to `~/projects/ship` and the saved tabs are written
- **THEN** that pane's saved directory is `~/projects/ship`

### Requirement: Nothing live is saved
The saved tabs SHALL NOT hold a pane's program or arguments, its title, its exit status, its status records, its geometry, or its screen and scrollback.

#### Scenario: Program arguments stay off disk
- **WHEN** a pane runs `sleep 1234` and the saved tabs are written
- **THEN** the saved file does not contain `1234`

#### Scenario: Status records stay off disk
- **WHEN** a pane's program has reported `working` and the saved tabs are written
- **THEN** the saved file has no `program` field, and after a restart the restored pane has no records

### Requirement: Saved file location
Each server port SHALL have its own saved file at `$XDG_STATE_HOME/ship/tabs/<port>.json`, defaulting to `~/.local/state/ship/tabs/<port>.json` on Linux and macOS; other platforms' location is best effort. A server SHALL NOT read or write another port's file, and a second server on the same port SHALL fail to bind before it reads the file.

#### Scenario: Two ports
- **WHEN** servers run on ports 43179 and 43180 with different tabs and both restart
- **THEN** each restores its own saved tabs from its own file

### Requirement: Saved file permissions
The saved file SHALL be readable and writable only by the server's user, mode `0600` on Unix, in a `tabs` directory of mode `0700` on Unix.

#### Scenario: Private file
- **WHEN** a server has written its saved tabs
- **THEN** the file is mode `0600` and its `tabs` directory mode `0700`

### Requirement: Saved file format
The saved file SHALL hold pretty-printed JSON in the shape `ship tab list` prints, with each pane's `cwd` set to its saved directory, `command` empty, `status` running, and `dir`, `title` and tab `geometry` absent. Loading SHALL ignore `command`, `status`, `dir`, `title` and `geometry`, so `ship tab list` output is a valid saved file.

#### Scenario: Hand-written file
- **WHEN** a user saves `ship tab list` output as the port's saved file and starts the server
- **THEN** the server restores those tabs, each pane running a shell in that pane's `cwd`

### Requirement: Saving changes
A change made by a user or a script, through any tab or pane command, key, mouse action or menu item, or reported by a shell as a directory change, SHALL schedule a save for 5 seconds after the first unsaved change while the server keeps running. Later changes SHALL NOT postpone that deadline. The save SHALL write the latest projection at that deadline; serialization and disk I/O take additional time. A change that leaves the saved tabs as last written SHALL NOT cause a write.

#### Scenario: Killed after an edit
- **WHEN** a user splits a pane, its scheduled save completes, and the server is killed with SIGKILL
- **THEN** the next start restores the split

#### Scenario: Steady directory changes
- **WHEN** a shell changes directory every second for 20 seconds
- **THEN** each first unsaved change schedules a save 5 seconds later, without later changes postponing it

### Requirement: Program exits wait
A change caused only by a program ending SHALL NOT be saved until the server has kept running for 60 seconds after it, unless a save of another change happens first.

#### Scenario: Programs die before the server stops
- **WHEN** every pane's program is killed and the server receives SIGTERM 20 seconds later
- **THEN** the next start restores every pane as it was before the programs were killed

#### Scenario: Program ends while the server keeps running
- **WHEN** a change caused by a program ending is followed by 60 seconds in which the server keeps running
- **THEN** the saved tabs reflect that change

### Requirement: Atomic saves
A save interrupted at any point, including by SIGKILL or a crash, SHALL leave the file holding either the previous saved tabs or the new ones.

#### Scenario: Killed during a save
- **WHEN** the server is killed with SIGKILL while it writes a save
- **THEN** the next start restores either the saved tabs from before that save or the ones after it

### Requirement: Failed saves
A save that fails SHALL be logged as an error and retried 5 seconds later, and the server SHALL keep serving.

#### Scenario: Unwritable directory
- **WHEN** the `tabs` directory is read-only and a user renames a tab
- **THEN** the server logs an error every 5 seconds and keeps serving commands

### Requirement: Saving on stop
When the server stops through `ship server stop`, SIGTERM or SIGINT, it SHALL save any change from a user, script or shell that is not yet saved. It SHALL NOT save a change still waiting for its 60 seconds after a program ended. With no such unsaved change, it SHALL NOT write the file.

#### Scenario: Stop right after an edit
- **WHEN** a user renames a tab and runs `ship server stop` one second later
- **THEN** the next start restores the new name

#### Scenario: Stop without changes
- **WHEN** a server restores its saved tabs and is stopped without any change
- **THEN** the saved file is not written

### Requirement: Kept previous copy
Before a server's first save replaces an existing saved file, the server SHALL copy that file's contents, readable or not, to `<port>.json.prev` beside it, replacing any older copy. Later saves in the same server run SHALL NOT change the copy. The server SHALL NOT restore from the copy on its own.

#### Scenario: Recover after starting fresh
- **WHEN** a user runs `ship server --no-restore`, makes an edit, stops the server, moves `<port>.json.prev` over `<port>.json` and starts the server
- **THEN** the server restores the tabs from before `--no-restore`

#### Scenario: Later saves leave the copy alone
- **WHEN** a server saves three times in one run
- **THEN** `<port>.json.prev` holds the file as it was before the first of those saves

### Requirement: Restore on startup
On startup, before it accepts any connection, the server SHALL rebuild the saved tabs with the same IDs, names, structure, proportions, zoom and each pane's `right_click_passthrough`, `exit_behavior` and `scrollback_bytes`. Each restored pane SHALL run `[server.defaults.pane] shell`, else the login shell, as configured at restore time, in its saved directory, which SHALL be its starting directory for `ship pane get`. It SHALL NOT run the pane's earlier program. A missing saved file SHALL start with no restored tabs.

#### Scenario: Structure comes back
- **WHEN** a server with nested tabs, named tabs and panes, custom proportions and a zoomed pane restarts
- **THEN** `ship tab list` shows the same IDs, names, nesting, proportions and zoom

#### Scenario: Pane settings come back
- **WHEN** a pane with `exitBehavior: keep` and `scrollbackBytes: 50000000` is saved and the server restarts
- **THEN** the restored pane, a fresh shell, shows the same two values

#### Scenario: Saved before pane settings existed
- **WHEN** a saved file from before exit behavior and per-pane history size is restored
- **THEN** its panes get `exitBehavior: close` and `scrollbackBytes: 10000000`, the built-in defaults

#### Scenario: Program not rerun
- **WHEN** a pane running `nvim foo.rs` in `~/projects/ship` is restored
- **THEN** the pane runs the configured shell in `~/projects/ship` and `nvim` is not started

#### Scenario: Reconnecting client
- **WHEN** a client stays open while the server stops and starts again
- **THEN** after reconnecting it keeps the tab or pane it had selected

### Requirement: Starting without restoring
`ship server --no-restore` SHALL start without reading the saved file. Its help SHALL say that the saved tabs are replaced on the first save and kept as `<port>.json.prev`. Every server launched automatically SHALL restore.

#### Scenario: Starting fresh
- **WHEN** the port has saved tabs and a user runs `ship server --no-restore`
- **THEN** `ship tab list` prints `{}` and the saved file is unchanged until the first save

### Requirement: Missing pane directory
If a pane's saved directory doesn't exist or isn't a directory, its shell SHALL start in the home directory, and the server SHALL log a warning naming the pane and its saved directory.

#### Scenario: Missing directory
- **WHEN** a pane's saved directory is deleted before the server restarts
- **THEN** the pane's shell starts in the home directory and the server log names the pane and its old directory

### Requirement: Pane that can't start
A restored pane whose shell fails to start SHALL be left out of the restored tabs without stopping the server, and the server SHALL log a warning naming the pane and its saved directory. Its tab's zoom SHALL remain when the zoomed pane was restored.

#### Scenario: Shell can't start
- **WHEN** the configured shell doesn't exist at restore time
- **THEN** the server starts without the restored panes and logs a warning for each

### Requirement: Unusable saved file
If the saved file can't be read or parsed, the server SHALL start with no restored tabs and log a warning naming the file and the `.prev` path. It SHALL NOT delete or change the file before its first save. A file that parses SHALL be trusted without structural validation, including duplicated IDs, mismatched tab keys or a zoom on a foreign pane. When one JSON object repeats a tab key, the last entry SHALL be restored.

#### Scenario: Unreadable file
- **WHEN** the saved file holds invalid JSON at startup
- **THEN** the server starts with no restored tabs, the server log names the file and the `.prev` path, and that copy holds the invalid contents after the first save

#### Scenario: Duplicate pane ID
- **WHEN** a hand-edited saved file uses one pane ID in two tabs
- **THEN** the loader accepts the parsed file without rejecting its duplicated IDs; behavior of the hand-edited structure is not guaranteed

### Requirement: Where restore problems go
Restore problems SHALL be reported only in the server log, not to clients or in `ship server status`.

#### Scenario: Status after a restore problem
- **WHEN** a pane's saved directory was missing at startup and a user runs `ship server status`
- **THEN** the output has no restore field, and the warning is in the server log
