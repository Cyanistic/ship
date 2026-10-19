# Ship guide

How Ship behaves once you're past the [README](../README.md). Every example assumes `ship` is on your `PATH` and `jq` is installed. Run `ship --help`, or `--help` on any subcommand, for the full list of options.

- [The server and clients](#the-server-and-clients)
- [Saved tabs](#saved-tabs)
- [Reading and steering a client](#reading-and-steering-a-client)
- [Tabs](#tabs)
- [Panes and splits](#panes-and-splits)
- [Scrolling and reading output](#scrolling-and-reading-output)
- [Mouse and clipboard](#mouse-and-clipboard)
- [Configuration](#configuration)
- [Logs and levels](#logs-and-levels)
- [Upgrading](#upgrading)

## The server and clients

Ship is split in two. The server owns your tabs, panes and the programs running in them. A client is the full-screen view you get from running `ship`. Closing a client doesn't touch the server, so your programs keep running.

Bare `ship`, and any `tab` or `pane` command, reuses the local server or starts one in the background. A new server restores its saved tabs; if there are none, it creates one tab holding a shell in your home directory. When bare `ship` launches the server, its client opens on the first tab's first pane. Attaching to an already-running server starts with nothing selected. `ship server status` prints the server's health as JSON, and `ship server stop` stops it, ending every program inside. Neither one ever starts a server.

You can attach several clients at once, from different terminals. They share the work itself: tabs, panes, what each program prints, zoom, and each pane's scroll position. The server holds a separate selection for each attached client. Sidebar folds, whether the sidebar is shown, and which pane a client last used in each tab stay local to that client. If a client loses its connection, it reconnects on its own.

Keys, pastes, actions and clicks run in input order. On a slow link, typing right after `alt-n` waits for the new tab and reaches its shell. Navigation also waits for each server reply, so held navigation keys can keep moving after release. Mode changes, the sidebar toggle and config reloads apply immediately. A failed action shows its error without cancelling later input. Disconnecting drops waiting input rather than replaying it after reconnecting; `alt-q` lets earlier typing land before detaching. A server action can still overtake typing just before it, because they travel on separate connections.

Typing with a tab selected reaches its zoomed pane, otherwise the pane you last selected there, otherwise its first pane. The tab itself stays selected. Resizing the terminal does not overwrite a selection set by a script.

Panes are sized for the smallest client viewing the tab. A larger client draws the same layout in its top-left corner and dots the leftover space.

## Saved tabs

After a server restart or reboot, Ship rebuilds the saved tabs with fresh shells in each pane's last directory. Tab order and nesting, tab and pane IDs and names, splits and their proportions, zoom, and right-click passthrough come back. Programs and their arguments, titles, exit status, screens and scrollback do not. A pane that was running an editor or agent returns to a shell, not that program or its conversation. Client-local folds, modes and sidebar settings are not restored.

Ship starts a save five seconds after the first unsaved change. Later changes, including repeated directory changes, do not push that timer back. Writing takes additional time; a crash can lose changes that have not reached disk yet. `ship server stop`, SIGTERM and SIGINT flush pending changes without waiting for that timer. Live-only changes, such as a title or exit status, do not trigger saves, and restoring and stopping without a saved change leaves the file alone.

Each port has its own file under `$XDG_STATE_HOME/ship/tabs/`, defaulting to `~/.local/state/ship/tabs/` on Linux and macOS. For the default server it is `43179.json`. The file is readable only by its owner (0600), in an owner-only directory (0700). Saves replace it atomically. On a run's first save, Ship also keeps the old file as `<port>.json.prev`; later saves in that run leave the copy alone. A failed save is logged and retried five seconds later.

### Starting fresh

`ship server --no-restore` skips loading the saved file. Without `--starter`, it starts empty. Adding `--starter` creates a shell tab only when nothing was restored, and that new tab counts as an unsaved change. The first save replaces the saved tabs and keeps the earlier file as `.prev`. Starting empty and making no changes leaves the saved file untouched. Automatically launched background servers always restore.

For example, use a separate port to avoid your default server. Run this in one terminal, then use a second terminal for commands against it:

```sh
ship server --port 43326 --starter
```

```sh
ship --server-url http://127.0.0.1:43326 tab list
ship --server-url http://127.0.0.1:43326 server stop
```

To start that port empty instead, run:

```sh
ship server --port 43326 --no-restore
```

### Recovering the previous file

Ship never restores `.prev` automatically. Stop the server before copying it back, then restart without `--no-restore`. For the example port above:

```sh
ship --server-url http://127.0.0.1:43326 server stop
SAVED="${XDG_STATE_HOME:-$HOME/.local/state}/ship/tabs/43326.json"
cp "$SAVED.prev" "$SAVED"
ship server --port 43326
```

Replace the port in both the commands and the file path for another server. The next run's first save replaces `.prev` again, so copy it elsewhere first if you need to keep it longer.

### Restore problems

A missing directory falls back to your home directory. A shell that cannot start leaves its pane out. An unreadable file or invalid JSON starts with no restored tabs, without deleting the file; the first later save keeps its bytes in `.prev` if they can be read. A file that parses is trusted, so take care when editing it by hand.

Warnings name the affected pane or file in the [server log](#logs-and-levels), including the `.prev` path for a bad file. They do not appear in the client or `ship server status`. For the default port, read `~/.local/state/ship/logs/43179/current.log`, or the equivalent under `$XDG_STATE_HOME`.

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

### Setting names and pane attributes

`ship tab set` and `ship pane set` replace the old `rename` commands. An omitted attribute stays as it is; a blank name clears it and restores the derived label. Empty or unchanged edits succeed without changing the shared revision.

```sh
TAB=$(ship tab create --name notes | jq -r .id)
PANE=$(ship pane create --tab "$TAB" | jq -r .id)
ship pane set --pane "$PANE" --name build --right-click-passthrough true
ship pane set --pane "$PANE" --right-click-passthrough toggle
ship pane set --pane "$PANE" --name ""
ship tab set --tab "$TAB" --name ""
ship pane set --pane "$PANE"                     # leaves everything alone
ship pane create --tab "$TAB" --right-click-passthrough -- sh
ship tab close --tab "$TAB"
```

`right_click_passthrough` is a shared pane attribute accepting `true`, `false` or `toggle`. It defaults to off, including on a tab's starter pane, and survives a server restart with the saved tabs. When it is on and the program has asked for mouse input, right-clicks on the pane's contents reach the program without opening a menu or changing selection. The following drag and release stay with that program, even outside the pane. Labels and borders still open the menu, as do contents when the program isn't tracking the mouse, such as a shell prompt. The pane menu can toggle the setting for every client. `--right-click-passthrough` on pane create starts it on.

Bindings use the same fields under `server.tab.set` and `server.pane.set`:
```toml
[client.modes.normal.keys]
"alt-p" = { action = { server.pane.set = { right_click_passthrough = "toggle" } } }
```

Old `server.tab.rename` and `server.pane.rename` bindings are configuration errors. Migrate them to `set`; unlike rename, omitting `name` no longer clears it.

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

`alt-r` enters resize mode, where `h`/`j`/`k`/`l` move the selected pane's edge on that side by one cell. At the edge of the tab, it moves the opposite edge instead, so `l` on the rightmost pane narrows it. `shift-h` and `shift-l` narrow or widen this client's sidebar by one column. They do nothing while the sidebar is hidden. `esc` leaves resize mode.

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

Click a pane's contents to select it; that click isn't passed to the program. Click a sidebar row to select its tab, or its `▾`/`▸` marker to fold or unfold it. Sidebar clicks act on release, only if the pointer hasn't moved a cell during the press. Clicks wait behind earlier input but keep the target you clicked; if it disappears before the click runs, nothing happens. While input is waiting, a press on even the selected pane's contents selects it instead of reaching its program.

Drag a shared border between panes with the left button to resize them; a junction grabs both axes. The held border uses the selected-border color, but doesn't select a pane or send the press to a program. A press without motion does nothing. Borders stop before any pane's content becomes smaller than one column or row; lone and zoomed panes have no border to grab. If the grabbed pane closes, the drag ends quietly. Positions take their turn behind earlier input. On a slow link, the border can trail the pointer, then settle at the latest position rather than replaying every motion.

Drag the sidebar's right divider to resize it for this client. Its width stays between 10 columns and the width that leaves 20 columns for panes; on narrower terminals, the pane minimum wins. Hiding and showing the sidebar keeps the width. Neither dragging nor the resize-mode keys writes to your config, and an unrelated config reload keeps your chosen width. Changing `sidebar_width` replaces it.

Drag a sidebar row to move its tab, carrying its children and running programs with it. The pointer's row picks the gap above that row, or the gap after the last row when below it. Move left or right in two-column indent steps to choose the nesting depth. The depth is limited by the rows on either side of the gap; an underline on the row above shows where the tab will land. Above the first tab, the underline is on the blank row under `tabs`.

Only visible rows can be drop targets; the sidebar does not scroll or auto-scroll during a drag. Drags use the left button, and Esc does not cancel them.

Release to move the tab. Dropping as a child of a folded tab unfolds that parent in this client. Dragging doesn't select the tab, and a moved selected tab stays selected. A drop into the tab's own subtree or back where it already is shows no line and does nothing. Releasing outside the sidebar cancels the drop. A queued drop is checked again at its turn; if its source or target has disappeared or the move is no longer valid, nothing happens and no error is shown.

Programs that ask for the mouse, like Neovim with `mouse=a`, get left clicks, drags and the wheel in the selected pane. Otherwise, drag to select text, double-click for a word or triple-click for a line, and release to copy. Dragging past the edge scrolls. Everyone viewing the pane sees the highlight, but only the client that made it gets the copy.

Copies go through your terminal's clipboard escape (OSC 52). Most modern terminals support it; macOS Terminal.app doesn't, and some terminals cap how much you can copy. When a program copies to the clipboard itself, such as Neovim's `"+y`, every client viewing that tab receives it. Programs can't read your clipboard.

To use your terminal's own selection instead, hold its mouse override key (usually Shift), or set `mouse = false` under `[client]`.

### Right-click menus

Right-click a pane's contents, label or border for its menu, or a sidebar row for the tab menu. Contents clicks instead reach the program when the pane's right-click passthrough is on and the program is tracking the mouse. Opening a menu selects the clicked pane or tab, in input order, before any picked action. On shared borders the menu belongs to the pane below or to the right, and to the bottom-right pane at a junction. Labels and outer edges belong to their own pane.

Click `≡` at the end of the `tabs` heading, or right-click the heading or the space below the last tab, for the global menu. It does not change selection. Menus belong only to the client that opens them. They use your terminal's default foreground and background, with ANSI cyan borders and a bold black-on-cyan highlight. These colors follow your terminal palette, not a separate Ship theme.

Hover over an item to highlight it, then click or press Enter to run it. Up/Down and `k`/`j` also move the highlight. Moving over the border or outside the menu keeps the last highlight. Esc closes without acting. A press outside closes the menu without selecting or sending input there; resizing the terminal also closes it. While open, the menu consumes keys, bindings and paste instead of passing them to a pane.

The defaults are:
- Pane: split right, split down, zoom, right-click passthrough, close.
- Tab: new tab, new child tab, close. A new tab is created just after the clicked tab; a new child is appended under it. Both start a selected shell.
- Global: new tab, hide sidebar, reload config, detach.

Menu actions work like key actions, including the status-line error on failure. Mode changes, sidebar actions and reloads apply immediately; other actions wait behind earlier input. The passthrough toggle changes the shared pane setting, so every attached client follows it.

Customize each target under `[client.menus]`:

```toml
[client.menus]
pane = [
  { action = { server.pane.zoom = {} }, label = "fill" },
  { action = { server.pane.close = {} } },
]
tab = []
```

A named target replaces its whole default list, in the order given; omitted targets keep their defaults. An empty list disables that menu, leaving the old right-click behavior in place. Each item requires exactly one action, written as in a key binding. Without `label`, it uses the action's default label. Unknown targets, item fields or actions fail `ship config check` with a line and column. Valid saves reload menus with the rest of the client settings and close any open menu; invalid saves keep the last good settings. With `[client] mouse = false`, no menu opens.

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

`sidebar_width` is the starting width, subject to the limits described under [Mouse and clipboard](#mouse-and-clipboard). A drag or `client.sidebar.resize` overrides it until the client exits or the configured value changes. For a five-column key action:
```toml
[client.modes.normal.keys]
"alt-[" = { action = { client.sidebar.resize = { direction = "left", amount = 5 } } }
```

Only `left` and `right` are accepted; omitting `amount` moves one column. This action has no CLI command.

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
"f" = { action = { server.pane.zoom = {} }, label = "fill" }
```

Migrate old bindings from `[{ … }]` to `{ action = { … } }`, and from `[]` to `{}`. Bind separate steps to separate keys, rather than an action list. A bare action without the `action` field, or an unknown binding field such as `hnt`, is a config error.

A `oneshot` mode takes one key and returns to normal. A `sticky` mode stays until a binding leaves it, and swallows keys it doesn't bind. `clear_defaults = true` starts a mode with none of Ship's bindings. Bind `client.config.reload = {}` to reload on demand.

The four default create keys select what they make. In your own binding, set `select = true` to do the same for the pressing client; without it, creation leaves the selection alone. Bindings cannot name an `attachment`. For example, `"alt-t"` above creates an empty tab without moving you.

`server.` actions are the CLI's commands with the same names and options: `server.tab.create.starter = "shell"` is `ship tab create --starter shell`. `client.` actions act on your own view, like `client.pane.focus`, `client.sidebar.toggle` and `client.detach`; Ship's own defaults are in [`defaults.toml`](../crates/ship-client/src/keymap/defaults.toml). Key spellings like `ctrl-alt-x` and `shift-tab` come from [crokey](https://docs.rs/crokey).

### The status line

There's no permanent status bar. A one-row overlay appears at the bottom of the pane area only when there's something to say: you're outside normal mode, the client is reconnecting, the selected pane's program has exited, or an action failed. It shows the mode, any status, and hints for the mode's keys. A binding's optional `label` overrides the word derived from its action: the mode name, a direction, the chord sent, or the action name. For example, the `f` binding above shows `f fill` instead of `f zoom`. Unbound chords don't appear in the hints.

## Logs and levels

Server logs live in `$XDG_STATE_HOME/ship/logs/<port>/`, or `~/.local/state/ship/logs/<port>/` when `XDG_STATE_HOME` is unset, on Linux and macOS. Every run starts a new file named with its local start time, including subseconds. `current.log` links to the part being written:

```sh
tail -F "${XDG_STATE_HOME:-$HOME/.local/state}/ship/logs/43179/current.log"
```

`tail -F` follows the path when `current.log` moves to a new run or part. Replace `43179` with your server's port, or use the `log` path from `ship server status` to open one specific part.

Background startup failures name their run file. Foreground servers write the same files and also print to stderr. Logs survive a server crash; restarting creates a new file instead of overwriting the crashed run's log. Each line has local time with its UTC offset.

A full-screen client writes `logs/client/<start-time>-<pid>.log` under the same state directory, with `current.log` pointing at the newest client's file. It does not log to stderr while drawing. One-shot commands use stderr and create no log file.

Files continue in numbered parts (`<start-time>.2.log`, and so on) at 5 MiB, without splitting a log event. `current.log` follows each new part. At startup and rollover, Ship removes the oldest log files to keep each port's directory, and the client directory separately, at about 200 MiB. The current part is never removed. Files are owner-only (0600), and newly created log directories are owner-only (0700) on Linux and macOS. Where symbolic links are unavailable, open the run file directly. Raw process output from a background server stays in its first part after rollover; logged panics go to the current part.

HTTP responses carry an `x-request-id` for correlation: Ship preserves a supplied ID or generates a UUID. Request spans and completion summaries are logged at debug; 5xx completions are warnings, while success and 4xx remain debug. Summaries include the request ID, method, path, status and duration, even when debug spans are disabled. The explicit header allowlist also records a valid request `x-ship-attachment-id` as `attachment_id` on spans and summaries, and the response's numeric `x-ship-revision` as `revision` on summaries. Missing or malformed values are omitted from diagnostics without changing request handling. Query strings, bodies and all other headers, including other `x-ship-*` headers, are not recorded by this middleware. Duration measures time until the response is ready, not the lifetime of a streaming body.

By default, Ship logs at `info` and dependencies at `warn`. Set `SHIP_LOG` to `error`, `warn`, `info`, `debug` or `trace` before starting a process:

```sh
SHIP_LOG=debug ship server --port 43180
SHIP_LOG=debug ship
```

At `debug` and `trace`, this also enables actor message spans. A background server inherits the launching command's environment. Changing that environment does not change an already-running server's level. An invalid level fails startup.

`RUST_LOG` replaces the whole filter, including `SHIP_LOG`. For example, `RUST_LOG=warn,ship=debug,ratatui_ghostty=debug,kameo=info` sets Ship and its terminal widget to debug and enables actor spans. Invalid filter syntax fails startup.

The server also logs Ghostty's own messages under the `libghostty_vt::ghostty` target, with Ghostty's scope in a `ghostty_scope` field. They name terminal input Ghostty doesn't handle, such as `unimplemented mode: 2031`, which helps tell a Ghostty gap from a Ship bug when a pane renders wrong. Warnings appear by default; add `libghostty_vt=info` to the filter to also see unknown sequences Ghostty ignores. Ghostty emits nothing below `info`.

To inspect or change a running server without restarting it:

```sh
ship server status                  # Health JSON, including the current log part
ship server log                     # Active/startup filters and current file as JSON
ship server log debug               # Same level mapping as SHIP_LOG=debug
ship server log --filter 'warn,ship=trace'
ship server log --reset              # Restore the startup filter
```

Each log command prints one JSON line with `active`, `startup` and `file`. Use only one of a level, `--filter` or `--reset`. Invalid changes fail and leave the filter unchanged. Changes take effect immediately without affecting panes and last until reset or server exit. Neither `server log` nor `server status` starts a missing server. Use `--server-url` or `SHIP_SERVER_URL` to select another server.

To record a timeline, set `SHIP_TRACE` before starting a server or client. For example, in separate terminals:

```sh
SHIP_TRACE=/tmp/ship-trace ship server --port 43180
SHIP_TRACE=/tmp/ship-trace ship --server-url http://127.0.0.1:43180
```

Type a few keys, detach the client with `alt-q`, then stop that server with `ship --server-url http://127.0.0.1:43180 server stop`. Clean exits finish `/tmp/ship-trace/server-<pid>.json` and `client-<pid>.json`. Open each file in [Perfetto](https://ui.perfetto.dev) to inspect operations such as `turn`, `send`, `route`, `command`, `publish`, `apply` and `redraw`. Client and server timelines are separate, not joined.

Timelines record debug-level Ship operations and actor messages regardless of `SHIP_LOG`, `RUST_LOG` or live log-filter changes. Setting `SHIP_TRACE` on a client does not enable it on an already-running server. The directory is created if needed; one-shot commands record no timeline. Without `SHIP_TRACE`, no timeline layer or file is created. Recording works in debug and release builds. Timeline files are not size-limited or pruned, and crashes may leave them incomplete; use recording for a bounded session. The payload-recording boundary below also applies to timelines.

At debug detail, a warning inside an operation shows the nested operations it came from. For example, a failed pane launch has a chain like:

```text
WARN actor.handle_message{actor.name="ServerState" ... CreatePane}:spawn{pane=pane:...}: ... error=...
```

Read this as "the state actor was handling CreatePane, and spawn failed." Actor messages start new chains, so the sending HTTP request is not included. Long-running pane tasks have their own `pane` roots; the client and server each log their own steps.

Ship does not automatically record terminal contents, typed or pasted input, clipboard data, full program argument lists or environments. Failure messages keep useful program names and dependency error details, which can contain sensitive information. Check logs before sharing them.

## Upgrading

If an older config uses `hint` on a key binding, rename it to `label`. `hint` is no longer accepted. The label still appears after the chord in the status line.

The mouse change moves protocol 10 to 11 and replaces `ship tab rename` / `ship pane rename` and their bindings with `set`. Use `--name ""` to clear a name; omitting `name` leaves it unchanged.

Diagnostics moves protocol 12 to 13, adds live log-filter control and includes the current log part in health/status.

Update the client and server together. Before replacing the binary, finish or save running work and run `ship server stop` with the client that matches the running server. Stopping the server ends every program in it. Then rebuild or install the new version and start again. The new client refuses to talk to an older server with a different protocol version, including to stop it; keep the old client available until that server is stopped.
