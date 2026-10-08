# Research notes

- [Terminal transport handoff](terminal-transport-handoff.md): current compressed full-snapshot direction for structural state and terminal screens, historical native benchmarks, and bounded live loopback codec evidence. Production compression and attachment/revision semantics remain open. Historical temporary paths remain in the handoff; the [durable experiment README](../../experiments/terminal-transport/README.md) maps them to preserved sources/results and provides safe reproduction commands.
- [Pane terminals transport probe](pane-terminals-transport-probe.md): streaming NDJSON input POST, raw-body limits, compressed SSE flushing, HTTP/2 auto-detection, TCP_NODELAY, utoipa schemas on crossterm key mirrors, and Herdr's Windows-only portable-pty patches. macOS loopback only.
- [Screen cost per workload](screen-cost.md): Ship, Herdr and Zellij CPU across spinner, rain, full-colour and scroll workloads; Zellij's dirty-line technique; a server-rendering probe; Ghostty's per-row dirty flags; and the deferred dirty-rows-on-the-wire idea. macOS loopback only.
- [Terminal feasibility](terminal-feasibility.md): earlier native capture, wrapper compatibility and structdiff feasibility findings. Its patch experiments are historical evidence, not the current full-snapshot pipeline.
- [Typed relay contract](typed-relay.md): self-contained typed publication, sink-adapter and client-delivery contract; no private-repository dependency or implemented Ship actor integration.
- [Config and plugin ownership](config-plugin-ownership.md): config/profile and plugin ownership research.
- [Agent state detection](agent-state-detection.md): which agents report state through hooks and which need screen rules, Herdr's manifest format and engine size, and a deferred leaning toward reading Herdr's manifests with a small engine. From Herdr's source and docs; nothing built in Ship.

These notes are research evidence, not implicit authorization to adopt a production protocol or change architecture.
