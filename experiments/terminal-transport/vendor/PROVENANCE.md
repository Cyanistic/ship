# Preserved experimental dependencies

These sources keep the scratch benchmark runnable. They are not Ship production dependencies, an upstream contribution, or a decision about production patch delivery. Scratch and upstream sources were not edited during this import.

## ratatui-ghostty wrapper

- Copied in full from `/private/tmp/ship-screen-prototype/wrapper-compat`, including its tests and examples.
- Baseline: published `ratatui-ghostty` **0.2.0**, repository <https://codeberg.org/jint/ratatui-ghostty>.
- Packaged `.cargo_vcs_info.json` records commit **`be6d2d89105ca8cb867e7963888f82ba58b52316`**. That metadata is preserved in `../preservation/wrapper-cargo-vcs-info.json`.
- Published `.crate` SHA-256: **`2dbd592bc403a76d2eb5c491ebe5041f1973bd20d7f439970af2fe9ea1b4ac32`**.
- `wrapper-compat.patch` is a generated unified diff against the installed published crate's original manifest and every differing supplied file. Differences are limited to `Cargo.toml`, `src/session.rs`, and three existing tests: `input_encode.rs`, `terminal_move_after_callback.rs`, and `widget_render.rs`. All other supplied files match that crate byte-for-byte.

Existing scratch changes, preserved rather than newly implemented:

1. Raise `libghostty-vt` from `0.1.1` to `0.2.1`.
2. Register the demo at `examples/demo/main.rs`.
3. Replace `TerminalOptions` with `Terminal::new(cols, rows)` and `set_scrollback_max_lines(Some(scrollback))`.
4. Use typed `ColorScheme::Dark` / `Light` and remove the obsolete `ffi` import.
5. Adapt those three tests to the same constructor/scrollback API. No new test cases were added by this import.

The scrollback byte limit remains at its default. Equivalence with the old scrollback semantics was not established by the scratch probe.

License: the published manifest declares **MIT**, but neither the package nor the recorded source commit contains a license/copyright notice file. The Codeberg tree API for that exact commit was checked during import. `wrapper-compat/LICENSE` is an explicitly labeled local notice containing standard MIT terms, not a claimed upstream file or invented copyright attribution. Resolve any missing upstream notice before public redistribution. No publishing is part of this task.

## libghostty-rs bindings

- Repository: <https://github.com/uzaaft/libghostty-rs>.
- Exact checkout: **`8953a740bc378cec3e07e1f6ca949f0595eab19b`**.
- Copied from `/private/tmp/ship-screen-prototype/libghostty-rs`, excluding `.git` and build/cache artifacts.
- Scratch checkout was clean at import. No binding patches were present; every copied file matches the supplied checkout.
- Both crates retain version **0.2.1**. Their workspace manifests, checked-in generated FFI bindings, build script, examples, font license, and upstream `LICENSE` are unchanged.
- Manifests declare **MIT OR Apache-2.0**; the checkout supplies a MIT `LICENSE`, which is preserved. No Apache license file was supplied.

## Native Ghostty is fetched, not vendored here

`libghostty-rs/crates/libghostty-vt-sys/build.rs` pins:

- Repository: <https://github.com/ghostty-org/ghostty>.
- Commit: **`22d13172cde98a0a4dda05d3d6a3fcb0dd8ed018`**.

Without an override, the build script clones and checks out that commit into Cargo's `OUT_DIR/ghostty-src`, then runs Zig. The native engine is outside this preserved Rust source import. Its source, licenses and Zig package downloads remain build-time material under ignored build directories. This is not a complete offline source bundle.

The script supports `GHOSTTY_SOURCE_DIR` for a supplied native checkout and `GHOSTTY_ZIG_SYSTEM_DIR` for a prepared Zig package store. These are optional upstream capabilities, not required paths in this experiment. The verification build used neither override. Default optimized native builds use `ReleaseFast` and `baseline` CPU. Zig 0.16.0 and Rust/Cargo 1.98.0 were used successfully on macOS arm64.

## Local configuration changes

Only the experiment's root `Cargo.toml` changes dependency locations: all three crates.io patches now use relative `vendor/` paths. Workspace exclusions let the bindings inherit their own vendored workspace settings instead of the experiment's settings. The experiment remains an independent `[workspace]`. `Cargo.lock` is byte-identical to the scratch lockfile. No vendored source or generated binding edits were made.
