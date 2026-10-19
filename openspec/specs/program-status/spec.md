# program-status Specification

## Purpose

Keep what each pane's program reports about itself through OSC 7501, the Program Status Protocol (revision 0.4), so clients and scripts can see it. A **status record** is one thing a program last reported: a state (`idle`, `working`, `done`, `blocked` or `error`) with an optional `kind`, `progress`, `app`, `title` and `msg`, kept under its id. The root record's id is empty. Ship owns no per-agent knowledge: any program that speaks the protocol is shown without a Ship update.

## Requirements

### Requirement: Support query
Every pane SHALL answer the protocol's support query, `OSC 7501 ; ?`, with the states `idle`, `working`, `done`, `blocked` and `error` and the kinds `permission`, `question` and `auth`, ending the reply with the terminator the query used. The reply SHALL reach the program before any other reply that the same output provokes, such as an answer to DA1.

#### Scenario: Query before DA1
- **WHEN** a pane's program writes the support query followed by DA1 in one write
- **THEN** it reads the support reply before the DA1 reply

### Requirement: Status records
Each pane SHALL keep at most one status record per id, with the root record under the empty id. A report SHALL replace the record under its id completely and make it the most recently updated. A `clear` report SHALL remove the record under its id and every record beneath it, so a `clear` without an id removes them all.

#### Scenario: Child records beside the root
- **WHEN** a pane's program reports `working`, then `working` with `id=build`, then `blocked` with `id=build/test`
- **THEN** `ship pane get` shows records under `""`, `build` and `build/test`

#### Scenario: Subtree clear
- **WHEN** the program then sends `clear` with `id=build`
- **THEN** only the root record remains

### Requirement: Reports Ship doesn't understand
A report whose id doesn't match the protocol's grammar, one to 32 of `A-Z a-z 0-9 _ . + -` per segment joined by `/`, SHALL be dropped. A report with an unknown state SHALL be dropped, an unknown `kind` SHALL be treated as absent, and unknown keys SHALL be ignored.

#### Scenario: Bad ids
- **WHEN** the program reports with `id=a//b`, or with an empty `id=`
- **THEN** neither report changes the pane's records

#### Scenario: Unknown key
- **WHEN** the program reports `working` with an extra key `color=red`
- **THEN** the pane keeps the working record without it

### Requirement: Report order
Reports split across the program's writes SHALL be read as if written at once. Reports, prompt starts and resets in the same output SHALL apply in the order they were written.

#### Scenario: Split report
- **WHEN** the program writes the first half of a report and the rest in a later write
- **THEN** the pane keeps the record as if the report were written at once

### Requirement: Record limit
A pane SHALL keep at most 64 status records. A report that would add a 65th SHALL first evict the record updated longest ago.

#### Scenario: Sixty-fifth id
- **WHEN** a program reports 65 distinct ids in order
- **THEN** the pane keeps the last 64 and not the first

### Requirement: Clean text
A report SHALL be dropped when its decoded `title` or `msg` holds a control character, when its `app` falls outside the protocol's character set, or when a field exceeds the protocol's limits: `app` 32 bytes, `title` 192, `msg` 2048 and `id` 128. Bidirectional formatting characters SHALL be removed from `title` and `msg` before a record is kept.

#### Scenario: Escapes in a report
- **WHEN** a program reports with an `app` containing an escape sequence, or a base64 `msg` that decodes to text containing ESC
- **THEN** the pane's records are unchanged

#### Scenario: Bidirectional override
- **WHEN** a program reports a `msg` containing U+202E
- **THEN** the kept `msg` doesn't contain it

### Requirement: Record lifetimes
A pane's records SHALL all be removed when its program exits and when the terminal is fully reset (RIS, `ESC c`). Working and blocked records SHALL be removed when the shell starts a new prompt through OSC 133 `A`. Done, error and idle records SHALL survive a prompt.

#### Scenario: Killed agent
- **WHEN** an agent reporting `working` runs inside a shell that emits OSC 133 prompt marks and is killed with `kill -9`
- **THEN** the pane has no working record once the shell prints its next prompt

#### Scenario: Finished agent
- **WHEN** an agent reports `done` and exits back to a shell with prompt marks
- **THEN** the pane keeps its done record

#### Scenario: Pane program exits
- **WHEN** a pane whose program reported `working` exits
- **THEN** `ship pane get` shows the pane exited with no records

### Requirement: Records in the pane data
A pane's status records SHALL be its `program` field in `ship pane get`, in `ship tab list` and in the state every attached client receives: an object keyed by record id, with the root record under `""`, listed oldest update first. Each record SHALL hold `state` and whichever of `kind`, `progress`, `app`, `title` and `msg` were reported. A pane with no records SHALL omit the field.

#### Scenario: Root record
- **WHEN** a pane's program reports `working` with `app=deploy` and a `msg` of `Deploying v2.4.1`
- **THEN** `ship pane get` shows `"program": { "": { "state": "working", "app": "deploy", "msg": "Deploying v2.4.1" } }`

#### Scenario: Plain shell
- **WHEN** a pane runs a shell that has reported nothing
- **THEN** `ship pane get` shows no `program` field
