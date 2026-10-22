<div align="center">
<img src="website/public/marks/app-large.svg" height="216" width="216" alt="A pixel sailing ship with three crewmates on deck">

**Ship - A Terminal Workspace for Working Alongside Coding Agents**

[Overview](#overview) •
[What works today](#what-works-today) •
[Installation](#installation) •
[Keys](#keys) •
[Configuration](#configuration) •
[Guide](docs/site/README.md) •
[License](#license)

</div>

> [!WARNING]
> **Ship is a work in progress.** There are no releases yet. See [what works today](#what-works-today) and [what isn't built yet](#not-built-yet).

## Overview

Ship is a terminal multiplexer, like tmux or Zellij, except it treats coding agents as first-class citizens instead of just another program in a pane. Your shells and agents run in tabs and split panes on a background server, so they keep going after you close the window.

Programs can tell Ship what they're doing through a recently proposed protocol, the [Program Status Protocol](https://www.superlogical.com/rex/docs/build/program-status), and Ship shows it in the sidebar:

```text
─ agents ─────────────
▲ claude blocked
ship › api
● codex working
ship › docs
◆ pi unseen
site
```

So you can see what's working, what's waiting on you and what finished without flipping through tabs. Nothing about the protocol is specific to agents, but agents are where it's actually useful, and Claude Code, Codex, Pi and crush already support it. Support is up to each agent's authors, so there's no list of agents in Ship to keep up to date.

Everything Ship does is also a `ship` command that prints JSON, and it's the same binary your agent calls from bash. So an agent running in Ship can start another one and hand it work:

```sh
$ ship pane create --name worker -- claude
{"id":"pane:8df435b8…","name":"worker","command":["claude"],"status":{"state":"running"},…}

$ ship pane send --pane pane:8df435b8… "write a haiku about ships" --key enter --wait
{"id":"pane:8df435b8…",…,"program":{"":{"state":"done","app":"claude"}}}

$ ship pane read --pane pane:8df435b8…
write a haiku about ships
wooden hull at dusk
three small crewmates keep the watch
the build turns green
```

No SDK or plugin needed, same reason you'd just run grep instead of rewriting it in every language. [Handing work to another agent](docs/site/scripting/handing-work-to-another-agent.md) has the full loop.

The name started as a joke about shipping software faster, and the ship and crew theme grew out of that.

### Why another multiplexer?

[Herdr](https://github.com/herdrdev/herdr) already does a lot of this, and Ship is heavily inspired by it, but it tries to cover most of what Herdr does day to day with a much smaller codebase, by using libraries for the hard parts (terminal emulation from [Ghostty](https://ghostty.org), PTYs, transport) and only writing the tab, pane and agent parts itself.

## What works today

- **Detaching.** A background server owns your terminals. Close the window or press `alt-q` to detach, and everything keeps running until you come back.
- **Saved tabs.** After a server restart or reboot, your tabs, splits and directories come back with fresh shells. Programs, agent conversations and scrollback aren't restored.
- **Agent status.** Agents that speak the Program Status Protocol get a sidebar entry showing whether they're working, blocked, finished unseen or idle, and their tabs show it too. A sound plays when one finishes or needs you while you're looking at something else, and `alt-a` jumps to it.
- **Nested tabs.** A tab can hold its own tabs, so a big task can keep its smaller ones together. The sidebar shows the tree, and you can fold the parts you aren't looking at or drag rows to reorder and nest tabs.
- **Split panes.** Split, zoom, swap and resize from the keyboard, or drag shared borders to resize. New splits open in the directory you were working in.
- **Several windows.** Attach from more than one terminal at once. Each one picks its own tab and pane, and the work underneath is shared.
- **Scrollback and copy.** Scroll back through a pane's history, and drag to select and copy text.
- **A CLI that prints JSON.** Everything the server does is also a `ship` command, so scripts and agents can create panes, read output and change which tab and pane a specific window has selected.
- **Handing work to another agent.** `ship pane send` types a prompt or keys into a pane, and `ship pane wait` blocks until the agent there stops working, so one agent can give another a task and read the answer.
- **Custom keys and menus.** Rebind anything, build tmux-style prefix modes, and change the right-click menus for panes, tabs and the sidebar. Programs that use the mouse can get right-clicks instead of Ship's menu, per pane. Config changes apply when you save.

Linux and macOS are the priority platforms. Windows is best effort.

### Not built yet

- Releases and packages.
- Searching a pane's history, and a keyboard copy mode. For now, `ship pane read --lines N | grep ...` covers search.
- Resuming agent conversations or rerunning programs after a restart. Saved tabs come back with fresh shells.
- Closing a pane automatically when its program exits.
- Connecting to another machine's server directly. The server only accepts local connections for now, so [use an SSH tunnel](docs/site/windows-and-the-server.md#using-ship-on-another-machine).
- Testing on Linux. Most features have only been checked on macOS so far.

## Installation

There are no releases yet, so you'll need to build from source. You need:

- a recent stable [Rust toolchain](https://www.rust-lang.org/tools/install)
- [Zig](https://ziglang.org/download/) 0.16 on your `PATH`, to build Ghostty's terminal library
- `git` and a network connection on the first build, which fetches Ghostty's source
- on Linux, `pkg-config` and ALSA's development headers for sound (`libasound2-dev` on Debian and Ubuntu, `alsa-lib-devel` on Fedora)

Then:
```sh
git clone https://github.com/shipmux/ship.git
cd ship
cargo install --path crates/ship
```

Run `ship`. It starts a server in the background if one isn't running, restores its saved tabs, and opens on the first tab's first pane. With nothing to restore, it creates a tab with your shell. Press `alt-q` to detach; the server and everything in it keeps running, and `ship` brings you back. `ship server stop` shuts the server down and ends every program in it.

On macOS, set your terminal to use Option as Alt (often called "Option as Meta"), or Ship's keys will type special characters instead.

### A quick look at the CLI

Leave `ship` open in one terminal and try this from another:
```sh
$ TAB=$(ship tab create --name demo | jq -r .id)
$ PANE=$(ship pane create --tab "$TAB" -- sh -c 'seq 100' | jq -r .id)
$ ship pane read --pane "$PANE" | tail -3
98
99
100
$ ship tab close --tab "$TAB"
```

The demo tab shows up in the sidebar as soon as you create it and goes away when you close it. Inside a Ship pane you can leave out `--tab` and `--pane`, since commands default to the pane they run in. The [guide](docs/site/README.md) walks through the rest, and `ship --help` lists every command.

## Keys

All of Ship's default keys are Alt chords. Any other key goes to the program in the selected pane.

| Key | Does |
| --- | --- |
| `alt-n` | New tab with a shell |
| `alt-shift-n` | New tab nested under the current one |
| `alt-x` | Close the tab |
| `alt-\|` / `alt--` | Split the pane right / down |
| `alt-shift-x` | Close the pane |
| `alt-left` / `alt-right` | Previous / next tab |
| `alt-h` / `alt-j` / `alt-k` / `alt-l` | Move to the pane left, below, above or right |
| `alt-tab` | Next pane in the tab |
| `alt-a` | Jump to the next agent that needs you |
| `alt-shift-h` / `j` / `k` / `l` | Swap the pane with its neighbor in that direction |
| `alt-f` | Zoom the pane to fill the tab, or unzoom |
| `alt-r` | Resize mode (below) |
| `alt-g` | Tabs mode (below) |
| `alt-b` | Hide or show the sidebar |
| `alt-pageup` / `alt-pagedown` | Scroll the pane up / down a page |
| `alt-q` | Detach |

In resize mode, `h` / `j` / `k` / `l` move the pane's edge, `shift-h` / `shift-l` resize the sidebar and `shift-j` / `shift-k` move the line between tabs and agents. In tabs mode, `j` / `k` move between tabs and `h` / `l` fold and unfold. `esc` leaves either mode.

The mouse works too: click a pane, a tab or an agent to select it, scroll with the wheel, drag to select and copy text, drag pane borders and the sidebar's dividers to resize, and drag sidebar rows to move tabs. Right-click for actions, or click `≡` beside `tabs` for the global menu.

## Configuration

Ship reads `~/.config/ship/config.toml` (or `$XDG_CONFIG_HOME/ship/config.toml`). Without one, it uses its defaults. A running client picks up changes as soon as you save.

```toml
[server]
shell = "/bin/bash"            # default: your login shell

[client]
sidebar_width = 40

[client.modes.normal.keys]
"alt-q"  = {}                                     # alt-q reaches the program
"ctrl-b" = { action = { client.mode = "prefix" } } # tmux habits

[client.modes.prefix]
kind = "oneshot"                               # back to normal after one key

[client.modes.prefix.keys]
"c"      = { action = { server.tab.create = { starter = "shell", select = true } } }
"ctrl-b" = { action = { client.send = "ctrl-b" } }  # a literal ctrl-b for the pane
```

`ship config check` points at the first mistake in the file, with its line and column. The [guide](docs/site/configuration.md) covers every setting, custom modes and how key actions map onto CLI commands.

## License

Ship is licensed under the [GNU General Public License v3.0 or later](LICENSE). The vendored [`ratatui-ghostty`](vendor/ratatui-ghostty) keeps its own MIT license.

Contributions are accepted under `MIT OR Apache-2.0`, as the pull request template states, so the project can change its license later without asking each contributor.
