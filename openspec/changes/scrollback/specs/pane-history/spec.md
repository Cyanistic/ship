# Spec Delta

## Purpose

Let users look back through what a pane printed, with one scroll position shared by every viewer, and let agents and scripts read a pane's text from the CLI.

## ADDED Requirements

### Requirement: Shared scroll position
Each pane SHALL have one scroll position shared by every client viewing it. Scrolling a pane from any client SHALL change what every viewer of that pane sees.

#### Scenario: Two clients on one pane
- **WHEN** two clients view the same pane and one scrolls it up
- **THEN** both show the same history lines

### Requirement: Scroll a pane
`server.pane.scroll` and `ship pane scroll` SHALL scroll the selected or targeted pane. `--direction up|down` with `--amount N` SHALL move N lines; without `--amount` they SHALL move one page, the pane's height. `--direction top|bottom` SHALL go to the oldest history or the bottom and ignore the amount. The pane SHALL default like other pane commands.

#### Scenario: Scroll to the top from the CLI
- **WHEN** a pane has printed `seq 50000` and a user runs `ship pane scroll --direction top`
- **THEN** the pane shows the oldest line its history still holds

#### Scenario: Fixed number of lines
- **WHEN** a binding invokes `server.pane.scroll = { direction = "up", amount = 5 }`
- **THEN** the selected pane moves up five lines

#### Scenario: Misspelled argument
- **WHEN** a binding writes `server.pane.scroll = { direciton = "up" }`
- **THEN** the config is rejected with the unknown argument named

### Requirement: Scrolled view stays put
A scrolled pane SHALL stay on the same content while new output arrives. When that content leaves the history, the pane SHALL show the oldest content that remains.

#### Scenario: Output while scrolled
- **WHEN** a pane is scrolled up and its program prints 1,000 more lines
- **THEN** the pane keeps showing the same lines it showed before

### Requirement: Input returns to the bottom
Input reaching a scrolled pane, from keys, a paste or `client.send` on any client, SHALL return the pane to the bottom before the program receives it.

#### Scenario: Typing into a scrolled shell
- **WHEN** a shell pane is scrolled up and a user types `ls`
- **THEN** the pane returns to the bottom and the shell receives `ls`

### Requirement: History size
Each pane's history SHALL be limited to `[server] scrollback_bytes` bytes as read when the pane starts, with no line limit.

#### Scenario: Default history keeps a long log
- **WHEN** a 200-column pane on the default setting prints `seq 50000` and is scrolled to the top
- **THEN** it shows line 1

### Requirement: Alternate screen has no history
A pane on the alternate screen SHALL have no history to scroll, and scroll actions on it SHALL do nothing.

#### Scenario: Scroll a full-screen program
- **WHEN** a pane runs Neovim and a user runs `ship pane scroll --direction up`
- **THEN** Neovim's screen is unchanged

### Requirement: Exited panes keep their history
A pane whose program has exited SHALL scroll and be readable like a running one.

#### Scenario: Scroll after exit
- **WHEN** a shell prints a long log and exits, and a user runs `ship pane scroll --direction top` on its pane
- **THEN** the pane shows the start of its history

### Requirement: Read a pane
`ship pane read` SHALL print a pane's text: the visible screen by default, or with `--lines N` the last N lines including history. `--unwrap` SHALL join soft-wrapped lines and `--ansi` SHALL keep colors and styles as escape sequences. The pane SHALL default like other pane commands. It SHALL have no key binding form.

#### Scenario: Last lines while scrolled
- **WHEN** a pane printed 200 numbered lines and is scrolled up, and a user runs `ship pane read --lines 50`
- **THEN** it prints lines 151 to 200

#### Scenario: Unwrapped line
- **WHEN** an 80-column pane printed one 300-character line and a user runs `ship pane read --unwrap`
- **THEN** the line is printed whole

#### Scenario: Colors kept
- **WHEN** a pane shows `ls --color` output and a user runs `ship pane read --ansi`
- **THEN** the output keeps the color escape sequences

### Requirement: Reads ignore scroll and follow the active screen
`ship pane read` SHALL read from the bottom regardless of the pane's scroll position. On the alternate screen it SHALL read that screen only. It SHALL work on exited panes.

#### Scenario: Reading a full-screen program
- **WHEN** a pane runs Neovim and a user runs `ship pane read --lines 500`
- **THEN** it prints only Neovim's screen

#### Scenario: Agent reads a sibling
- **WHEN** an agent pane starts `cargo test` in a sibling pane and runs `ship pane read --pane <id> --lines 50`
- **THEN** it gets the end of the test output without user help
