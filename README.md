<div align="center">
<img src="assets/ship.svg" height="220" width="220" alt="A tall sailing ship crewed by three little terminal crewmates">

**Ship - A Terminal Workspace for Working Alongside Coding Agents**

[Overview](#overview) •
[Features](#features) •
[Status](#status) •
[Installation](#installation)

</div>

> [!WARNING]
> **Ship is a work in progress.** There are no releases yet, and it can't host a terminal today. The [Features](#features) section describes where Ship is headed. [Status](#status) covers what works right now.

## Overview

If you run a few coding agents at once, you end up with a pile of terminals: an agent in one, an editor in another, a shell running tests, another agent somewhere else. Typing the commands is the easy part. The hard part is keeping track of it all. Which agent finished? Which one is stuck waiting on you? Where did you leave it?

Ship puts all of that in one place. It keeps your shells, editors and agents together, keeps them running when you close the window, shows you which agent needs attention, and lets agents drive the workspace themselves.

The name treats agents as your crew, and it's also about shipping software.

## Features

### Organize your work
- **Tabs and split panes:**
  Group related work into tabs. Tabs can nest inside other tabs, so a big task can hold its own smaller ones.

- **Multiple clients, independent views:**
  Attach from more than one terminal at a time. Each client keeps its own selection and focus while sharing the same underlying work.

### Leave and come back
- **Work that keeps running:**
  Ship's server owns your terminals, so closing the window doesn't stop anything. Reattach later and everything is where you left it.

- **Restore after a restart:**
  If the server or machine restarts, Ship rebuilds your tabs and panes, and resumes agent conversations where the agent supports it.

### Know which agent needs you
- **Agent status at a glance:**
  See which agents are working, blocked, idle or done with something you haven't looked at yet. When Ship can't tell, it says so instead of guessing.

- **Jump to the one that needs you:**
  An agent list and notifications get you to a waiting agent without checking every pane.

### Let agents operate the workspace
- **A command-line interface agents can use:**
  Agents can open panes, run commands, send input, read output, and wait for another agent to finish, all through `ship` itself.

### Small and fast
- **Built on existing libraries:**
  Ship leans on solid libraries for the mechanical parts (terminal emulation, PTYs, transport) and only writes the parts that are actually Ship. The goal is a codebase small enough to actually read. Small enough, in fact, that the whole thing fits in your agent's (sorry, crewmate's) context window.

- **Snappy:**
  Ship should feel at least as fast as tmux, Zellij and Herdr.

- **Linux and macOS first:**
  Both are priority platforms. Windows is best effort.

### Why another multiplexer?
tmux and Zellij keep terminals alive, but they don't know what an agent is. [Herdr](https://github.com/herdrdev/herdr) does, and Ship is heavily inspired by it. Ship's angle is getting most of that daily value from a much smaller codebase.

## Status

Today, `ship` runs a background server that keeps track of nested tabs and panes. You can create, rename, move and remove them from the command line. Running `ship` opens a full-screen client on the whole server that updates as things change. Each client keeps its own selection and reconnects on its own if the connection drops.

Panes are placeholders for now: there's no terminal inside them yet. Next up is giving them real PTYs, so a pane runs an actual shell, editor or agent that the server keeps alive while clients come and go.

## Installation

There are no releases yet, so you'll need to build from source. You need a recent stable [Rust toolchain](https://www.rust-lang.org/tools/install).

1. Clone the repo
```sh
git clone https://github.com/Cyanistic/ship.git
cd ship
```
2. Build it
```sh
cargo build --release
```
3. Run it, or put it on your PATH with `cargo install --path crates/ship` so the examples below work as written
```sh
./target/release/ship
```

Running `ship` finds the local server or starts one in the background, then opens the client. A server that `ship` starts begins with one tab holding a shell in your home directory, and the client opens on it. Press `C-b d` to detach. The server keeps running after `ship` exits, and `ship server stop` stops it.

To see tabs change live, leave `ship` open in one terminal and add a tab from another:
```sh
TAB=$(ship tab create --name notes | jq -r .id)
ship pane create "$TAB" --name scratch
```
Then press `C-b )` and `C-b (` in the client to move between top-level tabs.

Run `ship --help` to see everything else.
