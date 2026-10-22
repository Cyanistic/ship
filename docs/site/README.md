# Overview

Ship is a small terminal workspace for running several coding agents at once. A background server keeps your shells, editors and agents alive in tabs and split panes after the window closes, the sidebar shows which agent is working, blocked, finished or idle, and agents can open panes and hand work to each other through a CLI that prints JSON.

Ship is a work in progress and has no releases yet. The [README](../../README.md#what-works-today) keeps the honest list of what works today and what isn't built.

## Install

There are no releases yet, so Ship builds from source. You need a recent stable [Rust toolchain](https://www.rust-lang.org/tools/install), [Zig](https://ziglang.org/download/) 0.16 on your `PATH`, and `git` with a network connection for the first build, which fetches Ghostty's source. On Linux you also need `pkg-config` and ALSA's development headers for sound (`libasound2-dev` on Debian and Ubuntu, `alsa-lib-devel` on Fedora).

```sh
git clone https://github.com/shipmux/ship.git
cd ship
cargo install --path crates/ship
```

## First run

Run `ship`. It starts a server in the background if one isn't running, restores its saved tabs, and opens on the first tab's first pane. With nothing to restore, it creates a tab with your shell. Press `alt-q` to detach; the server and everything in it keeps running, and `ship` brings you back.

> [!NOTE]
> On macOS, set your terminal to use Option as Alt (often called "Option as Meta"), or Ship's keys will type special characters instead.

## The keys to start with

All of Ship's default keys are Alt chords, and any other key goes to the program in the selected pane. These few are enough for a first session:

| Key | Does |
| --- | --- |
| `alt-n` | New tab with a shell |
| `alt-\|` / `alt--` | Split the pane right / down |
| `alt-h` / `alt-j` / `alt-k` / `alt-l` | Move to the pane left, below, above or right |
| `alt-a` | Jump to the next agent that needs you |
| `alt-x` | Close the tab |
| `alt-q` | Detach |

[Tabs](tabs.md) and [Panes](panes.md) list the rest, and [Configuration](configuration.md#keys-and-modes) shows how to rebind them.

## Let your agent set you up

A coding agent can walk you through installing and configuring Ship. Paste this into one:

```text
Help me understand and set up Ship. Read https://shipmux.dev/agent-guide.md first, then ask me how I like to work before you change anything.
```

Agents that want the whole map can start from [llms.txt](https://shipmux.dev/llms.txt), which links a plain markdown copy of every page.

## Guides

This picks up where the [README](../../README.md) leaves off. The examples assume `ship` is on your `PATH` and [`jq`](https://jqlang.org) is installed. `ship --help`, or `--help` on any command, lists every option.

- [Windows and the server](windows-and-the-server.md)
- [Tabs](tabs.md)
- [Panes](panes.md)
- [Scrolling, copying and the mouse](scrolling-copying-and-the-mouse.md)
- [Agents](agents.md)
- [Saved tabs](saved-tabs.md)
- [Configuration](configuration.md)
- [Shell completions](shell-completions.md)
- [Troubleshooting](troubleshooting.md)
- [Upgrading](upgrading.md)

## Scripting

For scripts and agents that drive Ship from the command line.

- [Scripting Ship](scripting/README.md)
- [Reading output](scripting/reading-output.md)
- [Typing into a pane](scripting/typing-into-a-pane.md)
- [Handing work to another agent](scripting/handing-work-to-another-agent.md)
- [Reading command errors](scripting/reading-command-errors.md)
- [Steering a window](scripting/steering-a-window.md)
