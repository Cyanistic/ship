# Keys and config

Status: Locked again on 2026-10-09 with amendment A-5: an action in a list starts only once the client shows what the action before it did (FR-030). Verifying slice 5 showed `alt-m`, a starter tab then a pane, failing 4 times in 10 with "no tab selected": the pane create ran before the client had seen the new tab. Cyan chose a revision number on every server response, which the client waits for, over server-side batches and over moving keys to the server, because each client owns its own keys ("it feels to me like the smart thing is probably to have each client own it"), and asked for it to be tried first ("let's go for it and see if that fixes things!"). The same amendment moves the status line's row out of the server's sizing: clients report the area they draw a tab in, which leaves this paper's behavior unchanged ("let's just add it to this branch since it's small"). Cyan approved writing it into the papers in chat ("go for it!"). Before that, amendment A-4: a binding can be a list of actions, run in order and stopped at the first failure (Summary, Interface, FR-030). Implementing slice 3 showed that figment's merge blends a file binding into the default it replaces, and a list is replaced whole instead. Cyan decided it in chat because action lists are established practice (Zellij binds a list of actions, tmux chains commands with `\;`) and mean something on their own ("if sequences are actually an established pattern, and have real meaning, i don't see why not? kill two birds with one stone?"), then asked for the papers to follow once the code worked. Before that, amendment A-3: commands that take a tab also accept `--pane ID`, meaning the tab holding that pane, and `--tab` wins when both are given (Command line, FR-026, FR-027). Cyan proposed it in chat after slice 1, for consistency, and approved the edits without another review ("sounds good! go for it! i don't need to review your edits"). A probe showed clap counts an env-filled `--pane` as given, so the two flags aren't a strict either-or: `--tab` takes precedence instead. Before that, amendment A-2: config warnings are dropped, so FR-016 keeps only its error and `ship config check` reports exactly what loading rejects. Cyan decided it in chat during the program review ("yeah go for it.... at least for now"), noting that any later warnings would need a much simpler, declarative design. Before that, amendment A-1: modes move under `[client]` (`[client.modes.<name>]`), so the file's top level holds only `[server]` and `[client]` (Interface, FR-003, FR-014). Cyan decided it in chat during the architecture draft and asked for a direct edit without another Plannotator pass ("just edit it directly"). First locked on 2026-10-09: Cyan approved the draft in Plannotator with "LGTM" and, in conversation, decided three changes made alongside that review: chords stay case-insensitive instead of rejecting uppercase letters (FR-011), new tabs from keys start with a shell (U-1, now FR-029), and floating panes are deferred to their own change (U-2, now a non-goal). It supersedes [notes.md](../notes.md) wherever the two differ. The locked [pane terminals](../../archive/2026-10-07-pane-terminals/design/product.md) and [drop sessions](../../archive/2026-10-08-drop-sessions/design/product.md) papers stay in force except where this paper amends them.

## Summary

Ship gets a config file and configurable keys. The `C-b` placeholder keys go away. The defaults become plain Alt chords taken from Cyan's Herdr config. The file has two parts:

- **Modes:** each mode is a named table of key bindings. Users can define their own modes, each one either sticky or one-shot.
- **Server settings:** one setting today, the shell new panes run.

Every binding names an action under `server.` or `client.`, or a list of them that runs in order:

- **`server.` actions** change shared state that every attached client sees. Each one is also a `ship` CLI command with the same path.
- **`client.` actions** change only your own view, so only keys and the mouse can trigger them.

The CLI changes to match: `rm` becomes `close`, and commands find their tab or pane through `--tab` and `--pane`. Inside a pane, those flags default to the pane the command runs in.

Needs Cyan's attention:

- **Pane creation before layout:** until the layout change lands, creating a pane from a key ignores the direction and adds the pane to the selected tab (FR-019). Without that, no default key would make a pane until layout lands.
- **New tabs from keys start with a shell:** `server.tab.create` takes an optional starter (FR-029). The default `alt-n` sets it, and `ship tab create` stays empty unless given `--starter`.

