# Spec Delta

## ADDED Requirements

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

## MODIFIED Requirements

### Requirement: Every pane runs a program
Every pane SHALL run exactly one program in its own terminal, started when the pane is created. A pane SHALL NOT exist without having started its program. If the program cannot be started, pane creation SHALL fail with an understandable error and change nothing.

#### Scenario: Create a pane
- **WHEN** a user runs `ship pane create <tab>`
- **THEN** the pane is printed as JSON and the server has a running child process for it

#### Scenario: Program cannot start
- **WHEN** a user runs `ship pane create <tab> -- definitely-not-a-command`
- **THEN** the command fails with a message saying why, and `ship tab list` shows the same output as before the attempt

## REMOVED Requirements

### Requirement: Ending programs
**Reason**: Replaced by a version without sessions, whose session-specific scenarios no longer apply.
**Migration**: See "Ending pane programs"; removing a tab ends its programs.
