# Spec Delta

## ADDED Requirements

### Requirement: Menus in the config
The client config SHALL accept `[client.menus]` with the targets `pane`, `tab` and `global`. Each target SHALL be an ordered list of items. Each item SHALL be a table with exactly one action under `action`, written as in a key binding, and an optional `label`. An item without `label` SHALL be labeled the way a key binding's hint is derived from its action. An unknown target, an unknown key in an item, or an item without `action` SHALL be a config error with its line and column. Ship's defaults SHALL define all three targets. A target in the user's file SHALL replace that target's default list whole, and targets the file doesn't name SHALL keep their defaults. An empty list SHALL mean that target has no menu. Menus SHALL reload with the rest of `[client]`.

#### Scenario: Replace one target
- **WHEN** the file sets `[client.menus] pane = [{ action = { server.pane.zoom = {} }, label = "fill" }]` and the user right-clicks a pane
- **THEN** the pane menu shows only `fill`, and the tab and global menus keep their default items

#### Scenario: Label from the action
- **WHEN** the file sets `pane = [{ action = { server.pane.zoom = {} } }]`
- **THEN** the item's label is the hint a key bound to that action would show

#### Scenario: Unknown target or key
- **WHEN** a file sets `[client.menus] pain = []`, or an item with `lable = "x"`, and the user runs `ship config check`
- **THEN** it reports the unknown key with its line and column and exits 1

### Requirement: Default menus
With no `[client.menus]` in the file, the menus SHALL be, in order:

| Target | Label | Action |
| --- | --- | --- |
| pane | split right | `server.pane.create = { direction = "right", select = true }` |
| pane | split down | `server.pane.create = { direction = "down", select = true }` |
| pane | zoom | `server.pane.zoom = {}` |
| pane | right-click passthrough | `server.pane.set = { right_click_passthrough = "toggle" }` |
| pane | close | `server.pane.close = {}` |
| tab | new tab | `server.tab.create = { starter = "shell", select = true }` |
| tab | new child tab | `server.tab.create = { starter = "shell", child = true, select = true }` |
| tab | close | `server.tab.close = {}` |
| global | new tab | `server.tab.create = { starter = "shell", select = true }` |
| global | hide sidebar | `client.sidebar.toggle = {}` |
| global | reload config | `client.config.reload = {}` |
| global | detach | `client.detach = {}` |

#### Scenario: Default pane menu
- **WHEN** a user with no config right-clicks a pane
- **THEN** the menu shows split right, split down, zoom, right-click passthrough and close, in that order

#### Scenario: New tab beside a row
- **WHEN** the tab menu is open on tab B's row and the user picks `new tab`
- **THEN** a tab with a shell is created right after B under B's parent and its pane is selected, and picking `new child tab` instead appends it as B's last child

### Requirement: Opening a menu
With mouse capture on, a right-click SHALL open:
- the pane menu on a pane's contents (subject to mouse-input's Right-click passthrough), its label, or its border;
- the tab menu on a sidebar row, including its fold marker;
- the global menu on the sidebar's `tabs` heading or on sidebar space below the last row.

On a border cell, the pane SHALL be the one whose label is there. Otherwise it SHALL be the pane below a horizontal edge or right of a vertical edge, the pane below and to the right at a junction, and the pane inside on the tab's outer edges. The sidebar heading SHALL show a button at its right end, and a left click on it SHALL open the global menu. A right-click that opens the pane or tab menu SHALL first select the clicked pane or tab for that client, as a left click on it does, taking its turn in input order. Opening the global menu SHALL NOT change selection. When the target's list is empty, a right-click SHALL act as it did before menus existed, and SHALL NOT select anything it wouldn't have selected then. With `[client] mouse = false`, no menu SHALL open.

#### Scenario: Right-click an unselected pane
- **WHEN** a user right-clicks in an unselected shell pane
- **THEN** that pane is selected and the pane menu opens at the pointer

#### Scenario: Right-click a border
- **WHEN** a user right-clicks the vertical border between a left and a right pane, below the right pane's label
- **THEN** the pane menu opens for the right pane, and at a 2x2 junction it opens for the bottom-right pane

#### Scenario: Global menu
- **WHEN** a user left-clicks the heading button, or right-clicks the `tabs` heading or the empty space below the last row
- **THEN** the global menu opens and the selection doesn't change

#### Scenario: Empty target
- **WHEN** the file sets `pane = []` and a user right-clicks a pane
- **THEN** no menu opens and the right-click acts as it did before menus existed

### Requirement: Using an open menu
A menu SHALL open at the pointer and be moved as needed to fit fully on the client's screen. Its items SHALL appear in config order with the first highlighted. A menu SHALL belong to the client that opened it, and other clients see nothing of it. At most one menu SHALL be open. While a menu is open:
- a left click on an item SHALL run it and close the menu;
- up and down, and `j` and `k`, SHALL move the highlight, and Enter SHALL run the highlighted item and close the menu;
- Esc SHALL close the menu without running anything;
- a press of any button outside the menu SHALL close it and SHALL NOT act anywhere else;
- a terminal resize SHALL close it;
- no key may reach a pane or run a binding.

#### Scenario: Keyboard choice
- **WHEN** a menu is open and the user presses `j` twice and Enter
- **THEN** the third item runs and the menu closes

#### Scenario: Dismiss
- **WHEN** a menu is open and the user presses Esc, or clicks a pane outside the menu
- **THEN** the menu closes, nothing runs, and the click doesn't select or reach that pane

#### Scenario: Fits the screen
- **WHEN** a user right-clicks a pane near the bottom-right corner of the terminal
- **THEN** the whole menu is drawn on screen

#### Scenario: Other clients
- **WHEN** one client has a menu open
- **THEN** a second attached client shows no menu and its keys reach its panes as usual

### Requirement: Menu items run as key actions
An item's action SHALL take its turn in the client's input order and behave exactly as the same action bound to a key, acting on the selection at its turn. Actions that a key applies at press, such as mode changes, the sidebar toggle and config reloads, SHALL apply when the item is picked. A failure SHALL show on the status line as a key action's does.

#### Scenario: Split from the menu
- **WHEN** the pane menu is open and the user clicks `split right`
- **THEN** the pane splits to the right, the new pane is selected, and the menu closes

#### Scenario: After a pending create
- **WHEN** a delayed create is still waiting, and the user right-clicks a pane and picks `zoom`
- **THEN** the clicked pane is the one zoomed

#### Scenario: Press-time item
- **WHEN** the user picks `hide sidebar` from the global menu while earlier input is waiting
- **THEN** the sidebar hides at once
