# Ship agent guide

You are helping a person install and set up Ship, a terminal workspace for running several coding agents at once. A background server owns tabs, panes and the programs in them; `ship` opens a window onto that server; agents report their state and show up in a sidebar. Work through the steps below in order, and treat the person as the decision maker: ask before you install anything, edit a file or stop a server.

Every page of these docs has a plain markdown copy. [llms.txt](https://shipmux.dev/llms.txt) lists them all. Read a page before you explain the feature it covers, and quote the docs rather than your memory of other multiplexers: Ship's keys, config and CLI are its own.

## What Ship does not do yet

Ship has no releases yet. Keep these out of your suggestions, because they are not built:

- release binaries or packages (Ship builds from source)
- searching a pane's history, or a keyboard copy mode (`ship pane read --lines N | grep ...` covers search)
- resuming agent conversations or rerunning programs after a restart (saved tabs come back with fresh shells)
- closing a pane automatically when its program exits
- connecting to another machine's server directly (use an SSH tunnel, see [Windows and the server](windows-and-the-server.md#using-ship-on-another-machine))

The [README](../../README.md#not-built-yet) is the source of truth for this list. If it disagrees with this page, the README wins.

## 1. Learn how the person works

Ask which terminal emulator and shell they use, which operating system, which coding agents they run, and whether they already have tmux or Zellij habits they want to keep. Done when you know the OS, terminal, shell and agents.

## 2. Install

Ship builds from source. Check each prerequisite with a command before installing anything, and report what is missing:

- a recent stable Rust toolchain: `cargo --version`
- Zig 0.16 on `PATH`: `zig version` prints `0.16.x`
- `git`, and network access for the first build, which fetches Ghostty's source
- on Linux, `pkg-config` and ALSA's development headers (`libasound2-dev` on Debian and Ubuntu, `alsa-lib-devel` on Fedora)

Then, with the person's go-ahead:

```sh
git clone https://github.com/shipmux/ship.git
cd ship
cargo install --path crates/ship
```

Done when `ship --version` prints a version.

## 3. Prepare the terminal

On macOS, the terminal must send Option as Alt (often called "Option as Meta"), or Ship's Alt keys type special characters. Tell the person where that setting lives in their terminal, or ask them to find it. Copying uses OSC 52, which most modern terminals support and macOS Terminal.app does not; mention it if they use Terminal.app.

Done when the person confirms the setting, or their terminal needs none.

## 4. First run

Ask the person to run `ship` in their own terminal: it is a full-screen program, so start it for them only if you can hand them the terminal. It starts the server, opens one tab with a shell, and `alt-q` detaches while everything keeps running. `ship server status` tells you whether a server is up.

Walk them through the first keys from the [Overview](README.md#the-keys-to-start-with). Done when they have split a pane and detached once.

## 5. Agents

Claude Code, Codex, Pi and crush report their state with no setup. Anything else that speaks OSC 7501, the Program Status Protocol, works too. Two cases need attention, both on the [Agents](agents.md) page:

- An agent that crashes leaves its last state behind unless the shell marks its prompts. For zsh, the snippet under "When an agent's status looks stuck" fixes that; offer to add it to `~/.zshrc`.
- Agents running inside tmux inside Ship can't report anything.

Done when an agent the person uses shows up in the sidebar's agent list.

## 6. Configuration, only what they ask for

Ship runs on its defaults with no config file. If the person wants changes, read [Configuration](configuration.md) first, then:

1. Write only the settings they change to `~/.config/ship/config.toml` (or `$XDG_CONFIG_HOME/ship/config.toml`). Copying all of `ship config defaults` pins every default and stops it following Ship's updates.
2. Run `ship config check`. It prints nothing and exits 0 when the file is valid, and otherwise points at the first mistake with its line and column.
3. Open windows apply the file when it is saved. Server settings apply to new panes only.

For tmux habits, the prefix-mode example on the Configuration page is the starting point. Done when `ship config check` exits 0.

## 7. Optional extras

- [Shell completions](shell-completions.md): one line in the shell's startup file.
- [Scripting Ship](scripting/README.md): if the person wants their agents to open panes, send prompts to other agents and read the answers, the scripting pages show the commands. Every command prints JSON.

## When something goes wrong

[Troubleshooting](troubleshooting.md) has the log locations and how to raise the log level.
