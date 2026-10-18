# Spec Delta

## MODIFIED Requirements

### Requirement: Sidebar clicks
A click on a tab row SHALL select that tab as the keyboard does. A click on a row's fold marker SHALL fold or unfold it. A click on an agent's entry SHALL select its pane. All three SHALL act on release, not on press, and only when the press didn't become a drag, taking their turn in input order at release. The wheel over a sidebar section SHALL scroll that section and nothing else.

#### Scenario: Click a row and a fold marker
- **WHEN** a user clicks a tab's row, then clicks the fold marker of a row with children
- **THEN** the first tab is selected as navigation keys would select it, and the second row folds

#### Scenario: Press that moves
- **WHEN** a user presses a row, moves the pointer a cell and releases over the same row
- **THEN** the press is a drag, so the tab isn't selected and no fold changes

#### Scenario: Click an agent
- **WHEN** a user clicks any line of an agent's entry for a pane in another tab
- **THEN** that pane is selected

#### Scenario: Wheel over the agents section
- **WHEN** the agents section overflows and a user turns the wheel over it
- **THEN** the agents section scrolls, and the tabs section and the selected pane don't

## ADDED Requirements

### Requirement: Sidebar split drag
A drag on the divider row between the sidebar's sections SHALL move the split to follow the pointer, within terminal-client's Sidebar split limits.

#### Scenario: Drag past the limit
- **WHEN** a user drags the divider row to the bottom of the sidebar
- **THEN** the split stops at 90% of the height, or higher when that would leave the agents section fewer than 3 rows
