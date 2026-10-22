# Configuration

Ship reads `~/.config/ship/config.toml`, or `ship/config.toml` under `$XDG_CONFIG_HOME`. Without one, it uses its defaults. `ship config defaults` prints them all, with what each setting accepts:

```console
$ ship config defaults | grep -A3 '\[client.agents\]'
```

> [!NOTE]
> Your file only needs what you change. Copy just those lines rather than the whole output: anything you copy stays fixed, and stops following Ship's defaults when they change. `--config <file>` or `SHIP_CONFIG` points somewhere else.

> [!NOTE]
> Open windows pick up changes when you save. If the file has a mistake, Ship keeps the last working settings and shows the error. `ship config check` points at the first mistake with its line and column.

## Server settings

```toml
[server]
shell = "/bin/bash"           # default: your login shell
scrollback_bytes = 10000000   # history per pane, about 10 MB by default
```

> [!NOTE]
> These apply to new panes. Panes that are already open keep what they started with.

## Client settings

```toml
[client]
sidebar_width = 40   # default: 26 columns
mouse = true         # false leaves the mouse to your terminal
```

## Agent settings

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

The built-in sounds are from Kenney's [Interface Sounds](https://www.kenney.nl) pack ([attribution](../../crates/ship-client/assets/sounds/ATTRIBUTION.md)).

## Keys and modes

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

The default keys are in [`defaults.toml`](../../crates/ship-core/src/defaults.toml). Key names like `ctrl-alt-x` and `shift-tab` follow [crokey](https://docs.rs/crokey).

## Menus

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
