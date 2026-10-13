# Spec Delta

## ADDED Requirements

### Requirement: Who changes a selection
The server SHALL hold each attachment's selection. It SHALL change only through its own client's navigation and clicks, a select naming that attachment, a create that selects for it, and Selection repair. A client SHALL show a selection set by someone else as it shows its own, within a frame of the server publishing it.

#### Scenario: A script steers a client
- **WHEN** a client is attached and a second terminal runs `ship view select --attachment <id> <pane>`
- **THEN** the client shows `<pane>` selected on its next frame, and typing goes to it

#### Scenario: Another client's create
- **WHEN** one client presses `alt-n` while a second client has a pane selected
- **THEN** the second client's selection doesn't move

### Requirement: Views routes
`GET /api/v0/views` SHALL return every attachment's viewing record as a JSON object keyed by attachment ID, in attach order, with the same records the observation stream publishes under `viewers`. `GET /api/v0/views/{attachment}` SHALL return one record. `PUT /api/v0/views/{attachment}/selection` with `{"selection": <tab or pane ID>}` or `{"selection": null}` SHALL set that attachment's selection and return the record as stored. An unknown attachment or node SHALL get 404 and change nothing.

#### Scenario: Read every view
- **WHEN** two clients are attached and a caller sends `GET /api/v0/views`
- **THEN** it receives an object with both attachment IDs, each holding that client's selection, when it has one, and area

#### Scenario: Select nothing
- **WHEN** a caller sends `PUT /api/v0/views/<id>/selection` with `{"selection": null}`
- **THEN** it receives the record without a selection, and that client shows the nothing-selected hint

#### Scenario: Unknown node
- **WHEN** a caller selects a pane ID that doesn't exist for an attached client
- **THEN** it receives 404 and the client's selection is unchanged

#### Scenario: Unknown attachment
- **WHEN** a caller sends `GET /api/v0/views/<id>` for an attachment that has ended
- **THEN** it receives 404

### Requirement: View commands
`ship view list` SHALL print every attachment's view as JSON keyed by attachment ID. `ship view selected --attachment ID` SHALL print that attachment's selection as a JSON string, or `null`. `ship view select --attachment ID NODE` SHALL select a tab or pane for it, and `--none` instead of `NODE` SHALL select nothing. `--attachment` SHALL default to `SHIP_ATTACHMENT_ID`.

#### Scenario: Find and steer a client
- **WHEN** a user runs `ship view list`, takes an attachment ID from it, and runs `ship view select --attachment <id> <tab>`, then `ship view selected --attachment <id>`
- **THEN** that client has the tab itself selected, and the last command prints the tab's ID

#### Scenario: Neither a node nor --none
- **WHEN** a user runs `ship view select --attachment <id>` with no node and no `--none`
- **THEN** the CLI rejects the arguments before any request, exits 2, and the selection is unchanged

#### Scenario: Unknown attachment or node
- **WHEN** a user runs `ship view select` naming an attachment or a node that doesn't exist
- **THEN** the command fails with a not-found error, exits 1 and nothing changes

### Requirement: Select behind a zoom
Selecting a pane hidden by its tab's zoom, through the views route or `ship view select`, SHALL unzoom that tab for every viewer and select the pane, in one edit.

#### Scenario: Script selects a hidden pane
- **WHEN** A is zoomed in a tab with B, two clients view the tab, and a script selects B for one of them
- **THEN** both clients see the tab unzoomed, and the script's client has B selected
