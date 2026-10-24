# Overview

<!--
Scope: A one-paragraph intro, then a first session: install, first run, the few keys for day one, a taste of the CLI, and the link lists that make up the site's sidebar.
Not here: the full key list (keys.md), settings (configuration.md), anything a first session can skip.
-->

Ship is a terminal multiplexer, like tmux or Zellij, except it treats coding agents as first-class citizens instead of just another program in a pane. Your shells and agents run on a background server, so they keep going after you close the window, and agents that support the [Program Status Protocol](https://www.superlogical.com/rex/docs/build/program-status) show up in the sidebar as working, waiting on you or done. Everything is also a `ship` command that prints JSON, so agents can open panes and hand work to each other. [Agents](agents.md) and [Handing work to another agent](scripting/handing-work-to-another-agent.md) show what both look like.

Ship is a work in progress and has no releases yet. The [README](../../README.md#what-works-today) lists what works today and what isn't built yet.

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

That's all you need for now. [Keys](keys.md) lists the rest when you want them.

## Try the CLI

Leave `ship` open in one terminal and try this from another:

```sh
$ TAB=$(ship tab create --name demo | jq -r .id)
$ PANE=$(ship pane create --tab "$TAB" --exit-behavior keep -- sh -c 'seq 100' | jq -r .id)
$ ship pane wait --pane "$PANE" --status exited >/dev/null
$ ship pane read --pane "$PANE" | tail -3
98
99
100
$ ship tab close --tab "$TAB"
```

The demo tab shows up in the sidebar as soon as you create it and goes away when you close it. Inside a Ship pane you can leave out `--tab` and `--pane`, since commands default to the pane they run in. [Scripting Ship](scripting/README.md) picks up from here.

## Let your agent set you up

A coding agent can walk you through installing and configuring Ship. Paste this into one:

```text
Help me understand and set up Ship. Read https://shipmux.dev/agent-guide.md first, then ask me how I like to work before you change anything.
```

Agents that want the whole map can start from [llms.txt](https://shipmux.dev/llms.txt), which links a plain markdown copy of every page.

## Guides

These cover day-to-day use, roughly in the order you'll run into things. The examples assume `ship` is on your `PATH` and [`jq`](https://jqlang.org) is installed.

- [Windows and the server](windows-and-the-server.md)
- [Tabs](tabs.md)
- [Panes](panes.md)
- [Scrolling, copying and the mouse](scrolling-copying-and-the-mouse.md)
- [Agents](agents.md)
- [Keys](keys.md)

## When you need them

You can skip these until something sends you looking. `ship --help`, or `--help` on any command, lists every option.

- [Saved tabs](saved-tabs.md)
- [Configuration](configuration.md)
- [Shell completions](shell-completions.md)
- [Troubleshooting](troubleshooting.md)

## Scripting

For scripts and agents that drive Ship from the command line.

- [Scripting Ship](scripting/README.md)
- [Reading output](scripting/reading-output.md)
- [Typing into a pane](scripting/typing-into-a-pane.md)
- [Handing work to another agent](scripting/handing-work-to-another-agent.md)
- [Reading command errors](scripting/reading-command-errors.md)
- [Steering a window](scripting/steering-a-window.md)