## Goal

Let Cyan drive Ship from the keyboard the way he drives Herdr, with chords he already knows, and change them in a file that reloads on save. Keys, the mouse and the CLI share one set of actions, so they can't drift apart, and the CLI's names and the config's names teach each other.

## Interface

### Config file

The file is `~/.config/ship/config.toml`. `--config <file>` overrides the location, then `SHIP_CONFIG`, then the default. Client and server read the same file, and each reads only its own part. The server reads `[server]`, and the client reads `[client]`, which holds the modes under `[client.modes.*]`. Anything else at the top level is an error. On another machine, that machine's server reads its own copy.

The default config:

```toml
[server]
shell = "/bin/zsh"                 # default: the login shell, as today

[client.modes.normal.keys]
"alt-n"       = { server.tab.create.starter = "shell" }
"alt-x"       = { server.tab.close = {} }
"alt--"       = { server.pane.create.direction = "down" }
"alt-|"       = { server.pane.create.direction = "right" }
"alt-shift-x" = { server.pane.close = {} }
"alt-left"    = { client.tab.prev = {} }
"alt-right"   = { client.tab.next = {} }
"alt-1"       = { client.tab.select.row = 1 }    # through alt-9
"alt-h"       = { client.pane.focus.direction = "left" }
"alt-j"       = { client.pane.focus.direction = "down" }
"alt-k"       = { client.pane.focus.direction = "up" }
"alt-l"       = { client.pane.focus.direction = "right" }
"alt-tab"     = { client.pane.next = {} }
"alt-b"       = { client.sidebar.toggle = {} }
"alt-g"       = { client.mode = "tabs" }
"alt-r"       = { client.mode = "resize" }
"alt-q"       = { client.detach = {} }

[client.modes.tabs]
kind = "sticky"

[client.modes.tabs.keys]
"j"   = { client.tab.next = {} }
"k"   = { client.tab.prev = {} }
"h"   = { client.tab.collapse = {} }
"l"   = { client.tab.expand = {} }
"esc" = { client.mode = "normal" }

[client.modes.resize]
kind = "sticky"

[client.modes.resize.keys]
"h"   = { server.pane.resize.direction = "left" }
"j"   = { server.pane.resize.direction = "down" }
"k"   = { server.pane.resize.direction = "up" }
"l"   = { server.pane.resize.direction = "right" }
"esc" = { client.mode = "normal" }

[client.modes.prefix]              # defined, empty, and entered by no key
kind = "oneshot"
```

How a user changes it:

```toml
[client.modes.normal.keys]
"alt-q"  = "none"                              # unbind: alt-q reaches the program
"ctrl-b" = { client.mode = "prefix" }          # opt into tmux habits

[client.modes.prefix.keys]
"ctrl-b" = { client.send = "ctrl-b" }          # send a literal ctrl-b to the pane
"c"      = { server.tab.create = {} }

[client.modes.resize]
clear_defaults = true                          # start this mode empty

[client.modes.resize.keys]
"left" = { server.pane.resize = { direction = "left", amount = 5 } }
"esc"  = { client.mode = "normal" }

[client.modes.panes]                           # a user-defined mode
kind = "sticky"

[client.modes.panes.keys]
"h" = { client.pane.focus.direction = "left" }
"esc" = { client.mode = "normal" }

[client.modes.normal.keys]
"alt-m" = [                                    # a list runs in order
  { server.tab.create.starter = "shell" },
  { server.pane.create = {} },
]
```

A binding is one action, a list of actions, or `"none"`. A list stops at the first action that fails.

