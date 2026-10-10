# Prompts and plugins

Status: undecided. Recorded on 2026-10-09 from the layout and sidebar grilling, where renaming from the UI was deferred because Cyan could not settle how the UI should ask for input yet. Nothing here is decided or authorizes implementation.

## The question

`ship tab rename` and `server.tab.rename` take the name up front, and an omitted name clears it. A UI rename has to ask for the name first. The question is how the UI collects input like that without growing a second, divergent way to run actions. The same question returns for any prompt: "run a command", "name the new tab", and anything a user invents.

## Options considered for rename

- **(a) Generic prompt in config.** `client.prompt = { label, fill, run }`: a prompt whose answer fills one field of a `server.` action. Rejected: the obvious next asks are a default value (`#{tab.name}`, as tmux's `command-prompt -I "#W"`), then conditions, which ends in a template language.
- **(a2) Command line with pre-typed text.** `client.command = "tab rename "`, parsed by the CLI's own clap parser. Same objection: pre-filling the current name needs substitution.
- **(b) Built-in prompts.** A client-only overlay that asks, then sends the same request the CLI sends. This is what Herdr does (below). The leading name was `client.prompt.tab.rename = {}`, a nested action like every other, rather than a string value like `client.mode`. `client.mode` takes a string only because modes are user-defined; built-in prompts are a closed set, so typos fail at config load.
- **(c) Defer.** Chosen for the layout change.

Two slopes showed up, both to avoid:

- **Templating:** any substitution in binding text leads toward tmux's formats.
- **Generic forms:** "`client.prompt.<x>` asks for the arguments of `server.<x>`" sounds like a rule, but `server.pane.create` has a name, a directory, a direction, a tab ID and an argv. One input per empty field needs field types, pickers, validation and ordering: a small UI framework. The leaning was a closed list of hand-designed, one-field prompts, with anything wider done by a program.

Also still open: when the prompt is emptied and Enter pressed, whether the name is cleared (the CLI's meaning) or nothing happens (Herdr). The leaning was to clear.

## Herdr's rename

From Herdr's source at 2563803d:

1. `rename_tab` is a fixed keybind action, not something composed in config (`src/input/keybindings.rs:36`).
2. It opens a client overlay pre-filled from the client's latest snapshot (`src/client/shell/overlay_input.rs:404`).
3. Enter turns the text into the same API method the CLI uses, `Method::TabRename` (`overlay_input.rs:933`).

One `ClientRenameOverlay` type serves renaming a workspace, a tab or a pane, and naming a new tab or workspace (`prompt_new_tab_name`). Empty input, or an auto name left unchanged, sends nothing for tabs, so the derived label isn't pinned by accident. The right-click menu opens the same overlay (`src/client/shell/context_menu.rs`). Plugins and config cannot reuse this prompt; a plugin that wants input opens its own popup pane.

## Herdr's plugins

A plugin is a directory with a `herdr-plugin.toml` manifest declaring `[[actions]]`, `[[events]]` hooks, `[[panes]]`, `[[startup]]` commands and link handlers. Each entry is an argv command. Source: Herdr's [plugin docs](https://github.com/herdrdev/herdr/blob/d6b40d4edd550ccea081f089605a64314f8c8b27/docs/next/website/src/content/docs/plugins.mdx); see also [config and plugin ownership](config-plugin-ownership.md).

- **Acting:** "the entire Herdr CLI is the plugin API". Plugins call `herdr ...` through `HERDR_BIN_PATH`, with context in `HERDR_PANE_ID`, `HERDR_TAB_ID` and `HERDR_PLUGIN_CONTEXT_JSON`.
- **Showing:** only by running a terminal program in a pane, placed as an overlay, popup, split, tab or zoomed pane. "Native non-terminal plugin UI" is not part of plugin v1. Sidebar text comes from metadata tokens a plugin reports, for example `herdr pane report-metadata $PANE --source gh-pr --token pr="#123 ✓"`.
- **Where it runs:** the server spawns plugin commands (`src/app/api/plugins/runtime.rs:121`). A key bound with `[[keys.command]] type = "plugin_action"` makes the client send the invocation, adding context only it knows, such as the current text selection (`src/client/shell/actions.rs:203`).
- **Real plugins checked:** `wyattjoh/herdr-plugin-gh-pr` (event hooks plus metadata tokens, no UI), `persiyanov/herdr-reviewr` (a Rust TUI in a split pane), `cloudmanic/herdr-plus` (Go fuzzy pickers in popups), `nikok6/herdr-mirror` (its own daemon syncing remote servers over SSH).
- **Where the cost lands:** on plugin authors. gh-pr notes "herdr has no background-poll mechanism" and "no plugin-extensible right-click menu"; startup hooks are one-shot, not supervised daemons; herdr-plus documents Windows path workarounds.

This fits Ship: every `server.` action is already a `ship` command printing JSON, panes already get `SHIP_PANE_ID`, and plugins would sit outside the implementation budget and cross the network boundary for free.

## Claude Code hooks

From Claude Code's documented hook model, not re-checked in this session: hooks are commands registered per event (`PreToolUse`, `PostToolUse`, `UserPromptSubmit`, `Stop` and others) with an optional matcher. The host spawns the command with event JSON on stdin and waits. Exit 0 continues, exit 2 blocks with stderr shown as the reason, and JSON on stdout can carry a structured decision. Unlike Herdr plugins, "before" hooks can veto, which gives programs a say inside the host's flow without an embedded language. A per-event spawn makes this suitable only for rare events such as closing a tab.

## Program drives the flow

The direction that came out of the conversation: config never computes values. A binding names one program, and the program prompts, computes and calls `ship`. Ordering is line order and failure is `set -e`, so Ship defines no evaluation semantics. Making config fields computable, for example `cwd = { command = [...] }` or a prompt with a callback, would make Ship define order, failure, parallelism, caching and which side runs each command.

```toml
"alt-;" = { server.pane.create = { command = ["~/.config/ship/run.sh"], floating = true } }
```

```sh
#!/bin/sh
set -e
printf 'run: '; read -r cmd
root=$(git rev-parse --show-toplevel)
ship pane create --cwd "$root" -- sh -c "$cmd"
```

A separate `server.run` action with a `placement` field was considered and dropped: placement is a pane property, and `server.pane.create` already takes a command. `floating`, its size and "close when the program exits" belong to the floating panes change. Programs with no UI, such as background hooks, are left until hooks are designed.

Cyan's concern with this direction is user experience: a bare `read` prompt in a pane looks rough next to Herdr's native overlay, and nobody should write a script to rename a pane. The layering discussed: built-in prompts for the client's own few needs, and programs in floating panes for the long tail, where tools like fzf and gum make them look native.

What scripts cannot reach: client-local state (sidebar, selection, folds), since `ship` only talks to the server, and per-keystroke decisions such as passing a key through to Neovim.

## Embedded scripting

Lua (`mlua`) or `rhai` was weighed for variables and logic in config. Costs: a second public API surface, async action lists (the action queue waits on server revisions between actions), errors moving from config load to key press, and a VM per side if server hooks are wanted. Owned glue estimated at 300 to 500 lines for a generic `ship.run(action, args)`, 1,000 to 2,000 for a pleasant typed API. Leaning: defer; programs plus built-in actions cover the known needs. Zellij's WebAssembly plugins were judged a poor fit: the sandbox protects against strangers' code, which a single-user tool doesn't need, at the cost of wasmtime, a host API and an SDK.

## Process spawn cost

Loop averages on Cyan's macOS machine, 2026-10-09:

| Command | Per call |
| --- | --- |
| `/usr/bin/true` | about 1.1 ms |
| `ship --help` | about 2.9 ms |
| `ship server status` (a localhost round trip) | about 3.9 ms |

Linux is unmeasured. Script runtimes add their own startup (Node or Bun tens of milliseconds, typically). Spawning per event is fine for rare events; tight loops want a long-lived connection such as a future `ship watch` stream.
