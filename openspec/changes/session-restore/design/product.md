# Session restore

Status: Locked on 2026-10-15, approved by Cyan after Plannotator review ("LGTM"). Written on 2026-10-14 from a design conversation with Cyan and revised on 2026-10-15 with Cyan's answers to the three open unknowns. The approval doesn't authorize implementation; Cyan requests that separately.

Amended during slice 3 on 2026-10-16 after Cyan accepted the slice 2 timing clarification: keep the planned five-second trigger, rather than shortening it to pass an exact-five-second SIGKILL test. FR-005 and SC-002 distinguish the timer from completion of the disk write. No implementation timing changed.

## Goal

When Ship's server stops, whether through `ship server stop`, a crash or a reboot, every tab, split and directory is lost, and the user rebuilds the workspace by hand. This change saves the workspace's structure as it changes and rebuilds it when the server starts again, so that after a reboot bare `ship` opens the same tabs and splits, with a shell in each pane's last directory. It covers the save-and-restore half of milestone 2 in the product brief. Agent conversation resume, the other half, is deferred (see Non-goals).

Restore never reruns what a pane was running. Every restored pane is a fresh shell. Cyan found Zellij's held commands annoying: when one finished, the pane closed instead of dropping back to a shell.

Restore problems, such as a pane whose directory is gone, are reported only in the server log. The diagnostics change gives that log a stable path, `~/.local/state/ship/logs/<port>/current.log`, so this change depends on it. Cyan: "trying to send it to the client opens a lot of questions. plus, i'm sure the audience is technical enough to know how to find the logs."

"Workspace" stays the product word in this paper's Goal and user stories. The requirements name what is written to disk **saved tabs**, defined under "What is saved", because the glossary rules out a workspace entity.

## User stories

### P1: Get the workspace back after a restart

A user with several tabs and splits restarts the machine, runs `ship`, and finds the same workspace.

- **Given** two top-level tabs, one with a child tab, and a tab split three ways with custom proportions and one pane zoomed, **when** the server stops and starts again, **then** the tab tree, tab and pane names, split proportions and zoom are as they were.
- **Given** a pane whose shell has moved to `~/projects/ship`, **when** the server restarts, **then** that pane runs a new shell in `~/projects/ship`.
- **Given** a pane that was running `nvim foo.rs`, **when** the server restarts, **then** that pane runs a shell in the same directory, and `nvim` is not started.
- **Given** a client left open while the server is stopped and started again, **when** it reconnects, **then** it keeps the tab or pane it had selected, because restored tabs and panes keep their IDs.
- **Given** the default server is not running after a reboot, **when** the user runs bare `ship`, **then** the background server it launches restores the workspace without adding a starter tab, and the client opens on the first top-level tab's first pane.

Tested independently by building a workspace, running `ship server stop`, starting the server again, and comparing `ship tab list` and `ship pane get` output from before and after.

### P2: Changes are saved without asking

The user never runs a save command. What they do is on disk shortly after they do it.

- **Given** a running server, **when** the user splits a pane, renames a tab or turns on `right_click_passthrough`, and the server is killed with SIGKILL 10 seconds later, **then** the next start restores that change.
- **Given** a pane's shell changes directory, **when** the server is killed 10 seconds later, **then** the restored pane starts in the new directory.
- **Given** the server is killed in the middle of a save, **when** it starts again, **then** it restores either the state before that save or the state after it, never a damaged one.

Tested independently with SIGKILL at chosen delays, and by killing the server during a save in a loop.

### P3: Shutdown doesn't erase the workspace

During a reboot or logout, the operating system can kill the programs in panes before the server itself is told to stop. On macOS, Herdr measured this gap at 15 to 20 seconds. Those deaths must not reach the saved tabs.

- **Given** a workspace with running shells, **when** every pane's program is killed and the server receives SIGTERM 20 seconds later, **then** the next start restores every pane.
- **Given** a user ends a pane's program and leaves the server running, **when** 60 seconds pass, **then** the saved tabs reflect that change.
- **Given** a user ends a pane's program and the machine reboots within 60 seconds, **then** the workspace comes back as it was before the program ended. This is accepted.

