# Writing Code

Rules for agents that write or change Ship's Rust. `AGENTS.md` still applies; this adds what only matters when editing code.

## Size

Get small by expressing each invariant once through Rust types, derives and macros, not through opaque compression or generic frameworks built for hypothetical features.

## Verification

Ship starts with zero authored test code. Verify work with builds, formatting, Clippy, real workflows and temporary probes. Propose a permanent test only after encountering flaky behavior, and add it once Cyan agrees on its coverage.

## Design

Keep one source of truth. Store what can't be derived and compute the rest; duplicated or cached state needs a reason.

Model state as state. When something has a current value, send and store the value, not the sequence of changes that produced it. Reserve event streams for things where each occurrence matters.

Name your shapes. Prefer named types over anonymous inline structure, so data can be reused, documented and passed around.

Earn every layer. A wrapper, abstraction or coordination mechanism should do something the thing beneath it doesn't. If it only forwards or restates, remove it.

Check what's already guaranteed. Before building machinery, find out what the platform and dependencies already handle, verify it, and cover only the gap.

## Errors and results

Construct errors with `err!`. Wrap an existing `AppError` with `.context(...)`, which keeps its error code.

Discard a result you don't need with `.ok()`, not `let _ =`.

## Platform-specific code

Keep platform-specific code behind `cfg`. Use the portable std or dependency API wherever one exists. Confine each Unix-only mechanism to one small function or module gated with `#[cfg(unix)]`, and don't call `nix` or `libc` from otherwise portable code. On other platforms, give the item a plain fallback or leave it missing, so a port fails to compile exactly at that boundary.
