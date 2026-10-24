# Configuration

<!--
Scope: The config file: where it lives, then server, client and agent settings, keys, modes and menus, each with a short example.
Not here: what each default key does (keys.md), every setting and value (`ship config defaults` prints them).
-->

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
[server.defaults.pane]
shell = "/bin/sh"                 # default: your login shell
scrollback_bytes = 10_000_000       # bytes of history per pane
right_click_passthrough = false
exit_behavior = "close"            # "close", "keep" or "keep_failed"
```

These are defaults for new panes, including a new tab's starter pane. Save the file and the next pane takes them; panes already open keep their values. A relative `shell` path resolves from the config file's folder. If the file can't be read or has errors, new panes use the built-in defaults and the server logs the error.

`ship pane create --exit-behavior keep --scrollback-bytes 50000000` overrides the defaults for that pane. [Panes](panes.md#when-a-program-exits) explains the exit behaviors. `--right-click-passthrough` turns passthrough on, and `--right-click-passthrough=false` turns it off even when the default is on.

You can change an existing pane's history limit without restarting its program:

```sh
ship pane set --pane "$PANE" --scrollback-bytes 50000000
```

Raising the limit keeps the history already there and lets it grow. Lowering it drops the oldest history beyond the new limit right away. The limit is in bytes, not lines.

> [!NOTE]
> If your file still has `shell` or `scrollback_bytes` under `[server]`, move them to `[server.defaults.pane]`. The old settings are no longer accepted; `ship config check` points at them with their line and column.

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
