# Spec Delta

## ADDED Requirements

### Requirement: Error bodies
An HTTP error response's body SHALL be one error object. An internal error SHALL have `source` equal to `internal`, `message`, and `type` with the error's `kind` and, for kinds that carry it, `data`. It MAY have `data` and `causedBy`, the error it wraps as a nested error object. An external error SHALL have `source` equal to `external`, `code` shaped like `type`, `error`, a text snapshot of the foreign error, and optional `data`. Keys SHALL use camelCase and kinds SHALL use snake_case.

#### Scenario: Missing pane over HTTP
- **WHEN** a client requests a pane that doesn't exist
- **THEN** the 404 body has `"source":"internal"` and `"type":{"kind":"not_found"}`

#### Scenario: Error with a cause
- **WHEN** a request fails with an error that wraps another
- **THEN** the wrapped error appears under `causedBy` as its own object

### Requirement: CLI error line
When a `ship` command other than `ship config check` fails, it SHALL print its error as one error object, as in Error bodies, on a single line that is the last line on stderr, write nothing to stdout, and exit 1. Log lines MAY come before it. It SHALL print the same line whether or not stderr is a terminal. Usage errors that the argument parser reports SHALL keep their text and exit 2, and `--help` and `--version` SHALL behave as before.

#### Scenario: Missing pane on the CLI
- **WHEN** a user runs `ship pane get --pane <id>` for a pane that doesn't exist
- **THEN** stdout is empty, the last line on stderr parses as JSON with `.type.kind` equal to `not_found`, and the exit status is 1

#### Scenario: Misspelled flag
- **WHEN** a user runs `ship pane read --pain <id>`
- **THEN** the parser's usual text and suggestion are printed on stderr and the exit status is 2