Chord spelling comes from [crokey](https://docs.rs/crokey). Ship's docs point there and don't repeat it.

### Actions

The available-from-day-one column refers to the moment this change lands. "Not yet" actions are accepted in the config and report "not available yet" when used.

| Action | Arguments | Day one | Arrives with |
| --- | --- | --- | --- |
| `server.tab.create` | `starter` (`"shell"`; absent: empty tab) | yes | |
| `server.tab.close` | | yes | |
| `server.pane.create` | `direction` (right, down) | yes, direction ignored (FR-019) | layout |
| `server.pane.close` | | yes | |
| `server.pane.resize` | `direction`, `amount` (default: one step) | not yet | layout |
| `client.tab.next` / `client.tab.prev` | | yes | |
| `client.tab.select` | `row` (1-9) | not yet | layout |
| `client.tab.expand` / `client.tab.collapse` | | not yet | layout |
| `client.pane.next` / `client.pane.prev` | | yes | |
| `client.pane.focus` | `direction` | not yet | layout |
| `client.sidebar.toggle` | | not yet | layout |
| `client.mode` | mode name | yes | |
| `client.send` | chord | yes | |
| `client.detach` | | yes | |
| `client.config.reload` | | yes | |
| `"none"` | | yes | |

Zoom, swap, copy mode and scrollback editing join the list with their own changes.

### Command line

```text
before                                   after
ship tab rm <id>                         ship tab close [--tab ID | --pane ID]
ship tab get <id>                        ship tab get [--tab ID | --pane ID]
ship tab rename <id> [name]              ship tab rename [--tab ID | --pane ID] [NAME]
ship tab move <id> [parent|--before|--after]   ship tab move [--tab ID | --pane ID] [PARENT | --before ID | --after ID]
ship pane rm <id>                        ship pane close [--pane ID]
ship pane get <id>                       ship pane get [--pane ID]
ship pane rename <id> [name]             ship pane rename [--pane ID] [NAME]
ship pane create <tab> [-- CMD…]         ship pane create [--tab ID | --pane ID] [--name N] [--cwd DIR] [-- CMD…]
                                         ship config check [FILE]
```

`ship tab create [PARENT]` gains `--starter`, which opens the tab with a pane running the shell. Without it, the tab is empty, as today. `ship tab list` doesn't change.

How a command finds its target:

```text
in a pane       ship pane close            → the pane it runs in
in a pane       ship tab close             → the tab holding that pane
anywhere        ship pane close --pane p7  → p7
anywhere        ship tab close --pane p7   → the tab holding p7
outside panes   ship pane close            → error: pass --pane or run inside a pane
```

## User stories

### P1: Use Herdr's chords from the first run

**Given** no config file, **when** I run `ship`, **then** `alt-n` makes a tab with a shell, `alt-|` makes another pane in it, `alt-right` and `alt-left` move between tabs, `alt-tab` moves between panes, and `alt-q` detaches.

**Given** a key with no binding in normal mode, such as `ctrl-b`, **when** I press it, **then** the program in the pane receives it.

Independent verification: start with no config, perform each chord, and check the result with `ship tab list`.

### P1: Change a key and see it apply

**Given** a running client, **when** I bind `"alt-y" = { server.tab.create = {} }` in the config and save, **then** `alt-y` makes a tab without restarting anything, and `alt-n` still does too.

**Given** a running client, **when** I save a config with a mistake, **then** the client keeps the keys it had and shows what's wrong and where.

**Given** a config that binds `"alt-q" = "none"`, **when** I press `alt-q`, **then** the client stays attached and the program receives `alt-q`.

Independent verification: edit the file while a client is attached and press the changed keys.

### P1: Know a config is right before using it

**Given** a config with a typo in an action, a field, or a mode name, **when** I run `ship config check`, **then** it names each error with its location and exits non-zero.

**Given** a config with `"alt-n"` and `"Alt-N"` in the same mode, **when** I check it, **then** it reports both lines as one chord bound twice.

Independent verification: run `ship config check` on files with each kind of mistake.

### P2: Build my own modes

**Given** a sticky mode I defined, **when** I enter it, **then** its keys work until I enter another mode, keys it doesn't bind do nothing, and the client shows the mode's name.

**Given** a one-shot mode, **when** I enter it and press a bound key, **then** the action runs and I'm back in normal mode. **When** I press an unbound key instead, **then** I'm back in normal mode and nothing else happens.

**Given** a one-shot mode whose key enters another mode, **when** I press the chain, **then** each step works like a tmux key table.

Independent verification: define one mode of each kind and walk through each case.

### P2: Script the same actions

**Given** an agent in pane p5, **when** it runs `ship pane close`, **then** p5 closes, whatever I have selected.

**Given** any `server.` action in my config, **when** I drop `server.` and run the rest as a `ship` command, **then** it does the same thing to the target I name.

Independent verification: bind keys and run the matching CLI commands, then compare results.

## Non-goals

- **The CLI driving a client.** `client.` actions stay reachable only from keys and the mouse. A pane that moves your view (vim-navigator style) is a later decision, tied to plugins.
- **Keyboard enhancement flags** (the kitty keyboard protocol). Plain Alt chords work without them.
- **Command bindings.** A key can't run a shell command. Scripting goes through the `ship` CLI.
- **Configurable mouse gestures.** The layout change fixes them. They trigger the same actions keys do.
- **Printing the default config.** There is no `ship config defaults`.
- **A per-setting environment variable layer.** `SHIP_CONFIG` picks the file, and nothing else in the environment sets config values.
- **Opening a fresh client where the last client was.** That's a separate small change.
- **Floating panes and their key.** The notes put `alt-w` on toggling a floating pane, replacing Cyan's Herdr scratch shell. The action probably belongs under `server.pane`, since a pane's terminal has one size, but the floating-panes change decides it along with the rest of their design.
- **Settings beyond `shell`.** Each later setting arrives with its feature. Port, server URL, timing, backoff and `TERM` are not settings.
- **Splits, the sidebar, nav folding, resize and focus by direction.** The layout change builds them. This change only names their actions.

## Functional requirements

### File and loading

- **FR-001:** Ship MUST read its config from `--config <file>` if given, else `SHIP_CONFIG` if set, else `~/.config/ship/config.toml` (the platform's config directory).
- **FR-002:** With no config file, Ship MUST run on the defaults shown in Interface.
- **FR-003:** The client MUST read only `[client]`, and the server MUST read only `[server]`. An error in one part MUST NOT affect the process that reads the other.
- **FR-004:** Unknown fields, unknown actions and unknown action arguments MUST be errors.
- **FR-005:** Relative paths in the file, once any setting takes one, MUST resolve from the file's folder.
- **FR-006:** A client MUST reload its modes when the config file is saved, including when the file is replaced by rename or reached through a symlink. `client.config.reload` MUST also reload it.
- **FR-007:** A client that starts with a bad config MUST run on the defaults and show the error. A client whose config becomes bad on reload MUST keep its running config and show the error.
- **FR-008:** The server MUST read `shell` each time it starts a pane without an explicit command, so a saved change applies to the next pane. If the file is bad, the server MUST use the login shell and log the error.
- **FR-009:** `ship config check [FILE]` MUST report every error with its location and exit non-zero when there is one. It MUST report the duplicate chords and mode problems below.

### Keys and modes

- **FR-010:** Each mode MUST map each chord to exactly one action. Two spellings of one chord in the same mode, such as `"alt-n"` and `"Alt-N"` or `"ctrl-alt-x"` and `"alt-ctrl-x"`, MUST be an error.
- **FR-011:** Chords MUST be case-insensitive, as crokey parses them: `"alt-X"` means `alt-x`, and Shift is written as `shift-`.
- **FR-012:** A chord in the file MUST replace the default action on that chord in that mode. Other default chords MUST stay bound. `"none"` MUST unbind a chord, so the key reaches the program. `clear_defaults = true` MUST drop all of a mode's default bindings before the file's apply.
- **FR-013:** `normal` MUST always exist and MUST be where the client starts. Setting `kind` on `normal` MUST be an error.
- **FR-014:** Users MUST be able to define modes under `[client.modes.<name>]` with `kind` set to `sticky` or `oneshot`. A sticky mode MUST stay active until another mode is entered. A one-shot mode MUST return to normal after one key, bound or not.
- **FR-015:** In normal mode, an unbound key MUST reach the program. In a sticky mode other than normal, an unbound key MUST do nothing.
- **FR-016:** `client.mode` naming an undefined mode MUST be an error. Ship MUST NOT produce config warnings: `ship config check` reports exactly what loading would reject (A-2).
- **FR-017:** While a mode other than normal is active, the client MUST show the mode's name.
- **FR-018:** Any action MUST be bindable in any mode.

### Actions

- **FR-019:** Every action in the Actions table MUST be accepted in the config from day one. Using one whose feature isn't built MUST report "not available yet" and change nothing. Until the layout change lands, `server.pane.create` MUST add a pane running the shell to the selected tab and select it, ignoring `direction`.
- **FR-020:** A key binding MUST act on the client's selection. A `server.` action whose target isn't selected, such as `server.pane.close` with only a tab selected, MUST report that and change nothing.
- **FR-021:** `server.tab.create` from a key MUST create the tab after the selected tab under the same parent, or at the top level when nothing is selected, and select it.
- **FR-022:** `client.send` MUST send its chord to the selected pane as if typed.
- **FR-023:** `client.tab.next` and `client.tab.prev` MUST keep today's order and wrapping until the layout change redefines them as moving between visible rows.
- **FR-030:** A binding MUST be one action, a list of actions, or `"none"` (A-4). A list MUST run its actions in order, each after the one before has finished and the client shows its result (A-5). It MUST stop at the first action that fails, keep what the earlier actions did, and show the error. An empty list, or `"none"` inside a list, MUST be an error.

### Command line

- **FR-024:** Every `server.` action MUST be a `ship` command at the same path without `server.`, taking the same arguments as flags.
- **FR-025:** `ship tab rm` and `ship pane rm` MUST become `ship tab close` and `ship pane close`.
- **FR-026:** Commands that act on an existing tab or pane MUST take it as `--tab ID` or `--pane ID`. Commands that act on a tab MUST also accept `--pane ID` for the tab holding that pane, and `--tab` MUST win when both are given (A-3). Run inside a pane, `--pane` MUST default to that pane and `--tab` to the tab holding it. Run outside a pane without the flag, the command MUST fail and say to pass the flag or run inside a pane.
- **FR-027:** `ship pane create` MUST take its tab as `--tab ID` or `--pane ID`, with the same default as FR-026. Once layout lands, `--pane` may also name the pane to split; the new pane still goes in that pane's tab.
- **FR-028:** Command output MUST stay as today: one JSON result on stdout, or nothing for close.
- **FR-029:** `server.tab.create` MUST accept an optional `starter`. With `starter = "shell"`, the new tab MUST open with one pane running the shell from FR-008, and a key binding MUST select that pane. Without it, the tab MUST be empty. `ship tab create --starter` MUST do the same.

## Success criteria

- **SC-001:** With no config file, every day-one default chord in Interface does what its action says, checked once each.
- **SC-002:** A saved config change applies to an attached client without a restart, and a broken save leaves the client's keys unchanged and shows the error.
- **SC-003:** `ship config check` catches a typo'd action, field and mode name, and a duplicate chord spelling, each with its location.
- **SC-004:** For every day-one `server.` action, the key binding and the CLI command produce the same result on the same target.
- **SC-005:** Cyan replaces his Herdr habits with Ship's defaults for a working day and finds no chord he expects missing, outside the layout actions.

## Open unknowns

None. U-1 was resolved in conversation (FR-029). U-2, floating panes, was deferred to its own change (Non-goals).
