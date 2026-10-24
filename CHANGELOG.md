# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0](https://github.com/shipmux/ship/releases/tag/v0.1.0) - 2026-10-24

### Added

- [**breaking**] gather pane settings under [server.defaults.pane]
- *(website)* a large app mark with the crew on deck, used in the README
- print shell completion scripts with ship completions
- print the built-in defaults with ship config defaults
- print CLI failures as JSON
- let pane wait hold for a first report and document pane automation
- log lifecycle events and request errors
- play sounds when agents finish or need attention
- restore saved tabs at startup
- record opt-in client and server timelines
- change server log filters without restarting
- save per-run logs and configure startup levels
- add right-click passthrough to mouse-aware panes
- add configurable context menus
- add sidebar tab drag-and-drop
- add border drags and client-local sidebar resizing
- add client view selection commands
- use single-action key bindings with optional hints
- add ship pane read
- start key splits in the pane's directory and add child tabs
- add pane zoom, swap and edge resize
- add the tab sidebar and status-line overlay
- draw split panes together with directional focus
- implement server-owned pane splits
- keymap and modes, with Alt chords to actions and C-b gone
- config path and the server's configured shell
- one command tree for the CLI's tab and pane commands
- start the default server with a starter tab
- [**breaking**] replace sessions with top-level tabs
- the full-screen client
- input, view and size for attached panes
- pane screens on the attach stream
- every pane runs a program
- optional tab and pane names, vendor ratatui-ghostty
- compress attach streams and shut down cleanly
- say why an attach stream ends
- reconnect observers after a dropped connection
- add per-observer selection and session switching
- add live session observation with ship attach
- add logical panes through the CLI and /api/v0
- add recursive tabs and tab moves through the CLI and /api/v0
- add session CRUD through the CLI and /api/v0
- complete schema inspection and client-update verification
- add the verified health exchange and local server lifecycle
- establish the verified Ship foundation

### Fixed

- run a key's list in order, and size panes to the client's area

### Other

- explain pane defaults and exit behavior
- give each page a scope and move the docs rules to docs/agents/docs.md
- trim the README to a pitch and links, move keys and the CLI demo into the guide
- rewrite the README and guide intro in plainer words, with real examples
- split the guide into docs/site pages
- name the health service in one shared constant
- Merge pull request #54 from shipmux/shipmux-urls
- name the completions arguments and take the binary name from Cargo
- point links at the shipmux organization
- license Ship under GPL-3.0-or-later
- rewrite the guide around tasks and refresh the README
- document agent status and record program status slice 5
- document session restore and sync specs
- complete server-selection documentation and verification
- rewrite README for new users and move detail to a guide
- finish scrollback docs and preserve deferred design notes
- read the config with toml and bind keys to lists
- clear workspace Clippy warnings
- keys and configuration in the README; close 3.5
- describe running panes in the README and drop sessions from spec purposes
- describe top-level tabs instead of sessions
- rewrite README for humans and add the ship logo
