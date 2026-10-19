# Spec Delta

## MODIFIED Requirements

### Requirement: Tab sidebar presentation
The sidebar's tabs section SHALL show a plain `tabs` title above one row per visible tab in tree order, indented by depth and using derived labels. Tabs with children SHALL have expanded/folded markers. It SHALL highlight the selected tab or the selected pane's tab, and no row when nothing is selected. A zoomed tab's row SHALL carry a zoom marker; while the sidebar is hidden, no zoom marker is shown. A tab row SHALL end with its rollup icon as agent-attention's Tab rollup describes. `client.sidebar.toggle` SHALL hide/show it per client while preserving its folds, width and split and giving freed width to panes. The `tabs` heading SHALL end with a button that opens the global menu (see context-menus). While a row is dragged, the drop line SHALL be an underline on the cells of the row above the gap, from the target depth's indent to the divider; the gap above the first row SHALL use the blank row under the heading.

#### Scenario: Nested rows
- **WHEN** a client opens a three-level tab tree and folds an intermediate row
- **THEN** the sidebar retains that row with a folded marker and hides its descendants without hiding their programs

#### Scenario: Toggle preserves presentation
- **WHEN** a client hides its folded sidebar and shows it again
- **THEN** the same folds return and another client's sidebar visibility is unchanged

#### Scenario: Zoom marker
- **WHEN** the selected tab is zoomed
- **THEN** its sidebar row carries a zoom marker, without requiring a pane border

#### Scenario: Empty selection beside sidebar
- **WHEN** nothing is selected or an empty tab is selected
- **THEN** the existing corresponding empty-state hint appears in the pane area beside the sidebar

#### Scenario: Heading button
- **WHEN** a client shows its sidebar
- **THEN** the `tabs` heading ends with a button, and the rows below it are unchanged

#### Scenario: Drop line
- **WHEN** a user drags a row over the gap below a top-level tab with the pointer one indent further right
- **THEN** the row above the gap is underlined from that indent to the divider, and no row moves until release

#### Scenario: Rollup icon
- **WHEN** a pane in a tab's child tab is `working`
- **THEN** both tabs' rows end with the working icon

## ADDED Requirements

### Requirement: Sidebar sections
Whenever the sidebar is shown, it SHALL hold the tabs section on top and the agents section (see agent-attention) below, with a divider row between them. Each section SHALL scroll on its own when its content overflows, and the tabs section SHALL keep the selected row visible as it does today.

#### Scenario: No agents
- **WHEN** a client shows the sidebar with no pane reporting
- **THEN** the agents section below the divider row shows `no agents`

#### Scenario: Overflow
- **WHEN** a short terminal holds 30 tabs and 20 reporting panes
- **THEN** each section scrolls with the wheel without moving the other

### Requirement: Sidebar split
The split between the sidebar's sections SHALL default to half the sidebar's height. A client MAY move it by dragging the divider row or with `client.sidebar.resize` up or down. The split SHALL stay between 10% and 90% of the height with at least 3 rows for each section, SHALL apply only to that client, and SHALL last until the client exits.

#### Scenario: Default split
- **WHEN** a client shows the sidebar
- **THEN** the tabs section fills the top half and the agents section the bottom half

#### Scenario: Another client
- **WHEN** one client drags its split down
- **THEN** a second client's split is unchanged
