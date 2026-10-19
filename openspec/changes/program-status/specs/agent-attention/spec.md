# Spec Delta

## Purpose

Show each client which agents need the user, and tell them when one finishes or asks for something, from the status records panes carry (see program-status). A **display state** is what a client shows for a pane with a root record: `blocked`, `error`, `working`, `unseen` or `idle`. Everything here is client-local: each client derives, marks and notifies on its own.

## ADDED Requirements

### Requirement: Display states
A client SHALL derive a display state for each pane with a root record. A root state of `blocked`, `error` or `working` SHALL display as itself. A root state of `idle` or `done` SHALL display as `unseen` while the client has marked the pane unseen, and as `idle` otherwise. A reported `done` alone SHALL NOT make a pane unseen. A pane without a root record SHALL have no display state, whatever child records it has.

#### Scenario: Done seen elsewhere
- **WHEN** a client attaches to a pane whose root record is already `done`
- **THEN** the pane displays `idle`

#### Scenario: Child records only
- **WHEN** a pane has a record under `build` and none under `""`
- **THEN** the pane has no row in the agents section and its tab gets no icon from it

### Requirement: Unseen marks
A pane SHALL become unseen when a done notification fires for it. It SHALL stop being unseen once it is in view while the client's window has focus. A pane is **in view** when the client draws it in the current tab, which while the tab is zoomed means only the zoomed pane. Until the outer terminal reports focus, the window SHALL count as focused. A client attaching or reattaching SHALL start with no unseen panes.

#### Scenario: Finished in another tab
- **WHEN** an agent in a tab the user isn't viewing goes from `working` to `idle` and stays there past the delay
- **THEN** its row and tab show unseen, and the mark clears once the user views the pane with the window focused

#### Scenario: In view on another monitor
- **WHEN** an agent's pane is in view, the client's window is unfocused, and the agent finishes
- **THEN** the pane is marked unseen until the window regains focus with the pane in view

#### Scenario: Attaching to finished agents
- **WHEN** five agents finished while no client was attached and a client attaches
- **THEN** none is marked unseen and nothing plays

### Requirement: Icons
Each display state SHALL have an icon distinct in both shape and color: blocked `▲` red, error `✖` magenta, unseen `◆` cyan, working `●` yellow and idle `○` dark gray. Agent rows and tab rows SHALL use the same icons, and an agent row's `state` token SHALL take its icon's color.

#### Scenario: Blocked agent
- **WHEN** an agent's root record is `blocked`
- **THEN** its row's `icon` token and its tab show a red `▲`

### Requirement: Tab rollup
A tab row SHALL show the icon of the highest-ranked display state among its own panes and all its descendant tabs, ranking `blocked` and `error` first, then `unseen`, `working` and `idle`. A tab with no display states beneath it SHALL show no icon.

#### Scenario: Blocked inside a folded tab
- **WHEN** a pane two levels down in a folded tab is `blocked` and a sibling pane is `working`
- **THEN** the folded tab's row shows the blocked icon

### Requirement: Agent list
The agents section SHALL list one entry per pane with a display state, ordered by rank as in Tab rollup and by tree order, depth-first through the tabs, within each rank. With no such pane it SHALL show a `no agents` placeholder.

#### Scenario: Order
- **WHEN** one pane is `working`, a later pane in tree order is `blocked` and another is `idle`
- **THEN** the list shows the blocked pane, then the working one, then the idle one

### Requirement: Agent rows
Each entry SHALL be drawn from the configured lines of tokens (see config-file's Agent settings), with the tokens of a line separated by spaces. A token with no value SHALL be left out, and a line left with no tokens SHALL be left out.

#### Scenario: Default rows
- **WHEN** Claude Code is blocked in tab `api` under top-level tab `ship`, with the default rows
- **THEN** its entry shows `▲ claude blocked` above `ship › api`

### Requirement: Row tokens
`icon` SHALL be the display state's icon and `state` its name. `app` SHALL be the root record's `app`, else the pane's label as the sidebar and pane borders show it. `tab` SHALL be the path of tab labels from the top level joined by ` › `, shortened from the left when it doesn't fit. `pane` SHALL be the pane's label, and `title` and `msg` the root record's.

#### Scenario: Missing app and title
- **WHEN** the rows are `[["icon", "app"], ["title"]]` and the record has neither `app` nor `title`
- **THEN** the entry is one line holding the icon and the pane's label

### Requirement: Jump to what needs the user
`client.agents.next` SHALL go through the panes that need the user, `blocked` and `error` ones first and then `unseen` ones, in agent list order. It SHALL select the first of them after the selected pane in that order, wrapping to the first when the selected pane is the last or isn't one of them. With none, it SHALL do nothing. It SHALL take its turn in input order like other selection-changing actions.

#### Scenario: Blocked, then unseen
- **WHEN** one agent is blocked in another tab, another finished unseen, and the user presses `alt-a` twice
- **THEN** the first press selects the blocked pane and the second selects the unseen one

#### Scenario: Nothing needs the user
- **WHEN** every agent is working or idle and the user presses `alt-a`
- **THEN** the selection doesn't change

### Requirement: Notifications
A pane whose root state becomes `blocked` or `error` SHALL trigger an attention notification. A pane whose root state goes from `working` to `idle` or `done` SHALL trigger a done notification. A notification SHALL fire only if the new state still holds once the delay has passed since the change, and SHALL NOT fire if the pane is in view in a focused window at that moment.

#### Scenario: Quick flip
- **WHEN** an agent goes from `working` to `idle` and back to `working` within the delay
- **THEN** nothing plays and nothing is marked unseen

#### Scenario: In view and focused
- **WHEN** the agent's pane is in view in a focused window and the agent asks for approval
- **THEN** nothing plays and no popup appears

### Requirement: Notification sounds
A fired notification SHALL play its sound on the client's machine: the setting for the record's `app` when one is configured, else the default setting, else the bundled sound for that notification. The bundled done and attention sounds SHALL differ. A sound set to `off` SHALL play nothing.

#### Scenario: App override
- **WHEN** the config sets `done = "off"` for app `droid` and both Droid and Claude finish unseen
- **THEN** only Claude's finish plays a sound

### Requirement: Sound failures
A sound file that is missing or can't be decoded SHALL be replaced by the bundled sound for its notification when the config loads, with a warning naming the file in the client log. A missing or failing audio device SHALL NOT affect terminal work.

#### Scenario: Missing file
- **WHEN** the default `done` sound is a path that doesn't exist and an agent finishes unseen
- **THEN** the bundled done sound plays and the client log names the path

#### Scenario: No audio device
- **WHEN** a client starts on a machine with no audio output
- **THEN** it attaches and works normally, and logs the problem once

### Requirement: Popups
When popups are on, each fired notification SHALL also be written to the outer terminal as one OSC 9 notification: `<app> needs you · <tab path>` for attention or `<app> finished · <tab path>` for done, followed by `: <msg>` when the record has one, where `<app>` and `<tab path>` are as in Agent rows. Control characters SHALL be removed from the line before it is written. Popups SHALL be off by default.

#### Scenario: Attention popup
- **WHEN** popups are on and Claude Code, in tab `api` under `ship`, asks `Apply these changes?` while the user is in another tab
- **THEN** the outer terminal shows `claude needs you · ship › api: Apply these changes?`

#### Scenario: Title with escapes
- **WHEN** popups are on and a pane label taken from a window title holds an escape sequence
- **THEN** the popup carries the label without it and the outer terminal's state is unchanged