Tested independently by killing pane programs with signals before stopping the server. Today an ended program only changes the pane's status, which isn't saved, so these stories become observable once a change like #16 (close a pane when its program exits) makes an exit change saved state.

### P4: A directory that no longer exists

- **Given** a pane last in `~/scratch/tmp-build`, **when** that directory is deleted and the server restarts, **then** the pane starts a shell in the user's home directory, and the server log names the pane and the directory it couldn't use.

Tested independently by deleting a pane's directory between stop and start, then reading the server log.

### P5: Starting fresh, and getting the old workspace back

- **Given** saved tabs, **when** the user runs `ship server --no-restore`, **then** the server starts with no restored tabs, and the saved tabs are replaced on the first save.
- **Given** a server's first save is about to replace saved tabs, **then** the old file is kept beside it as a single `.prev` copy, so a user who started fresh by mistake, or whose restore went wrong, can stop the server, move the copy back and start again.
- **Given** a saved file that the server can't read, **when** the server starts, **then** it starts with no restored tabs, the server log names the file and the path of the kept copy, and the unreadable file is not lost.

Tested independently with `--no-restore`, with a hand-corrupted file, and by restoring from the `.prev` copy by hand.

## Non-goals

- Agent conversation resume. It depends on hooks for each agent, the signals are unreliable, and `claude -c` in a restored pane covers most of the value. The brief's "supported agent conversation resume after restart" moves out of this change.
- Rerunning a pane's saved program, holding it behind a "press Enter" prompt, or any respawn action.
- Restoring screen contents or scrollback.
- Detecting that the operating system is shutting down. Herdr does this with platform code on all three systems (about 800 lines, and its macOS path is untested on a real reboot); Ship relies on P3's delay instead.
- Telling clients about restore problems, through the status line or anything else, and listing them in `ship server status`. A restore problem is a one-time startup event, and `ship server status` reports what is true now.
- An explicit save or restore command, or saving to a named file.
- More than one kept copy, or a history of past saved tabs.
- Restoring automatically from the `.prev` copy. Recovery is by hand.
- A version field in the saved file. One gets added only if a saved field ever changes meaning.
- Reading a newer Ship's saved file without loss. An older Ship drops fields it doesn't know.
- Named servers. The saved tabs belong to the server's port.
- Restoring a client's own state: folds, modes, sidebar width or visibility.

## Functional requirements

### What is saved

**Saved tabs** are the server's tab tree as last written to its port's saved file: every top-level tab and everything under it, without anything live.

- **FR-001** The saved tabs MUST hold the shared tab and pane structure: the tab tree with its order and nesting, tab and pane IDs, tab and pane names, each tab's split layout and proportions, each tab's zoomed pane, each pane's `right_click_passthrough` setting, and each pane's current directory, else its starting directory.
- **FR-002** The saved tabs MUST NOT hold a pane's program or arguments, its title, its exit status, or its screen and scrollback. Any other shared tab or pane attribute added later MUST be saved unless that later change says otherwise.
- **FR-003** The saved file MUST be readable and writable only by the user who runs the server.
- **FR-004** Each server port MUST have its own saved tabs. A server MUST NOT read or write another port's.

### When it is saved

- **FR-005** A change made by a user or a script (any tab or pane command, key, mouse action or menu item) or reported by a shell (a directory change) MUST schedule a save 5 seconds after the first unsaved change while the server keeps running. Later changes MUST NOT postpone the deadline. Serialization and disk I/O take additional time; this is a save trigger, not a completion guarantee.
- **FR-006** A change caused only by a program ending MUST NOT be saved until the server has kept running for 60 seconds after it, unless a save under FR-005 happens first. If the server stops before then, the change MUST NOT be saved.
- **FR-007** When the server stops through `ship server stop`, SIGTERM or SIGINT, it MUST save any FR-005 change not yet saved, and MUST NOT save a change still waiting under FR-006. With nothing waiting under FR-005, it MUST NOT write the file.
- **FR-008** A save interrupted at any point, including by SIGKILL or a crash, MUST leave the file holding either the previous saved tabs or the new ones.
- **FR-009** Before a server's first save replaces an existing saved file, the server MUST keep that file's contents in a `.prev` copy beside it, replacing any older copy. Later saves in the same server run MUST NOT touch the copy.

