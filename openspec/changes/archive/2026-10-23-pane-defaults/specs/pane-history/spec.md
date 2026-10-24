## MODIFIED Requirements

### Requirement: History size
Each pane's history SHALL be limited to its own `scrollback_bytes`, with no line limit. A pane SHALL take it from its create request, else from `[server.defaults.pane] scrollback_bytes` when it is created. `ship pane set --scrollback-bytes` and `server.pane.set` SHALL change it on a live pane: raising it SHALL keep the existing history, and lowering it SHALL drop the oldest history beyond the new limit at once. The help for `ship pane set` SHALL say so.

#### Scenario: Default history keeps a long log
- **WHEN** a 200-column pane on the default setting prints `seq 5000` and is scrolled to the top
- **THEN** it shows line 1

#### Scenario: Longer output exceeds the cap
- **WHEN** a 200-column pane on the default setting prints `seq 50000` and is scrolled to the top
- **THEN** older rows have been pruned to respect the 10,000,000-byte cap; retaining line 1 is not required

#### Scenario: Size on create
- **WHEN** a script runs `ship pane create --scrollback-bytes 50000000`
- **THEN** `ship pane get` shows `scrollbackBytes: 50000000` for the new pane

#### Scenario: Lowering drops old history at once
- **WHEN** a pane has printed `seq 1 700000` and a script runs `ship pane set --pane <id> --scrollback-bytes 1000000`
- **THEN** `ship pane read` no longer returns the oldest lines, without waiting for more output

#### Scenario: Raising keeps history
- **WHEN** a script raises a pane's `scrollback_bytes` and the program prints more
- **THEN** the history from before the change is still readable and the history grows past the old limit
