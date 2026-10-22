# Upgrading

Update the window and the server together. Before installing a new version, stop the running server with the old `ship` (`ship server stop`), since a newer `ship` can't talk to an older server. Stopping ends every program in it, so finish or save your work first.

If your config stops loading after an upgrade, check for these older spellings:

| Old | New |
| --- | --- |
| `"key" = [{ ... }]` | `"key" = { action = { ... } }` |
| `"key" = []` | `"key" = {}` |
| `hint = "..."` | `label = "..."` |
| `server.tab.rename`, `server.pane.rename` | `server.tab.set`, `server.pane.set` |
