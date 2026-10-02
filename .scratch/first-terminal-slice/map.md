# Plan the first client/server terminal slice

Labels: wayfinder:map
Status: open

## Destination

An implementation-ready plan for one interactive terminal across Ship's separate HTTP/SSE client/server processes, with dependency-fit evidence and no blocking first-slice design decisions remaining.

## Notes

Planning only; this map does not implement the product. Work through one non-research decision ticket per session. Claim before work and record resolutions on the ticket, linking only a gist here.

This effort uses the local Markdown tracker because no external tracker is configured. See [local tracker operations](../README.md). Cyan can configure another tracker with `/setup-matt-pocock-skills` later.

Consult the global style skill (mixed audience for plans), grilling and domain-modeling for live decisions, and prototype for concrete experiments. Use engineering-philosophy for any code experiment. No permanent test code; temporary probes and real workflow checks are allowed. No public-interface, security, or architectural choices without Cyan's approval.

[First terminal slice](../../docs/planning/first-terminal-slice.md) is the readable planning view; decision provenance lives in tickets. The product brief remains revisable. No implementation is claimed.

## Decisions so far

- [Choose the first-slice architecture and working rhythm](issues/01-first-slice-direction.md): separate authoritative server/thin client, HTTP/SSE and library-generated revisioned state patches; small commits and no initial permanent tests.

## Not yet specified

- Whether capture/diff experiment results expose new integration constraints; graduate precise questions after reviewing the results.
- Linux/macOS validation details and toolchain/build requirements may need further decisions after integration experiments reveal actual constraints.

## Out of scope

- Splits/tabs and the rest of roadmap milestone 1: beyond this one-terminal checkpoint.
- Remote implementation, direct network exposure and local authentication: deferred; loopback access limitation is explicitly accepted for now.
- OpenAPI/SDK generation, config/port customization and preemptive dependency forks.
- Saved server-restart recovery, agent integration, audio/notifications and automation command catalog.
- Full multi-client implementation and no-viewer sizing policy; the minimum-size-among-current-tab-viewers direction is already recorded for later work.
- Production implementation: hand off when the plan is clear, rather than treating build tasks as decision tickets.
