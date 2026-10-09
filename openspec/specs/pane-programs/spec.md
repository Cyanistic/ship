# pane-programs Specification

## Purpose

Give every pane exactly one server-owned program running in its own terminal, report that program's state, and end it with every process it started when the pane goes away.

## Requirements

### Requirement: Every pane runs a program
Every pane SHALL run exactly one program in its own terminal, started when the pane is created. A pane SHALL NOT exist without having started its program. If the program cannot be started, pane creation SHALL fail with an understandable error and change nothing.

#### Scenario: Create a pane
- **WHEN** a user runs `ship pane create <tab>`
- **THEN** the pane is printed as JSON and the server has a running child process for it

#### Scenario: Program cannot start
- **WHEN** a user runs `ship pane create <tab> -- definitely-not-a-command`
- **THEN** the command fails with a message saying why, and `ship tab list` shows the same output as before the attempt

### Requirement: Program and starting directory
`ship pane create <tab>` SHALL start the user's login shell by default, in the CLI's current directory. `--cwd DIR` SHALL set the starting directory, and SHALL fail creation without changing anything when `DIR` is not an existing directory. `-- COMMAND ARGS...` SHALL run that command instead of the shell. `--name NAME` SHALL be optional.

#### Scenario: Default shell in the current directory
- **WHEN** a user runs `ship pane create <tab>` from `/tmp`
- **THEN** the pane runs the user's shell, and `ship pane get` reports `/tmp` as its starting directory

#### Scenario: Explicit command and directory
- **WHEN** a user runs `ship pane create <tab> --cwd /var -- sh -c 'sleep 100'`
- **THEN** the pane runs `sh -c 'sleep 100'` starting in `/var`

#### Scenario: Missing directory
- **WHEN** a user passes `--cwd` naming a directory that does not exist
- **THEN** pane creation fails and nothing changes

### Requirement: Color terminal
Programs in panes SHALL see a terminal that advertises 256 colors and 24-bit color.

#### Scenario: Colored output
- **WHEN** a program in a pane checks the terminal's advertised color support and prints 24-bit colored text
- **THEN** it finds 256 and 24-bit color support, and an attached client shows the exact colors

### Requirement: Pane inspection
`ship pane get` SHALL report the pane's command, starting directory, current title and status, which is running or exited with its exit code. The title SHALL follow the title the program sets, and SHALL be absent when the program has set none or has cleared it.

#### Scenario: Program sets a title
- **WHEN** a pane runs `sh -c 'printf "\033]0;hello\007"; sleep 100'`
- **THEN** `ship pane get` reports the title `hello` and the status running

### Requirement: Exited programs
When a pane's program exits, the pane SHALL remain with its last screen and exit status until it is removed, and SHALL ignore input.

#### Scenario: Program exits with a code
- **WHEN** a pane's program runs `exit 3`
- **THEN** the pane remains, `ship pane get` reports it exited with code 3, and attached clients still show its last screen

### Requirement: Programs outlive clients
Detaching, closing a client's terminal window or losing a client's connection SHALL NOT affect running programs.

#### Scenario: Detach and reattach
- **WHEN** a pane runs a counting loop, its only client detaches, and a client attaches again later
- **THEN** the loop kept running and the client shows its current output

### Requirement: Ending pane programs
Removing a pane, removing a tab containing it, or stopping the server SHALL end the pane's program and every process it started in its terminal. They SHALL get about two seconds to exit after hangup before being forced. On Linux and macOS no such process SHALL remain running or unreaped, except two gaps the architecture accepts: a process that ignores hangup in its own process group, and a pane removed under two seconds before server shutdown.

#### Scenario: Remove a pane with a background job
- **WHEN** a pane's shell has started `sleep 1000 &` and the user runs `ship pane rm` on the pane
- **THEN** within three seconds `ps` shows neither the shell nor `sleep`

#### Scenario: Program ignores hangup
- **WHEN** a pane runs `sh -c 'trap "" HUP; sleep 1000'` and the pane is removed
- **THEN** the program is killed after the grace period and no process remains

#### Scenario: Remove a tab
- **WHEN** a user removes a top-level or nested tab containing running panes, directly or in its descendants
- **THEN** every program in those panes ends the same way

#### Scenario: Stop the server
- **WHEN** the server receives SIGTERM with three running panes, one of them running a program that ignores hangup
- **THEN** the server exits within five seconds and no pane process remains
