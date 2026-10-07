# Research notes

- [Terminal transport handoff](terminal-transport-handoff.md): current compressed full-snapshot direction for structural state and terminal screens, historical native benchmarks, and bounded live loopback codec evidence. Production compression and attachment/revision semantics remain open. Historical temporary paths remain in the handoff; the [durable experiment README](../../experiments/terminal-transport/README.md) maps them to preserved sources/results and provides safe reproduction commands.
- [Pane terminals transport probe](pane-terminals-transport-probe.md): streaming NDJSON input POST, raw-body limits, compressed SSE flushing, HTTP/2 auto-detection, TCP_NODELAY, utoipa schemas on crossterm key mirrors, and Herdr's Windows-only portable-pty patches. macOS loopback only.
- [Terminal feasibility](terminal-feasibility.md): earlier native capture, wrapper compatibility and structdiff feasibility findings. Its patch experiments are historical evidence, not the current full-snapshot pipeline.
- [Typed relay contract](typed-relay.md): self-contained typed publication, sink-adapter and client-delivery contract; no private-repository dependency or implemented Ship actor integration.
- [Config and plugin ownership](config-plugin-ownership.md): config/profile and plugin ownership research.

Current planning: [session structure, typed IDs and actors](../planning/session-structure.md), plus the [later terminal checkpoint](../planning/first-terminal-slice.md).

These notes are research evidence, not implicit authorization to adopt a production protocol or change architecture.
