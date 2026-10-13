<div align="center">
<img src="assets/ship.svg" height="220" width="220" alt="A tall sailing ship crewed by three little terminal crewmates">

**Ship - A Terminal Workspace for Working Alongside Coding Agents**

[Overview](#overview) •
[What works today](#what-works-today) •
[Installation](#installation) •
[Keys](#keys) •
[Configuration](#configuration) •
[Guide](docs/guide.md)

</div>

> [!WARNING]
> **Ship is a work in progress.** There are no releases yet, and the agent features that motivate it aren't built yet. [What works today](#what-works-today) is the honest list.

## Overview

If you run a few coding agents at once, you end up with a pile of terminals: an agent in one, an editor in another, a shell running tests, another agent somewhere else. Typing the commands is the easy part. The hard part is keeping track of it all. Which agent finished? Which one is stuck waiting on you? Where did you leave it?

Ship puts all of that in one place. It keeps your shells, editors and agents together, keeps them running when you close the window, and is heading toward showing you which agent needs attention and letting agents drive the workspace themselves.

The name treats agents as your crew, and it's also about shipping software.

### Why another multiplexer?

tmux and Zellij keep terminals alive, but they don't know what an agent is. [Herdr](https://github.com/herdrdev/herdr) does, and Ship is heavily inspired by it. Ship's angle is getting most of that daily value from a much smaller codebase: it leans on solid libraries for the mechanical parts, like terminal emulation ([Ghostty](https://ghostty.org)'s), PTYs and transport, and only writes the parts that are actually Ship. The goal is a codebase small enough to fit in your agent's (sorry, crewmate's) context window.

## What works today

- **Work that keeps running.** A background server owns your terminals. Close the window or press `alt-q` to detach, and everything keeps running until you come back.
- **Tabs inside tabs.** Tabs can nest, so a big task can hold its own smaller ones. A sidebar shows the tree, and you can fold the parts you aren't looking at.
- **Split panes.** Split, zoom, swap and resize from the keyboard. New splits open in the directory you were working in.
- **More than one window.** Attach from several terminals at once. Each one picks its own tab and pane, while the work underneath is shared.
- **Scrollback and copy.** Scroll back through a pane's history, and drag to select and copy text.
- **A scriptable CLI.** Every action is also a `ship` command that prints JSON, so scripts and agents can create panes and read what another pane printed.
- **Your keys.** Rebind anything, build tmux-style prefix modes, and see changes the moment you save the config.

Linux and macOS are the priority platforms. Windows is best effort.

### Not built yet

- Agent status: seeing which agents are working, blocked, idle or done.
- An agent list and notifications to jump to whichever one needs you.
- Sending input to a pane, or waiting for another agent to finish, from the CLI.
- Restoring tabs and panes after the server or machine restarts. For now, stopping the server ends everything in it.
- Releases and packages.

## Installation

There are no releases yet, so you'll need to build from source. You need:

- a recent stable [Rust toolchain](https://www.rust-lang.org/tools/install)
- [Zig](https://ziglang.org/download/) 0.16 on your `PATH`, to build Ghostty's terminal library
- `git` and a network connection on the first build, which fetches Ghostty's source

Then:
```sh
git clone https://github.com/Cyanistic/ship.git
cd ship
cargo install --path crates/ship
```

Run `ship`. It starts a server in the background if one isn't running, then opens on a tab with your shell. Press `alt-q` to detach; the server and everything in it keeps running, and `ship` brings you back. `ship server stop` shuts the server down and ends every program in it.

On macOS, set your terminal to use Option as Alt (often called "Option as Meta"), or Ship's keys will type special characters instead.

### A quick look at the CLI

Leave `ship` open in one terminal and try this from another:
```sh
TAB=$(ship tab create --name demo | jq -r .id)
PANE=$(ship pane create --tab "$TAB" -- sh -c 'seq 100' | jq -r .id)
ship pane read --pane "$PANE" --lines 5
ship tab close --tab "$TAB"
```

The new tab shows up in the sidebar as soon as you create it, and `ship pane read` prints the last lines the pane wrote. Inside a Ship pane you can leave out `--tab` and `--pane`; commands default to the pane they run in. The [guide](docs/guide.md) walks through the rest, and `ship --help` lists every command.

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
| `alt-shift-h` / `j` / `k` / `l` | Swap the pane with its neighbor in that direction |
| `alt-f` | Zoom the pane to fill the tab, or unzoom |
| `alt-r` | Resize mode: `h` / `j` / `k` / `l` move the pane's edge, `esc` leaves |
| `alt-g` | Tabs mode: `j` / `k` move between tabs, `h` / `l` fold / unfold, `esc` leaves |
| `alt-b` | Hide or show the sidebar |
| `alt-pageup` / `alt-pagedown` | Scroll the pane up / down a page |
| `alt-q` | Detach |

The mouse works too: click a pane or a sidebar row to select it, scroll with the wheel, and drag to select and copy text.

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
"c"      = { action = { server.tab.create.starter = "shell" } }
"ctrl-b" = { action = { client.send = "ctrl-b" } }  # a literal ctrl-b for the pane
```

`ship config check` points at the first mistake in the file, with its line and column. The [guide](docs/guide.md#configuration) covers every setting, custom modes and how key actions map onto CLI commands.
