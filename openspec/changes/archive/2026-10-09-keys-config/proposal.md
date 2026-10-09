# Proposal

## Why

Ship's keys are a fixed `C-b` placeholder that nobody chose, and nothing about Ship can be configured. Cyan drives Herdr with plain Alt chords and wants the same in Ship, editable in a file that applies on save. The CLI and the keys also have no shared vocabulary today, so every new action would be built twice. This change lands before layout and sidebar, which name their actions in the vocabulary it defines.

## What Changes

- A config file at the platform config directory's `ship/config.toml`, chosen instead by `--config <file>` or `SHIP_CONFIG`. `[server]` holds `shell`, the program new panes run by default. `[client]` holds the key modes. Each process reads only its own part. Unknown fields and actions are errors.
- **BREAKING:** The `C-b` prefix keys are removed. The defaults are Alt chords from Cyan's Herdr config: for example, `alt-n` makes a tab with a shell, `alt-|` makes a pane, `alt-left`/`alt-right` move between tabs, `alt-tab` moves between panes, and `alt-q` detaches. Unbound keys in normal mode reach the program, `ctrl-b` included.
- Users define their own modes, each sticky or one-shot, under `[client.modes.<name>]`. `normal` is the start mode. The defaults ship `tabs`, `resize` and an empty one-shot `prefix`. A chord in the file replaces the default on that chord, `"none"` unbinds it, and `clear_defaults` empties a mode.
- Every binding names a `server.` action, which changes shared state and is also a `ship` command at the same path, or a `client.` action, which changes only that client's view. A binding can also be a list of actions that runs in order and stops at the first failure. Actions whose feature isn't built yet (splits, resize, sidebar, focus by direction) are accepted and report "not available yet". Until layout lands, `server.pane.create` adds a pane to the selected tab.
- The client reloads its keys when the file is saved, including rename-saves and symlinked files. A bad save keeps the running keys and shows the error. The server reads `shell` each time it starts a shell.
- `ship config check [FILE]` reports every error with its location and exits non-zero. There are no warnings: it reports exactly what loading rejects.
- **BREAKING:** `ship tab rm` and `ship pane rm` become `ship tab close` and `ship pane close`. Commands that act on an existing tab or pane take `--tab ID` or `--pane ID` instead of a positional ID; commands that act on a tab also take `--pane ID` for the tab holding it. Run inside a pane, they default to that pane or its tab, and outside a pane they fail and say so. `ship pane create` takes its tab as `--tab`.
- `ship tab create` gains `--starter`, which opens the tab with a pane running the shell, and `--before`/`--after`, which place it next to a sibling. `ship pane resize` exists and reports "not available yet".
- **BREAKING:** The tab create body takes `at`, the same destination move takes, instead of `parent`, plus an optional `starter`. The health `protocolVersion` becomes 6.

## Capabilities

### New Capabilities

- `config-file`: where the config file lives, how each process reads its part, errors and checking, reloading, and the server's `shell` setting.
- `keymap`: modes, chords, bindings, the default keys, and how key actions act on the client's selection.

### Modified Capabilities

- `session-structure`: `close` replaces `rm`, existing tabs and panes are targeted by `--tab`/`--pane` with in-pane defaults, and tab creation gains `--starter`, `--before` and `--after`.
- `terminal-client`: the fixed `C-b` pane keys are removed in favor of `keymap`. The empty-state hint names the default Alt key, and the status line shows the active mode and key-action messages.
- `session-observation`: top-level tab cycling moves from `C-b )`/`C-b (` to the `client.tab.next`/`prev` actions, by default `alt-right`/`alt-left`.
- `pane-programs`: a pane without a command runs the configured shell, else the login shell, and `ship pane create` takes `--tab`.
- `health-exchange`: protocol 6. The starter's shell follows the configured shell.
- `local-foundation`: help describes the Alt keys, `--config` and `ship config check`, and the README documents the config file and keys.

## Impact

All four crates change. New dependencies: figment and crokey (client and server config), notify-debouncer-mini (client watching), etcetera (config location).

- `ship-core`: a command tree with one named struct per `server.` action, with clap behind a new `clap` feature. `CreateTab` takes a destination and a starter. Protocol 6.
- `ship-server`: tab creation places tabs and starts the starter shell; the startup-only starter message goes away; pane spawn reads `[server] shell`.
- `ship-client`: one `execute` path shared by the CLI and keys; the keymap, mode machine, command queue and file watcher; the status line shows the mode and messages.
- `ship`: argument structs replaced by the core command tree, `--config`, `ship config check`, `--config` forwarded to the background server.

The three locked papers in [design/](design/), indexed by [design.md](design.md), are the authoritative design. Implementation follows the program paper's five slices, one at a time, and starts only when Cyan requests it. Linux and macOS are verified deliberately; Windows stays best effort and unverified. Floating panes, keyboard enhancement flags, the CLI driving a client, command bindings and permanent authored tests are not included.
