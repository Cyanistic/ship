## ADDED Requirements

### Requirement: Attach feed
`POST /api/v0/attach` SHALL accept `"feed": "latest"` or `"feed": "every"`, defaulting to `latest`. A `latest` stream SHALL send only the newest replica, so it may skip revisions in between. An `every` stream SHALL send each replica newer than its `attached` seed, once and in revision order. Screen and clipboard events SHALL NOT depend on the feed.

#### Scenario: Every revision in order
- **WHEN** a caller attaches with `{"feed": "every"}` and a script renames a pane 20 times
- **THEN** the caller receives 20 `state` events with consecutive revisions

#### Scenario: Default feed
- **WHEN** a caller attaches with `{}`
- **THEN** it receives the newest state after each change and may skip revisions in between, as before

### Requirement: Falling behind an every feed
The server SHALL keep at least the last 256 revisions for `every` streams. A stream that falls further behind SHALL end with `{"type": "ended", "reason": "lost"}` instead of skipping revisions, and its attachment SHALL detach.

#### Scenario: Stalled reader
- **WHEN** an `every` stream's reader stops reading while more revisions than the server keeps are published, and then resumes
- **THEN** it receives the revisions still kept and then `ended` with reason `lost`, and `ship view list` no longer shows the attachment

## MODIFIED Requirements

### Requirement: Observer attachments
`POST /api/v0/attach` SHALL accept a request without an `area`. An attachment without an area SHALL be left out of every tab's size, as one without a selection is, and its viewing record SHALL omit `area`. An **observer** is an attachment with neither an area nor a selection: it SHALL receive the `attached` replica and later `state` replicas as its feed sends them, and no `screen` events. A later `PUT /api/v0/attach/view` SHALL give the attachment an area like any other.

#### Scenario: Observer leaves geometry alone
- **WHEN** a full-screen client views a tab and a caller attaches with the body `{}`
- **THEN** the tab's published `geometry` is unchanged, the caller receives `attached` and `state` events and no `screen` events, and `ship view list` shows its record without an area

#### Scenario: Observer on the every feed
- **WHEN** a caller attaches with `{"feed": "every"}`
- **THEN** it receives a `state` event for each later revision and no `screen` events

#### Scenario: Observer detaches
- **WHEN** the observer's stream is dropped
- **THEN** its record leaves `ship view list` and the full-screen client keeps drawing as before
