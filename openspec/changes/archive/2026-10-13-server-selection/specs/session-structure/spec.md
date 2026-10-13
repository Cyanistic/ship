# Spec Delta

## ADDED Requirements

### Requirement: Creates that select
`ship tab create` and `ship pane create` SHALL accept `--select`. With it, the create SHALL select for one attachment the new pane, or the new tab when it has no pane, in the same edit that creates it, so no client sees the new tab or pane before that attachment selects it. Without `--select`, a create SHALL NOT change any selection. The HTTP create bodies SHALL carry this as an optional `select` holding the attachment ID.

#### Scenario: An agent creates without stealing focus
- **WHEN** a client has a pane selected and an agent in another pane runs `ship pane create --tab <id> -- cargo test`
- **THEN** the new pane runs `cargo test` and the client's selection doesn't move

#### Scenario: A script creates for a client
- **WHEN** a script runs `ship pane create --tab <id> --select --attachment <a> -- sleep 9` while a second client watches `ship view list` in a loop
- **THEN** client `<a>` lands on the new pane, and no listing shows the new pane without `<a>` selecting it

#### Scenario: A tab with a starter
- **WHEN** a script runs `ship tab create --starter --select --attachment <a>`
- **THEN** client `<a>` lands on the new tab's shell pane

#### Scenario: An empty tab
- **WHEN** a script runs `ship tab create --select --attachment <a>`
- **THEN** client `<a>` selects the new tab and shows its no-pane hint

### Requirement: The attachment a create selects for
A selecting create SHALL take its attachment from `--attachment ID`, defaulting to `SHIP_ATTACHMENT_ID`. With `--select` and neither, the command SHALL fail before any request, saying to pass `--attachment` or set `SHIP_ATTACHMENT_ID`, and exit 1. A create naming an attachment that doesn't exist SHALL fail with a not-found error and create nothing, including a starter's program.

#### Scenario: No attachment
- **WHEN** a user outside any client runs `ship tab create --select` with `SHIP_ATTACHMENT_ID` unset
- **THEN** the command fails asking for `--attachment`, exits 1, and `ship tab list` is unchanged

#### Scenario: Unknown attachment
- **WHEN** a user runs `ship tab create --starter --select --attachment <ended>` naming an attachment that has ended
- **THEN** the command fails with a not-found error, exits 1, `ship tab list` is unchanged and no shell was left running
