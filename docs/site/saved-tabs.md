# Saved tabs

<!--
Scope: What comes back after a restart and what doesn't, where the file lives, and how to start fresh.
Not here: the file's format (openspec/specs/session-restore).
-->

After a server restart or a reboot, Ship brings back your tabs, their nesting and names, the splits and their sizes, zoom, and each pane's last directory. Every pane starts a fresh shell. Running programs, agent conversations and scrollback don't come back.

Ship saves a few seconds after each change, and `ship server stop` saves right away. The file lives at `~/.local/state/ship/tabs/43179.json` (under `$XDG_STATE_HOME` if you set it; the number is the server's port).

If a pane's directory is gone, it opens in your home directory instead. If the file can't be read, Ship starts with no tabs and leaves the file alone so you can look at it.

## Getting back the previous tabs

Each time the server starts and saves, it keeps the file it replaced as `43179.json.prev`. To go back to it:

```sh
ship server stop
SAVED="${XDG_STATE_HOME:-$HOME/.local/state}/ship/tabs/43179.json"
cp "$SAVED.prev" "$SAVED"
ship
```

Copy the `.prev` file somewhere safe if you want to keep it, since the next run replaces it.

## Starting fresh

`ship server --no-restore` starts a server in the foreground without loading the saved tabs. Add `--starter` to get one shell tab. Your first change then replaces the saved tabs, keeping the old ones as `.prev`.

To try things without touching your usual tabs, run a second server on another port:

```sh
ship server --port 43326 --no-restore --starter
ship --server-url http://127.0.0.1:43326        # in another terminal
```
