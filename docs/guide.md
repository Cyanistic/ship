# Ship guide

This picks up where the [README](../README.md) leaves off. The examples assume `ship` is on your `PATH` and [`jq`](https://jqlang.org) is installed. `ship --help`, or `--help` on any command, lists every option.

- [Windows and the server](#windows-and-the-server)
- [Tabs](#tabs)
- [Panes](#panes)
- [Scrolling, copying and the mouse](#scrolling-copying-and-the-mouse)
- [Agents](#agents)
- [Scripting Ship](#scripting-ship)
- [Saved tabs](#saved-tabs)
- [Configuration](#configuration)
- [Troubleshooting](#troubleshooting)
- [Upgrading](#upgrading)

## Windows and the server

Ship runs in two parts. A background server owns your tabs, panes and the programs in them. Running `ship` opens a window onto that server. Closing the window, or pressing `alt-q`, leaves everything running, and running `ship` again brings you back.

The first `ship` starts the server for you. It restores your [saved tabs](#saved-tabs), or opens one tab with a shell if there's nothing to restore. `ship server status` tells you whether a server is running, and `ship server stop` shuts it down, ending every program inside.

You can open Ship from several terminals at once. The windows share the work: the same tabs, the same programs, the same output. Each window picks its own tab and pane, folds its own sidebar and keeps its own mode. Panes are sized to fit the smallest window looking at that tab, so a bigger window shows some empty space around them.

If a window loses its connection to the server, it reconnects on its own.

### Using Ship on another machine

The server only accepts connections from its own machine. To use it from somewhere else, forward its port over SSH:

```sh
ssh -N -L 43179:127.0.0.1:43179 otherbox
```

Then, in another terminal:

```sh
ship --server-url http://127.0.0.1:43179
```

Run `ship` on the other machine once first so its server is up. Passing `--server-url` makes Ship connect only to that address, so it won't quietly start a local server if the tunnel isn't up. All the CLI commands take `--server-url` too, or set `SHIP_SERVER_URL` once.

Sounds and clipboard copies happen on the machine you're sitting at. Through a tunnel, `ship server stop` stops the server but may report that it's still running, because the tunnel keeps accepting connections after the server is gone.

## Tabs

Tabs can hold panes, other tabs, or both, so a big task can keep its smaller pieces underneath it. The sidebar on the left shows the tree. `▾` marks an open tab and `▸` a folded one. A tab without a name is labeled with its first pane's directory.

| Key | Does |
| --- | --- |
| `alt-n` | New tab with a shell |
| `alt-shift-n` | New tab nested under the current one |
| `alt-left` / `alt-right` | Previous / next tab |
| `alt-g` | Tabs mode: `j` / `k` move, `h` / `l` fold / unfold, `esc` leaves |
| `alt-b` | Hide or show the sidebar |
| `alt-x` | Close the tab and everything in it |

Coming back to a tab puts you on the pane you last used there. You can also click a tab to select it, click its `▾`/`▸` to fold it, or drag it to reorder or nest it.

From the CLI, every command prints JSON, so `jq` can pull out IDs for the next step:

```sh
PROJECT=$(ship tab create --name project | jq -r .id)
REVIEW=$(ship tab create --name review "$PROJECT" | jq -r .id)   # nested under project
ship pane create --tab "$REVIEW" --name checks
ship tab list
ship tab set --tab "$REVIEW" --name code-review
ship tab close --tab "$PROJECT"   # closes review and its pane too
```

`ship tab move` reorders or reparents a tab. Setting a name to `""` clears it and brings back the automatic label.

## Panes

Every pane runs a program in its own terminal, your shell unless you say otherwise. When a program exits, its pane stays open with its output so you can read what happened. Close it with `alt-shift-x`.

| Key | Does |
| --- | --- |
| `alt-\|` / `alt--` | Split right / down |
| `alt-h` / `alt-j` / `alt-k` / `alt-l` | Move to the pane left, below, above or right |
| `alt-tab` | Next pane in the tab |
| `alt-shift-h` / `j` / `k` / `l` | Swap with the neighbor in that direction |
| `alt-f` | Zoom the pane to fill the tab, or unzoom |
| `alt-r` | Resize mode: `h` / `j` / `k` / `l` move the pane's edge, `esc` leaves |
| `alt-shift-x` | Close the pane |

A new split opens in the directory you were working in, so after `cd ~/src/ship`, `alt--` gives you a shell there. You can also resize by dragging the border between two panes.

Zoom belongs to the tab, so everyone looking at it sees the zoomed pane, and its sidebar row shows `Z`. Moving, splitting or resizing unzooms first.

The CLI can do the same things. Panes created from the CLI start in your current directory, or in `--cwd`:

```sh
TAB=$(ship tab create --name work | jq -r .id)
LEFT=$(ship pane create --tab "$TAB" | jq -r .id)
RIGHT=$(ship pane create --pane "$LEFT" --direction right -- htop | jq -r .id)
ship pane zoom --pane "$LEFT"                    # zoom, run again to unzoom
ship pane swap --pane "$LEFT" --other "$RIGHT"
ship pane resize --pane "$LEFT" --direction right --amount 5
ship tab get --tab "$TAB" | jq .layout
ship tab close --tab "$TAB"
```

Inside a Ship pane, you can leave out `--pane`; commands act on the pane they run in.

## Scrolling, copying and the mouse

### Scrolling

`alt-pageup` and `alt-pagedown` scroll a page at a time, and the mouse wheel scrolls whatever pane it's over. An `[offset/history]` marker shows how far up you are. New output doesn't pull you back down, but typing does. Each pane keeps about 10 MB of history.

Full-screen programs like Neovim and `less` have no scrollback in Ship; they manage their own screen.

Ship doesn't have built-in search or a copy mode yet. Until it does, read a pane's history and search it with your usual tools:

```sh
ship pane read --pane "$PANE" --lines 2000 | grep error
```

### Selecting and copying

Drag to select text and release to copy. Double-click selects a word and triple-click a line. Dragging past the edge of a pane scrolls it.

Copying uses your terminal's clipboard support (OSC 52). Most modern terminals have it; macOS Terminal.app doesn't. When a program copies on its own, like Neovim's `"+y`, that reaches your clipboard too.

Programs that use the mouse themselves, like Neovim with `mouse=a`, get clicks and drags instead. To use your terminal's own selection, hold its override key (usually Shift), or turn Ship's mouse handling off with `mouse = false` in the [config](#client-settings).

### Clicking and dragging

- Click a pane to select it, or a tab or agent in the sidebar to jump there.
- Drag the border between panes to resize them.
- Drag the sidebar's right edge to make it wider or narrower. Drag the `agents` line to change how much room tabs and agents get.
- Drag a tab in the sidebar to move it. Move left or right while dragging to choose how deeply it nests; an underline shows where it will land.

### Right-click menus

Right-click a pane, a tab or the empty sidebar space for a menu of actions, or click `≡` beside `tabs`. Pick with the mouse, or with the arrow keys and Enter. `esc` closes it.

| Menu | Items |
| --- | --- |
| Pane | Split right, split down, zoom, right-click passthrough, close |
| Tab | New tab, new child tab, close |
| Global | New tab, hide sidebar, reload config, detach |

Some programs want right-clicks for themselves. Turn on **right-click passthrough** for that pane, from its menu or with `ship pane set --right-click-passthrough true`, and right-clicks inside go to the program whenever it's using the mouse. Its label and border still open Ship's menu. You can change what's in each menu in the [config](#menus).

## Agents

Agents report their own state using [OSC 7501, the Program Status Protocol](https://www.superlogical.com/rex/docs/build/program-status). Claude Code, Codex, Pi and crush already do, so they work with no setup. Ship has no list of agents to update; anything that speaks the protocol shows up.

Each reporting agent gets an entry in the lower half of the sidebar:

```text
▲ claude blocked
ship › api
```

| Icon | State | Meaning |
| --- | --- | --- |
| `▲` red | blocked | Waiting on you, like a permission prompt |
| `✖` magenta | error | Something went wrong |
| `◆` cyan | unseen | Finished while you weren't looking |
| `●` yellow | working | Busy |
| `○` green | idle | Done and seen, or waiting for a prompt |

The ones that need you sort to the top. Tabs show the most urgent icon from anything inside them, so you can spot a blocked agent even in a folded tab.

Press `alt-a` to jump to the next agent that needs you. Click an entry to go to its pane.

**Unseen** is tracked per window. An agent that finishes while its pane isn't on screen in a focused window stays `◆` until you look at it. A window you attach later starts with nothing marked unseen.

### Sounds and notifications

Ship plays a sound when an agent finishes, or when one gets blocked or hits an error, as long as you're not already looking at it. It waits a second first, so an agent that pauses briefly and carries on stays quiet. With no working audio device, Ship carries on without sound.

Set `popups = true` to also get desktop notifications through your terminal, like `claude needs you · ship › api: Apply these changes?`. Ghostty and iTerm2 show them. The [agent settings](#agent-settings) cover sounds, popups and what each entry shows.

### When an agent's status looks stuck

An agent that crashes can't report that it stopped, so its last state hangs around. Ship can clear it when your shell shows its next prompt, but only if the shell marks its prompts. Bash and zsh don't by default. For zsh, add this to `~/.zshrc`:

```sh
_prompt_mark() { print -n '\e]133;A\e\\' }
precmd_functions+=(_prompt_mark)
```

Agents running inside tmux inside Ship can't report anything, because tmux doesn't pass the protocol through.

## Scripting Ship

Anything you can do with keys, you can also do with `ship` commands. They print JSON and take explicit IDs, so a script never acts on whichever pane you happen to have selected. Inside a Ship pane, `--pane` defaults to that pane. In these examples, `$PANE` holds a pane ID, like one printed by `ship pane create` or listed by `ship tab list`.

### Reading output

```sh
ship pane read --pane "$PANE"                 # what's on screen now
ship pane read --pane "$PANE" --lines 200     # the last 200 lines, including history
ship pane read --pane "$PANE" --ansi | cat    # keep the colors
```

Reading doesn't move anyone's scroll position. Panes whose program has exited can still be read.

### Typing into a pane

`ship pane send` types text and presses keys, as if you typed them:

```sh
ship pane send --pane "$PANE" "cargo test" --key enter
ship pane send --pane "$PANE" --key ctrl-c
git diff | ship pane send --pane "$PANE" -        # text from stdin
```

Text goes in first, then each `--key` in order. Key names are the same as in the [config](#keys-and-modes).

### Handing work to another agent

This is the loop for one agent starting another, giving it a task and reading the answer:

```sh
TAB=$(ship tab create --name workers | jq -r .id)
WORKER=$(ship pane create --tab "$TAB" --name worker -- claude | jq -r .id)
ship pane wait --pane "$WORKER"                                     # until it's ready
ship pane send --pane "$WORKER" "Write a haiku about ships" --key enter --wait \
  | jq -r '.program[""].state'
ship pane read --pane "$WORKER" --lines 40
```

`--wait` returns once the agent has stopped working and prints the pane. The state tells you how the turn ended: `done` or `idle` means it finished, `blocked` means it's asking you something.

To answer a prompt, send the keys and say you expect it to be blocked:

```sh
ship pane send --pane "$WORKER" --key down --key enter --wait --if blocked
```

Other options:

- `--if idle,done` only sends if the agent is in one of those states, so you don't type into an agent that's still busy.
- `pane wait --status done,exited` waits for specific states instead of "anything but working".
- Waits have no time limit. Wrap them in `timeout 600 ...` if you want one (`gtimeout` on macOS, from coreutils).
- If a `send --wait` fails or times out, the text may already have gone in. Read the pane before sending it again.

This works with any program that reports its status. For one that doesn't, use plain `send` and `read`.

### Steering a window

`ship view list` shows every attached window, and `ship view select` moves one of them to a tab or pane. Adding `--select` to a create command shows the new tab or pane in a window right away:

```sh
WINDOW=$(ship view list | jq -r 'keys_unsorted[0]')
ship tab create --name demo --starter --select --attachment "$WINDOW"
```

Without `--select`, creating something never moves anyone's view.

## Saved tabs

After a server restart or a reboot, Ship brings back your tabs, their nesting and names, the splits and their sizes, zoom, and each pane's last directory. Every pane starts a fresh shell. Running programs, agent conversations and scrollback don't come back.

Ship saves a few seconds after each change, and `ship server stop` saves right away. The file lives at `~/.local/state/ship/tabs/43179.json` (under `$XDG_STATE_HOME` if you set it; the number is the server's port).

If a pane's directory is gone, it opens in your home directory instead. If the file can't be read, Ship starts with no tabs and leaves the file alone so you can look at it.

### Getting back the previous tabs

Each time the server starts and saves, it keeps the file it replaced as `43179.json.prev`. To go back to it:

```sh
ship server stop
SAVED="${XDG_STATE_HOME:-$HOME/.local/state}/ship/tabs/43179.json"
cp "$SAVED.prev" "$SAVED"
ship
```

Copy the `.prev` file somewhere safe if you want to keep it, since the next run replaces it.

### Starting fresh

`ship server --no-restore` starts a server in the foreground without loading the saved tabs. Add `--starter` to get one shell tab. Your first change then replaces the saved tabs, keeping the old ones as `.prev`.

To try things without touching your usual tabs, run a second server on another port:

```sh
ship server --port 43326 --no-restore --starter
ship --server-url http://127.0.0.1:43326        # in another terminal
```

## Configuration

Ship reads `~/.config/ship/config.toml`, or `ship/config.toml` under `$XDG_CONFIG_HOME`. Without one, it uses its defaults. `--config <file>` or `SHIP_CONFIG` points somewhere else.

Open windows pick up changes when you save. If the file has a mistake, Ship keeps the last working settings and shows the error. `ship config check` points at the first mistake with its line and column.

### Server settings

```toml
[server]
shell = "/bin/bash"           # default: your login shell
scrollback_bytes = 10000000   # history per pane, about 10 MB by default
```

These apply to new panes. Panes that are already open keep what they started with.

### Client settings

```toml
[client]
sidebar_width = 40   # default: 26 columns
mouse = true         # false leaves the mouse to your terminal
```

### Agent settings

```toml
[client.agents]
rows = [["icon", "app", "state"], ["tab"]]  # what each sidebar entry shows
delay_ms = 1000                             # how long a state must hold before notifying
popups = false                              # true also sends desktop notifications

[client.agents.sounds.default]
done = { path = "~/sounds/ding.mp3" }
attention = { builtin = "attention" }

[client.agents.sounds.apps.codex]
done = "off"
```

`rows` lists the lines of each entry. The pieces you can use are `icon`, `app`, `state`, `tab`, `pane` (the pane's label), and `title` and `msg` (whatever the agent reported).

A sound is `"off"`, a built-in (`{ builtin = "done" }` or `{ builtin = "attention" }`), or a file (`{ path = "..." }`) in MP3, WAV, Ogg Vorbis or FLAC. `done` plays when an agent finishes and `attention` when it's blocked or errors. `[client.agents.sounds.apps.<name>]` overrides sounds for one agent. If a sound file is missing or broken, Ship plays the built-in one instead and says so in the client log.

The built-in sounds are from Kenney's [Interface Sounds](https://www.kenney.nl) pack ([attribution](../crates/ship-client/assets/sounds/ATTRIBUTION.md)).

### Keys and modes

Keys live in modes, and you start in `normal`. Binding a key replaces only that key; everything else keeps its default. `{}` unbinds a key so it reaches the program instead:

```toml
[client.modes.normal.keys]
"alt-q"  = {}                                       # alt-q goes to the program
"alt-t"  = { action = { server.tab.create = {} } }  # new empty tab
"ctrl-b" = { action = { client.mode = "prefix" } }  # tmux habits

[client.modes.prefix]
kind = "oneshot"                                    # back to normal after one key

[client.modes.prefix.keys]
"c"      = { action = { server.tab.create = { starter = "shell", select = true } } }
"%"      = { action = { server.pane.create = { direction = "right", select = true } } }
"ctrl-b" = { action = { client.send = "ctrl-b" } }  # a real ctrl-b for the pane
```

There are two kinds of actions:

- `server.` actions are CLI commands. `server.tab.create = { starter = "shell" }` does the same as `ship tab create --starter shell`, so `ship <command> --help` documents its options.
- `client.` actions change only your window, like `client.pane.focus`, `client.sidebar.toggle`, `client.mode` and `client.detach`.

A `oneshot` mode takes one key and returns to normal. A `sticky` mode stays until a binding leaves it. `clear_defaults = true` starts a mode with none of Ship's bindings. Add `label = "..."` to a binding to change how it's shown in the hints.

When you're in a mode other than normal, a one-line hint bar shows at the bottom with that mode's keys. The same bar shows errors and when a pane's program has exited.

All the defaults are in [`defaults.toml`](../crates/ship-client/src/keymap/defaults.toml). Key names like `ctrl-alt-x` and `shift-tab` follow [crokey](https://docs.rs/crokey).

### Menus

Each right-click menu can be replaced with your own list of actions, written like key bindings:

```toml
[client.menus]
pane = [
  { action = { server.pane.zoom = {} }, label = "fill" },
  { action = { server.pane.close = {} } },
]
tab = []    # no tab menu
```

The menus are `pane`, `tab` and `global`. Any you leave out keep their defaults.

## Troubleshooting

### Logs

The server writes logs to `~/.local/state/ship/logs/43179/` (under `$XDG_STATE_HOME` if set). To follow along:

```sh
tail -F ~/.local/state/ship/logs/43179/current.log
```

Windows log to `~/.local/state/ship/logs/client/`. Problems that don't show up on screen end up in one of these: a saved directory that disappeared is in the server log, and a sound file that wouldn't load is in the client log. Old logs are cleaned up automatically.

Logs don't record what you type or what programs print, but error messages can include program names and paths, so look them over before sharing.

### More detail

Turn up the server's logging without restarting it, and put it back when you're done:

```sh
ship server log debug
ship server log --reset
```

To start a server or window with more logging, set `SHIP_LOG`:

```sh
SHIP_LOG=debug ship
```

If a program renders wrong in a pane, the server log includes Ghostty's warnings about terminal features it doesn't support, which helps tell a terminal gap from a Ship bug.

### Recording a timeline

For something that feels slow, record a timeline and open it in [Perfetto](https://ui.perfetto.dev):

```sh
SHIP_TRACE=/tmp/ship-trace ship server --port 43180
SHIP_TRACE=/tmp/ship-trace ship --server-url http://127.0.0.1:43180   # another terminal
```

Do the slow thing, detach with `alt-q`, then stop that server with `ship --server-url http://127.0.0.1:43180 server stop`. The timelines are in `/tmp/ship-trace/`. They aren't size-limited, so keep recordings short.

## Upgrading

Update the window and the server together. Before installing a new version, stop the running server with the old `ship` (`ship server stop`), since a newer `ship` can't talk to an older server. Stopping ends every program in it, so finish or save your work first.

If your config stops loading after an upgrade, check for these older spellings:

| Old | New |
| --- | --- |
| `"key" = [{ ... }]` | `"key" = { action = { ... } }` |
| `"key" = []` | `"key" = {}` |
| `hint = "..."` | `label = "..."` |
| `server.tab.rename`, `server.pane.rename` | `server.tab.set`, `server.pane.set` |