### Restoring

- **FR-010** On startup, before it accepts any connection, the server MUST rebuild the saved tabs with the same IDs, names, structure, proportions, zoom and passthrough settings.
- **FR-011** Each restored pane MUST run the configured shell, else the login shell, as configured at restore time, starting in the pane's saved directory. That directory MUST be its starting directory for `ship pane get`.
- **FR-012** If a pane's saved directory doesn't exist or isn't a directory, its shell MUST start in the home directory, and the server MUST log a warning naming the pane and its saved directory.
- **FR-013** If the saved file can't be read or parsed, the server MUST start with no restored tabs and log a warning naming the file and the `.prev` copy that FR-009 will keep. It MUST NOT delete the file.
- **FR-014** `ship server --no-restore` MUST start without restoring. Its help text MUST say that the saved tabs are replaced on the first save and kept as the `.prev` copy. Bare `ship`, and any other command that launches the background server, MUST always restore.
- **FR-015** A restored pane whose shell fails to start MUST be left out of the restored tabs without stopping the server, and the server MUST log a warning naming the pane and its saved directory. The `.prev` copy keeps the full saved tabs.

### Changes to earlier promises

- **FR-016** `right_click_passthrough` MUST survive a server restart. This replaces session-structure's "The setting SHALL NOT survive a server restart" and the mouse change's matching non-goal.
- **FR-017** The product brief's milestone 2 MUST be amended: agent conversation resume moves to a later change, and "distinguishes resumed work from unavailable work" becomes "logs panes whose directory was unavailable".
- **FR-018** The glossary MUST gain **Saved tabs**, defined as above, with Workspace, Session, Snapshot and Saved state listed to avoid.

### The starter tab and the first selection

- **FR-019** `ship server --starter` MUST create its shell tab only when the server has no tabs after restoring. That includes a first run with no saved file, saved tabs that held no tabs, `--no-restore`, and an unreadable saved file. Its help text MUST say so. Without `--starter`, a server with nothing to restore MUST start with no tabs, as today.
- **FR-020** When bare `ship` launches the background server, the client it opens MUST select the first top-level tab's first pane, whether that tab was restored or created by `--starter`. A client attaching to a server that was already running MUST start with nothing selected, as today.

## Success criteria

- **SC-001** After `ship server stop` and a fresh start, `ship tab list` and each `ship pane get` match their earlier output except for program, title, status and starting directory, on Linux and macOS.
- **SC-002** A change is restored after SIGKILL following its completed scheduled save, in 20 out of 20 tries.
- **SC-003** Killing the server during saves in a loop of at least 200 iterations never leaves a file the next start can't read.
- **SC-004** With every pane's program killed and SIGTERM sent 20 seconds later, the next start restores every pane.
- **SC-005** Restoring 10 tabs and 30 panes makes bare `ship` no slower to show a usable screen than starting with one starter tab, judged by feel.
- **SC-006** No saved file contains a pane's program arguments.
- **SC-007** Ten restarts of the default server through bare `ship` leave the same number of top-level tabs as before the first.

## Open unknowns

None. U-1 to U-3 were answered on 2026-10-15:

- **U-1: Where the user learns about restore problems.** The server log only (FR-012, FR-013, FR-015), at the path the diagnostics change sets. No client notice and no `ship server status` field.
- **U-2: `--starter` with restored tabs.** `--starter` opens a tab only when there are no tabs after restoring (FR-019).
- **U-3: What a new client selects after restore.** Unchanged. The client that launched the server already asks for the first tab's pane, whether that tab was restored or a starter (FR-020).
