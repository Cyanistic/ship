# Ship guide

How Ship behaves once you're past the [README](../README.md). Every example assumes `ship` is on your `PATH` and `jq` is installed. Run `ship --help`, or `--help` on any subcommand, for the full list of options.

- [The server and clients](#the-server-and-clients)
- [Reading and steering a client](#reading-and-steering-a-client)
- [Tabs](#tabs)
- [Panes and splits](#panes-and-splits)
- [Scrolling and reading output](#scrolling-and-reading-output)
- [Mouse and clipboard](#mouse-and-clipboard)
- [Configuration](#configuration)
- [Upgrading](#upgrading)

## The server and clients

Ship is split in two. The server owns your tabs, panes and the programs running in them. A client is the full-screen view you get from running `ship`. Closing a client doesn't touch the server, so your programs keep running.

Bare `ship`, and any `tab` or `pane` command, reuses the local server or starts one in the background. A server started this way begins with one tab holding a shell in your home directory. `ship server status` prints the server's health as JSON, and `ship server stop` stops it, ending every program inside. Neither one ever starts a server.

You can attach several clients at once, from different terminals. They share the work itself: tabs, panes, what each program prints, zoom, and each pane's scroll position. The server holds a separate selection for each attached client. Sidebar folds, whether the sidebar is shown, and which pane a client last used in each tab stay local to that client. If a client loses its connection, it reconnects on its own.

Keys, pastes, actions and clicks run in input order. On a slow link, typing right after `alt-n` waits for the new tab and reaches its shell. Navigation also waits for each server reply, so held navigation keys can keep moving after release. Mode changes, the sidebar toggle and config reloads apply immediately. A failed action shows its error without cancelling later input. Disconnecting drops waiting input rather than replaying it after reconnecting; `alt-q` lets earlier typing land before detaching. A server action can still overtake typing just before it, because they travel on separate connections.

Typing with a tab selected reaches its zoomed pane, otherwise the pane you last selected there, otherwise its first pane. The tab itself stays selected. Resizing the terminal does not overwrite a selection set by a script.

Panes are sized for the smallest client viewing the tab. A larger client draws the same layout in its top-left corner and dots the leftover space.

## Reading and steering a client

`ship view list` prints every attached client's view as JSON, keyed by attachment ID in attach order. Each record contains its pane or tab selection, when it has one, and its terminal area. Use an ID from that listing to read or change one client's selection:

```sh
ship view list
ATTACHMENT=$(ship view list | jq -r 'keys_unsorted[0]')
TAB=$(ship tab create --name steer | jq -r .id)
PANE=$(ship pane create --tab "$TAB" | jq -r .id)
ship view select --attachment "$ATTACHMENT" "$PANE"
ship view selected --attachment "$ATTACHMENT"  # prints the pane ID as a JSON string
ship view select --attachment "$ATTACHMENT" "$TAB"
ship view select --attachment "$ATTACHMENT" --none
ship view selected --attachment "$ATTACHMENT"  # prints null
ship tab close --tab "$TAB"
```

Keep a client attached while running this example. Its display follows the selection when the server publishes it. Selecting a pane hidden by zoom unzooms its tab for every client. Unknown attachments or nodes fail without changing anything.

`--attachment` defaults to `SHIP_ATTACHMENT_ID` when that variable is set. `view select` requires either a node ID or `--none`; omitting both is an argument error, not a request to clear the selection. A successful select prints the stored view record. Attachment IDs last only until that client detaches.

### Creating and selecting together

CLI creates leave every client's selection alone unless you pass `--select`. With it, the server creates and selects in one edit, so the chosen client never sees the new node before selecting it:

```sh
ATTACHMENT=$(ship view list | jq -r 'keys_unsorted[0]')
TAB=$(ship tab create --name selected --starter --select --attachment "$ATTACHMENT" | jq -r .id)
PANE=$(ship pane create --tab "$TAB" --select --attachment "$ATTACHMENT" -- sleep 9 | jq -r .id)
ship view selected --attachment "$ATTACHMENT"  # prints the new pane ID
ship tab close --tab "$TAB"
```

Keep a client attached for this example. A tab with `--starter` selects its shell pane; an empty tab selects the tab itself. `--attachment` again defaults to `SHIP_ATTACHMENT_ID`; Ship does not set that variable in ordinary pane shells. Without either source of an attachment, `--select` fails before creating anything. An unknown attachment also creates nothing and leaves no starter program running. Passing `--attachment` alone does not select.

## Tabs

Tabs form a tree. A tab can hold panes, child tabs, or both, and can be empty.

The sidebar on the left shows one row per visible tab, indented by depth. `▾` marks an expanded tab and `▸` a folded one. The highlighted row is the selected tab, or the tab holding your selected pane. A tab without a name is labeled by its first pane's current directory (`~` at home), or by that pane's label.

`alt-n` adds a top-level tab with a shell. `alt-shift-n` adds one as the last child of the current tab. `alt-left` and `alt-right` move between visible rows, and returning to a tab lands you on the pane you last used there. `alt-g` enters tabs mode, where `j` and `k` move, `h` folds, `l` unfolds and `esc` returns to normal mode. `alt-b` hides the sidebar and gives its width to the panes.

From the CLI, build a three-level tree:
```sh
PROJECT=$(ship tab create --name project | jq -r .id)
REVIEW=$(ship tab create --name review "$PROJECT" | jq -r .id)
CHECKS=$(ship tab create --name checks "$REVIEW" | jq -r .id)
ship pane create --tab "$CHECKS" --name check-shell
ship tab list
ship tab close --tab "$PROJECT"    # closes everything under it too
```

`ship tab create --child` adds a child of the tab holding the current pane, like `alt-shift-n`. `--starter` gives a new tab a shell in your home directory. `ship tab move` reparents or reorders a tab.

## Panes and splits

Every pane runs a program, your login shell by default, in its own terminal. Closing a pane ends its program; the other panes keep running. When a program exits, its pane stays open with its output, and the status line says how it exited.

`alt-|` splits the selected pane to the right and `alt--` splits it downward. The selected pane has a heavier, colored border and receives your typing. `alt-h`/`j`/`k`/`l` moves to the neighbor in that direction. When several panes border yours on that side, Ship picks the one you used most recently, then the nearest.

### Where new panes start

A pane split from the keyboard starts in the selected pane's current directory, so after `cd ~/src/ship`, `alt--` opens a shell there. Ship learns the directory from the shell (through OSC 7, which most zsh and fish setups send). On Linux and macOS, if the shell never reports it, Ship reads the program's working directory instead. That path has symlinks resolved, so `/tmp` shows as `/private/tmp` on macOS.

Panes created from the CLI start in the directory you run the command from, or in `--cwd`. New tabs' shells start in your home directory. `ship pane get` shows both where a pane started (`cwd`) and where it is now (`dir`):
```sh
TAB=$(ship tab create --name dirs | jq -r .id)
PANE=$(ship pane create --tab "$TAB" --cwd /usr/share | jq -r .id)
ship pane get --pane "$PANE" | jq '{cwd, dir}'
ship tab close --tab "$TAB"
```

### Splitting from the CLI

```sh
TAB=$(ship tab create --name work | jq -r .id)
LEFT=$(ship pane create --tab "$TAB" --name left | jq -r .id)
RIGHT=$(ship pane create --pane "$LEFT" --direction right --name right | jq -r .id)
BOTTOM=$(ship pane create --pane "$RIGHT" --direction down --name bottom | jq -r .id)
ship tab get --tab "$TAB" | jq .layout
ship pane close --pane "$LEFT"     # right and bottom grow to fill the tab
ship tab close --tab "$TAB"
```

`--pane` splits that pane, and `--direction` is `right` unless you say `down`. With only `--tab`, Ship splits the tab's largest pane, or creates the first pane in an empty tab. Inside a Ship pane, `--pane` defaults to the pane you're in. A split fails with `no space for new pane` if either half would have no room.

`ship tab get` reports the tab's `layout` as a tree: each node's `kind` is `pane` or `split`, and a split has an `axis`, the first child's `ratio`, and `first` and `second` children. An empty tab has no `layout`.

### Zoom, swap and resize

`alt-f` zooms the selected pane to fill the tab, and pressing it again unzooms. Zoom belongs to the tab, so every client viewing it sees the zoom, and the sidebar row is marked `Z`. Moving focus, swapping, creating, closing or resizing a pane unzooms first.

`alt-shift-h`/`j`/`k`/`l` swaps the selected pane with its neighbor in that direction. The programs and their screens move with them.

`alt-r` enters resize mode, where `h`/`j`/`k`/`l` move the selected pane's edge on that side by one cell. At the edge of the tab, it moves the opposite edge instead, so `l` on the rightmost pane narrows it. `esc` leaves resize mode.

The same edits from the CLI each print the tab:
```sh
TAB=$(ship tab create --name edits | jq -r .id)
LEFT=$(ship pane create --tab "$TAB" | jq -r .id)
RIGHT=$(ship pane create --pane "$LEFT" --direction right | jq -r .id)
ship pane zoom --pane "$LEFT"                    # toggles
ship pane swap --pane "$LEFT" --other "$RIGHT"   # any two panes in one tab
ship pane resize --pane "$LEFT" --direction left --amount 5
ship pane resize --pane "$LEFT" --direction right --to 40
ship tab close --tab "$TAB"
```

`--to` places the edge's border at a zero-based column for left/right or row for up/down, measured from the tab's origin, not the terminal's. It uses the same opposite-edge fallback as `--amount` and stops before any pane's content drops below one column or row. An edge already there changes nothing. `--to` and `--amount` cannot be combined. Both are available in `server.pane.resize` bindings too. Keep a client viewing the tab to inspect its `geometry` with `ship tab get`; an unviewed tab uses an 80×24 fallback for resize.

## Scrolling and reading output

Each pane keeps about 10 MB of history. `alt-pageup` and `alt-pagedown` scroll the selected pane a page at a time, and the mouse wheel scrolls whichever pane it's over. Every client sees the same scroll position. New output doesn't pull you back down, but typing or pasting does. While scrolled, an `[offset/history]` indicator shows how many rows up you are.

Programs that use the alternate screen, like Neovim and `less`, have no scrollback in Ship. Copy mode and built-in search aren't built yet; piping `ship pane read` into `grep` or an editor covers search for now.

`ship pane read` prints a pane's text without moving its view:
```sh
TAB=$(ship tab create --name history | jq -r .id)
PANE=$(ship pane create --tab "$TAB" -- sh -c 'seq 200' | jq -r .id)
ship pane read --pane "$PANE"                       # the visible screen
ship pane read --pane "$PANE" --lines 200 | grep '^150$'
ship pane read --pane "$PANE" --ansi | cat          # keep colors
ship pane scroll --pane "$PANE" --direction top
ship pane scroll --pane "$PANE" --direction down --amount 5
ship tab close --tab "$TAB"
```

`--lines N` reads the last N rows including history, and `--unwrap` joins lines the terminal wrapped. `ship pane scroll` takes `up`, `down`, `top` or `bottom`; `up` and `down` move one page unless you give `--amount`. Exited panes can still be scrolled and read.

## Mouse and clipboard

Click a pane to select it; that click isn't passed to the program. Click a sidebar row to select its tab, or its `▾`/`▸` marker to fold or unfold it. Clicks wait behind earlier input but keep the target you clicked; if it disappears before the click runs, nothing happens. While input is waiting, a press on even the selected pane selects it instead of reaching its program.

Programs that ask for the mouse, like Neovim with `mouse=a`, get clicks, drags and the wheel in the selected pane. Otherwise, drag to select text, double-click for a word or triple-click for a line, and release to copy. Dragging past the edge scrolls. Everyone viewing the pane sees the highlight, but only the client that made it gets the copy.

Copies go through your terminal's clipboard escape (OSC 52). Most modern terminals support it; macOS Terminal.app doesn't, and some terminals cap how much you can copy. When a program copies to the clipboard itself, such as Neovim's `"+y`, every client viewing that tab receives it. Programs can't read your clipboard.

To use your terminal's own selection instead, hold its mouse override key (usually Shift), or set `mouse = false` under `[client]`.

## Configuration

Ship reads `ship/config.toml` from your config directory: `~/.config/ship/config.toml`, or under `$XDG_CONFIG_HOME` if you set it. `--config <file>` or `SHIP_CONFIG` points elsewhere. Without a file, Ship uses its defaults.

### Server settings

```toml
[server]
shell = "/bin/bash"           # default: your login shell
scrollback_bytes = 10000000   # default: about 10 MB per pane
```

These apply to panes opened afterwards; existing panes keep what they started with. The history cap is approximate. A relative `shell` path is relative to the config file's folder.

### Client settings

```toml
[client]
sidebar_width = 40   # default: 26 columns
mouse = true         # false leaves selection to your terminal
```

A running client reloads its settings and keys when you save the file. If the file has a mistake, the client keeps its current settings and shows the error. `ship config check` reports the first mistake with its line and column, and exits non-zero.

### Keys and modes

Keys belong to modes. You start in `normal`. Each binding names one `action`, and a key you bind replaces only that key's default. Use `{}` to unbind:
```toml
[client.modes.normal.keys]
"alt-t"  = { action = { server.tab.create = {} } } # new empty tab
"alt-q"  = {}                                     # alt-q reaches the program
"alt-y"  = { action = { server.pane.resize = { direction = "right", amount = 5 } } }
"ctrl-b" = { action = { client.mode = "prefix" } } # tmux habits

[client.modes.prefix]
kind = "oneshot"                                  # back to normal after one key

[client.modes.prefix.keys]
"c"      = { action = { server.tab.create = { starter = "shell", select = true } } }
"ctrl-b" = { action = { client.send = "ctrl-b" } }  # a literal ctrl-b for the pane

[client.modes.resize.keys]
"f" = { action = { server.pane.zoom = {} }, hint = "fill" }
```

Migrate old bindings from `[{ … }]` to `{ action = { … } }`, and from `[]` to `{}`. Bind separate steps to separate keys, rather than an action list. A bare action without the `action` field, or an unknown binding field such as `hnt`, is a config error.

A `oneshot` mode takes one key and returns to normal. A `sticky` mode stays until a binding leaves it, and swallows keys it doesn't bind. `clear_defaults = true` starts a mode with none of Ship's bindings. Bind `client.config.reload = {}` to reload on demand.

The four default create keys select what they make. In your own binding, set `select = true` to do the same for the pressing client; without it, creation leaves the selection alone. Bindings cannot name an `attachment`. For example, `"alt-t"` above creates an empty tab without moving you.

`server.` actions are the CLI's commands with the same names and options: `server.tab.create.starter = "shell"` is `ship tab create --starter shell`. `client.` actions act on your own view, like `client.pane.focus`, `client.sidebar.toggle` and `client.detach`; Ship's own defaults are in [`defaults.toml`](../crates/ship-client/src/keymap/defaults.toml). Key spellings like `ctrl-alt-x` and `shift-tab` come from [crokey](https://docs.rs/crokey).

### The status line

There's no permanent status bar. A one-row overlay appears at the bottom of the pane area only when there's something to say: you're outside normal mode, the client is reconnecting, the selected pane's program has exited, or an action failed. It shows the mode, any status, and hints for the mode's keys. A binding's optional `hint` overrides the word derived from its action: the mode name, a direction, the chord sent, or the action name. For example, the `f` binding above shows `f fill` instead of `f zoom`. Unbound chords don't appear in the hints.

## Upgrading

Update the client and server together. `ship` checks the server's protocol version and refuses to talk to one that doesn't match, so after rebuilding, run `ship server stop` and start again. Stopping the server ends every program in it, so finish or save running work first.
