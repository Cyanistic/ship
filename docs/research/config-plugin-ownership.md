# Configuration and plugin ownership precedents

Configuration and plugin systems are deferred beyond Ship's first terminal slice. Start with fixed defaults and minimal operational/exit bindings; no config file, reload mechanism, plugin runtime or registry. The observations below are context for a later slice, not adopted implementation choices.

## Source-only comparison

Inspected Herdr at d6b40d4edd550ccea081f089605a64314f8c8b27 and Zellij 0.46.0 at ef497e1ae398c6618119d7ea4b9efc84b9c4761a. No configuration/plugin runtime tests were performed.

| Responsibility | Herdr | Zellij |
| --- | --- | --- |
| Workspace key actions | Server-owned workspace dispatch; server publishes a reduced profile for client-local actions | Server matches keys using that client's bindings and input mode |
| Client-local behavior | Separate preferences/reload handling; detach and sidebar were verified client-local | Server retains per-client bindings, modes and themes |
| Reload | Separate server and client-local paths | Configuration changes propagate to relevant server modules, with per-client state |
| Plugin execution | Server-spawned native subprocesses using the CLI/socket interface; trusted same-user programs, no sandbox | Server-hosted WebAssembly guests with permission-controlled host APIs |
| Plugin UI | Terminal panes/popups through normal server rendering | Plugin stdout ANSI interpreted into a pane by the host |

A setting's storage location, parser, executor and scope are separate questions. Server execution does not imply one shared configuration for every client. Keybindings may initiate client-local presentation or server commands. Neither project's matching location is mandatory for Ship.

Herdr's native program/CLI model and Zellij's sandboxed plugin runtime are different extension models, not interchangeable implementations of the same feature. A plain Ship command interface could support later external automation without adopting a plugin registry or runtime.

## Deferred Ship questions

Revisit only when a customization or extension slice needs them:

- Which settings are shared, per-client or machine-local, and where are they stored/read?
- Where are configurable keybindings interpreted, including local actions versus server commands?
- What reload propagation and invalid-config behavior are required?
- Is an ordinary script using the control interface sufficient, or is there a concrete need for plugin packaging/runtime/UI support?
- What trust and permission model does that actual extension need require?

For now, local presentation stays client-owned and shared session/process behavior stays server-owned. Exact binding names remain part of the first-slice interaction contract, not a configurable binding system.

## Evidence and limitations

Herdr's reduced profile and client-local actions were traced in [client shell config](https://github.com/herdrdev/herdr/blob/d6b40d4edd550ccea081f089605a64314f8c8b27/src/client/shell/config.rs) and [actions](https://github.com/herdrdev/herdr/blob/d6b40d4edd550ccea081f089605a64314f8c8b27/src/client/shell/actions.rs). Reload paths were inspected in server/headless.rs and client/config_reload.rs. Native plugin spawning was traced in [plugin runtime](https://github.com/herdrdev/herdr/blob/d6b40d4edd550ccea081f089605a64314f8c8b27/src/app/api/plugins/runtime.rs); trust and CLI-as-API behavior is described in the [pinned plugin docs](https://github.com/herdrdev/herdr/blob/d6b40d4edd550ccea081f089605a64314f8c8b27/docs/next/website/src/content/docs/plugins.mdx).

Zellij client key forwarding and server matching were inspected in [input handler](https://github.com/zellij-org/zellij/blob/ef497e1ae398c6618119d7ea4b9efc84b9c4761a/zellij-client/src/input_handler.rs) and [server routing](https://github.com/zellij-org/zellij/blob/ef497e1ae398c6618119d7ea4b9efc84b9c4761a/zellij-server/src/route.rs). Per-client configuration fanout was traced in server/lib.rs and server/screen.rs. See official [configuration](https://zellij.dev/documentation/configuration), [plugins](https://zellij.dev/documentation/plugins) and [plugin rendering](https://zellij.dev/documentation/plugin-ui-rendering) docs; the WASM runtime was inspected in server/plugins/wasm_bridge.rs at the pin above.

The full Herdr client/server keybinding partition, RemoteLocal behavior and workspace-prefix matching were not fully traced. Zellij's exact attach-time configuration submission message was not pinned; per-client server state and reconfiguration were verified. Watcher/nested/web-client paths and complete CLI override inventories were excluded. Do not generalize the representative paths to every client mode.

The original agent report is temporarily at /tmp/ship-config-plugin-ownership.md. This condensed note preserves the relevant findings without depending on that file surviving.
