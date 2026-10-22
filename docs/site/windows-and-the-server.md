# Windows and the server

<!--
Scope: The window and server split, starting and stopping the server, and using Ship on another machine.
Not here: saved tabs (saved-tabs.md), the HTTP protocol (openspec/specs/).
-->

Ship runs in two parts. A background server owns your tabs, panes and the programs in them. Running `ship` opens a window onto that server. Closing the window, or pressing `alt-q`, leaves everything running, and running `ship` again brings you back.

The first `ship` starts the server for you. It restores your [saved tabs](saved-tabs.md), or opens one tab with a shell if there's nothing to restore. `ship server status` tells you whether a server is running, and `ship server stop` shuts it down, ending every program inside.

You can open Ship from several terminals at once. The windows share the work: the same tabs, the same programs, the same output. Each window picks its own tab and pane, folds its own sidebar and keeps its own mode. Panes are sized to fit the smallest window looking at that tab, so a bigger window shows some empty space around them.

If a window loses its connection to the server, it reconnects on its own.

## Using Ship on another machine

The server only accepts connections from its own machine. To use it from somewhere else, forward its port over SSH:

```sh
ssh -N -L 43179:127.0.0.1:43179 otherbox
```

Then, in another terminal:

```sh
ship --server-url http://127.0.0.1:43179
```

Run `ship` on the other machine once first so its server is up. Passing `--server-url` makes Ship connect only to that address, so it won't quietly start a local server if the tunnel isn't up. All the CLI commands take `--server-url` too, or set `SHIP_SERVER_URL` once.

The server also refuses web pages and proxies, so opening it in a browser or putting it behind a proxy such as Tailscale Serve or Caddy won't work. Use an SSH tunnel to reach it from elsewhere.

Sounds and clipboard copies happen on the machine you're sitting at. Through a tunnel, `ship server stop` stops the server but may report that it's still running, because the tunnel keeps accepting connections after the server is gone.
