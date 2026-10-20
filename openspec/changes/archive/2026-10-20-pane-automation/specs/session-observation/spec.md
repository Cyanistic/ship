# Spec Delta

## ADDED Requirements

### Requirement: Observer attachments
`POST /api/v0/attach` SHALL accept a request without an `area`. An attachment without an area SHALL be left out of every tab's size, as one without a selection is, and its viewing record SHALL omit `area`. An **observer** is an attachment with neither an area nor a selection: it SHALL receive the `attached` replica and each later `state` replica, and no `screen` events. A later `PUT /api/v0/attach/view` SHALL give the attachment an area like any other.

#### Scenario: Observer leaves geometry alone
- **WHEN** a full-screen client views a tab and a caller attaches with the body `{}`
- **THEN** the tab's published `geometry` is unchanged, the caller receives `attached` and `state` events and no `screen` events, and `ship view list` shows its record without an area

#### Scenario: Observer detaches
- **WHEN** the observer's stream is dropped
- **THEN** its record leaves `ship view list` and the full-screen client keeps drawing as before

## MODIFIED Requirements

### Requirement: Views routes
`GET /api/v0/views` SHALL return every attachment's viewing record as a JSON object keyed by attachment ID, in attach order, with the same records the observation stream publishes under `viewers`. `GET /api/v0/views/{attachment}` SHALL return one record. `PUT /api/v0/views/{attachment}/selection` with `{"selection": <tab or pane ID>}` or `{"selection": null}` SHALL set that attachment's selection and return the record as stored. An unknown attachment or node SHALL get 404 and change nothing.

#### Scenario: Read every view
- **WHEN** two clients are attached and a caller sends `GET /api/v0/views`
- **THEN** it receives an object with both attachment IDs, each holding that client's selection and area, when it has them

#### Scenario: Read an observer
- **WHEN** a full-screen client and an observer are attached and a caller sends `GET /api/v0/views`
- **THEN** the observer's record holds neither a selection nor an area

#### Scenario: Select nothing
- **WHEN** a caller sends `PUT /api/v0/views/<id>/selection` with `{"selection": null}`
- **THEN** it receives the record without a selection, and that client shows the nothing-selected hint

#### Scenario: Unknown node
- **WHEN** a caller selects a pane ID that doesn't exist for an attached client
- **THEN** it receives 404 and the client's selection is unchanged

#### Scenario: Unknown attachment
- **WHEN** a caller sends `GET /api/v0/views/<id>` for an attachment that has ended
- **THEN** it receives 404
