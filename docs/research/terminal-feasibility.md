# Terminal capture and patch feasibility

The supplied session plus a small compatibility patch passed the macOS feasibility checks. Owned screen conversion and JSON patch application worked. Cyan accepted the default whole-vector strategy for the first slice and deferred optimization until profiling real Ship usage shows a bottleneck.

This is dependency evidence, not a completed Ship client/server implementation. Decisions live in the [capture ticket](../../.scratch/first-terminal-slice/issues/02-screen-capture.md) and [diff ticket](../../.scratch/first-terminal-slice/issues/03-screen-diffs.md).

## Dependency setup and compatibility patch

The published ratatui-ghostty 0.2.0 depends on libghostty-vt 0.1.1. Its pinned Ghostty source requires Zig 0.15.2; the actual build failed with installed Zig 0.16.0. This is the old dependency's requirement, not the requirement of current bindings.

The successful scratch setup used:

- ratatui-ghostty 0.2.0 source, with its original manifest's binding dependency changed to 0.2.1.
- [libghostty-rs at 8953a740bc378cec3e07e1f6ca949f0595eab19b](https://github.com/uzaaft/libghostty-rs/tree/8953a740bc378cec3e07e1f6ca949f0595eab19b), declaring binding version 0.2.1 and requiring Zig 0.16.x.
- Cargo path patches for the wrapper, libghostty-vt and libghostty-vt-sys. A patch alone cannot override the wrapper's original incompatible version requirement.
- Zig 0.16.0, portable-pty 0.9.0, Ratatui 0.30.0 with serde, and structdiff 0.7.3 with serde in the final probe.

The wrapper changes were confined to session.rs: remove the old Options constructor, use Terminal::new(cols, rows), set_scrollback_max_lines(Some(scrollback)), use typed ColorScheme::Dark/Light, and remove the unused ffi import. The new byte limit remains at its default; equivalence of old/new scrollback semantics was not established.

This source has not been imported into Ship or pushed to a fork. A pinned Git fork versus subtree/source import remains an open delivery choice. No upstream PR has been made.

## Existing tests and example

All 80 enabled upstream wrapper tests passed: 5 color parsing, 21 Crossterm conversion, 23 style conversion, 20 input encoding, 1 boxed callback check, 10 widget/render/viewport checks. Three existing test constructors were adapted to the newer API; assertions were unchanged. No new permanent tests were authored.

One upstream callback-movement crash reproducer was ignored and not executed. Do not claim the newer bindings fix that reproducer.

The upstream multiplexer example built. A pseudo-terminal driver observed split-pane shell output, resize reflected in both pane status and inner stty output (50 columns by 29 rows), exit status 0, and alternate-screen cleanup. Its initial-pane marker remained unobserved, so the automated driver did not fully pass. Cyan subsequently ran the example and confirmed it worked. This does not claim Pi/Neovim, Linux, or complete lifecycle coverage.

## Screen conversion and JSON proof

The first probe was 159 lines of disposable Rust. A real /bin/sh emitted staged text, colors/styles, Unicode, and scrolling output; the supplied SessionHandle processed it. The probe captured 20x6 cells/cursor, then resized to 24x8.

Executable assertions checked reconstructed Ratatui Buffer equality, accessible cursor-field preservation, JSON-deserialized structdiff patch application to the old screen, and unchanged-state empty patches. Captures included `RED café 界  é`, `READY`, a colored/underlined `EDIT` overwrite, and `scroll-07` through `scroll-12`; resize exposed `scroll-05` through `scroll-12`.

The shell exited with status 0 and was reaped, the PTY reader was destroyed, and the session reported Exited and stopped. This was graceful completion, not proof of forced cancellation or joined reader threads.

## Collection experiment

A second probe extended the program to 210 lines. Default vectors, ordered vectors, and index-keyed maps compared the same captures. All 18 strategy/transition combinations passed JSON patch application and snapshot roundtrip equality; the parent inspected the code and independently reran both probes.

Real captured transitions, parent rerun:

| Transition | Default patch JSON bytes | Ordered patch bytes | Index-map patch bytes | Vector snapshot bytes |
| --- | ---: | ---: | ---: | ---: |
| Small edit, 20x6 | 13,609 | 668 | 779 | 13,624 |
| Output overflow/scroll, 20x6 | 13,516 | 7,097 | 8,709 | 13,531 |
| Resize to 24x8 | 21,603 | 9,535 | 20,626 | 21,595 |
| Unchanged | 2 | 2 | 2 | 21,595 |

A separate synthetic 80x24 sample used repeated ASCII rows and copied style metadata, not real terminal capture. One-cell edits yielded 222,733 / 147 / 184 bytes for default / ordered / map. A one-row scroll yielded 222,733 / 10,751 / 308,844 bytes; the map snapshot was 235,142 bytes. This is evidence of strategy tradeoffs on these samples, not a representative terminal benchmark.

All timings were single debug-build measurements of diff generation only. On the synthetic sample, ordered generation took about 233 ms, maps about 1.6-1.7 ms; the default took about 0.05-0.17 ms. Release builds were not measured. These results do not establish a production bottleneck. Release optimization could improve runtime but not these JSON payload sizes for the same representation.

## Limits and deferred work

- Dimensions, buffer and cursor are read under separate locks. Paused output and 120 ms stable samples establish smoke evidence, not atomic capture during continuous output. Coherent publication remains an implementation-contract concern.
- Scratch cells retain Ratatui Color, Modifier and CellDiffOption. The probe does not establish a final Ratatui-independent wire schema.
- Ratatui equality normalizes unset symbols to spaces. Nondefault skip/diff options and alternate cursor variants were preserved by conversion code but not exercised.
- Scrolling covered output overflow, not interactive scrollback navigation. No Linux run was performed.
- Default whole-vector replacement is accepted as the initial strategy. No row-based experiments, custom algorithms, release benchmarks or patch compression are prerequisites now. Profile the actual runnable slice before optimizing.
- StructDiff can be implemented for a custom nested screen type with recurse in its parent; that would require owning patch generation/types/application for that type. This is an available escape hatch, not an approved implementation. Changing representation could change wire patches and require coordinated clients, even without changing HTTP/SSE or server ownership.

## Scratch assets

The disposable source and logs were under /tmp/ship-screen-prototype: src/main.rs, src/main-baseline.rs.bak, wrapper-compat, libghostty-rs, upstream-tests-output.txt, demo-smoke-output.txt, screen-probe-parent-output.txt and screen-fine-diff-parent-output.txt. These are temporary paths, not durable links or repository deliverables.

The final run command was `cargo run --locked --manifest-path /tmp/ship-screen-prototype/Cargo.toml`. This note preserves observed results and dependency pins if those assets disappear. It is not a fully reproducible archive of the prototype source.
