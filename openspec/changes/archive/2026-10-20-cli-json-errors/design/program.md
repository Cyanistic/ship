# Program: CLI errors as JSON

Status: Locked on 2026-10-20. Cyan approved it in Plannotator review ("LGTM").

## Rationale

This is a document-only plan. The skeleton below names real paths and shows declarations, but no source file has been created or changed for it, and nothing here has been compiled.

The architecture's shape A puts the whole failure contract in `main`: `Err` becomes one JSON line on stderr and exit 1, and `Ok(code)` becomes that exit code. Only `ship config check` ever returns `Ok(FAILURE)`. The JSON spelling (`causedBy`, snake_case kinds) changes on the shared types in `ship-core`, so the CLI line and HTTP bodies can't drift apart.

What each touched file owns:

- `crates/ship-core/src/error.rs`: how an `AppError` is spelled in JSON, for both HTTP and the CLI.
- `crates/ship/src/main.rs`: turning `run`'s result into stderr output and an exit code.
- `crates/ship/src/commands.rs`: `config check` reporting its findings and saying whether it found any.
- Specs and the guide: the error line as a documented contract.

## Skeleton map

### Added

No new source files. When this paper locks, the OpenSpec change gets its usual artifacts, which are documents rather than implementation: `proposal.md`, `design.md` (a summary that points at the three papers), the delta specs listed in slice 3, and `tasks.md` (these three slices).

### Moved

None.

### Replaced

**`crates/ship-core/src/error.rs`, slice 1.** Two serde attributes. Fields, variants and Rust names stay.

```rust
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum ErrorCode { /* unchanged variants: NotFound -> "not_found", UpstreamHttpStatus(u16) -> "upstream_http_status", ... */ }

#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct InternalError {
    pub message: Cow<'static, str>,
    #[serde(rename = "type")]
    pub code: ErrorCode,
    pub data: Option<Value>,
    pub caused_by: Option<Box<AppError>>, // serialized as "causedBy"
}
```

`ExternalError` (`code`, `error`, `data`) has only one-word keys and stays as it is.

**`crates/ship/src/main.rs`, slice 2.** `run` and `dispatch` return an exit code on success. `main` prints the error itself.

```rust
fn main() -> ExitCode;
// Ok(code) => code.
// Err(error) => one line on stderr: serde_json::to_string(&error), falling back
//   to today's "ship: {error}" text if serialization fails; then ExitCode::FAILURE.

fn run(cli: Cli, source: Option<ValueSource>) -> Result<ExitCode>;

async fn dispatch(
    mut cli: Cli,
    source: Option<ValueSource>,
    diagnostics: &diagnostics::Diagnostics,
) -> Result<ExitCode>;
// The `config check` arm returns its ExitCode early. Every other arm keeps
// producing Result<()>, and the match ends with one `.map(|()| ExitCode::SUCCESS)`,
// so the other arms' bodies don't change.
```

The clap lines at the top of `main` (`get_matches`, `from_arg_matches(..).unwrap_or_else(|e| e.exit())`) stay as they are.

**`crates/ship/src/commands.rs`, slice 2.** `check_config` reports its findings and returns the exit code instead of a summary error.

```rust
/// `ship config check`: prints each part's first error on stderr as text.
/// Exits 1 when it found any, without a further error line.
pub fn check_config(path: &Path) -> Result<ExitCode>;
// findings empty => Ok(ExitCode::SUCCESS); otherwise Ok(ExitCode::FAILURE).
// Err stays possible only for failures of the check itself, which print as JSON.
```

### Untouched

- `crates/ship/src/cli.rs` and the clap parse path. FR-006 already holds.
- `crates/ship-client/src/automation.rs`. The `send --wait` context messages already satisfy FR-007.
- `crates/ship-server`. It serializes `AppError` as it does today and picks up the new spelling from `ship-core`. The OpenAPI document is generated at runtime from the same types, so there's no checked-in schema to regenerate.
- `crates/ship-client/src/ui`. Status line errors and the `detached` / `server stopped` exit messages stay.
- `crates/ship/src/diagnostics.rs`. Logs already write synchronously to stderr, so the JSON line is last without changes.

### Proposed settings

None.

### Blocking unknowns

None.

## Build order

1. **Error spelling** (`crates/ship-core/src/error.rs`). Input: any failing request. Output: HTTP error bodies use `causedBy` and snake_case kinds.
   Checks:
   - `cargo build`, `cargo clippy --all-targets` and `cargo fmt --check` are clean.
   - `curl -s http://127.0.0.1:43179/api/v0/panes/<missing pane ID>` returns a 404 body with `"type":{"kind":"not_found"}`.
   - A failure with a cause shows `causedBy` in its body.
   - `ship pane get --pane <missing>` still fails with the server's message, which proves the client deserializes the new spelling.

2. **JSON error line and config check exit** (`crates/ship/src/main.rs`, `crates/ship/src/commands.rs`). Input: any failing command. Output: one JSON line, last on stderr, exit 1, nothing on stdout.
   Checks:
   - Same build, Clippy and fmt checks.
   - `ship pane get --pane <missing>`: `tail -n 1` of stderr parses with `jq` and has `.type.kind == "not_found"`, stdout is empty, exit 1.
   - `ship pane send --pane <blocked pane> --wait ...`: `.message == "nothing was sent"`. A pane that never reports gives `.message == "the input was sent"` with the stall in `.causedBy`.
   - `ship pane read --pain x`: clap's text and exit 2, the same as on `main`. `ship --help` and `ship --version` exit 0 with their usual output.
   - `ship config check <broken file>`: the findings as text, exit 1, and no JSON line. `ship config check <valid file>`: nothing printed, exit 0.
   - `SHIP_LOG=debug ship pane get --pane <missing>`: log lines come first, and the JSON is still the last line.

3. **Specs and guide.** Input: the locked papers and slice 2's real output. Output: the contract is written down.
   - Delta specs (drafted when this paper locks, checked against slice 2's output here):
     - `health-exchange`, ADDED "Error bodies": the shape of `AppError` (`source`, `message`, `type.kind`, `type.data`, `data`, `causedBy`; `code`, `error`, `data` for external errors), camelCase keys, snake_case kinds, and that a failed CLI command prints the same object as its last stderr line with exit 1 and empty stdout.
     - `pane-automation`, MODIFIED "Automation errors": the error is that JSON line, and the "nothing was sent" / "the input was sent" / "the input may have been sent" messages are the outermost `message`.
     - `config-file`, MODIFIED "Config check": the findings are text, the exit status is 1 when there are any, and there's no further error line.
   - `docs/guide.md`: a short section on reading a failed command's error with `jq`, telling from a `send --wait` error whether the input went out, and exit statuses 1 and 2. The README doesn't change.
   Checks:
   - `openspec validate cli-json-errors` passes.
   - The guide's error example matches the actual output captured in slice 2.

## Deviation log

Empty.
