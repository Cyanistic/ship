//! The user's terminal while the client runs: raw mode, the alternate screen
//! and bracketed paste, restored on every exit path the process survives.

use std::io::{self, Stdout};

use crossterm::{
    cursor::{SetCursorStyle, Show},
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use ship_core::{
    prelude::*,
    screen::{Cursor, CursorShape},
};

/// Restores the terminal on drop. `enter` also installs a panic hook that
/// restores it before the panic message prints.
pub(super) struct TerminalGuard {
    pub terminal: Terminal<CrosstermBackend<Stdout>>,
    /// The cursor shape last set, so it's only written when it changes.
    cursor: Option<(CursorShape, bool)>,
}

impl TerminalGuard {
    pub fn enter() -> Result<Self> {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
        enable_raw_mode().map_err(|error| err!(Io, "cannot enter raw mode", @external: error))?;
        // From here on, the guard restores whatever succeeded.
        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))
            .map_err(|error| err!(Io, "cannot open the terminal", @external: error));
        let guard = Self {
            terminal: terminal.inspect_err(|_| restore())?,
            cursor: None,
        };
        execute!(io::stdout(), EnterAlternateScreen, EnableBracketedPaste)
            .map_err(|error| err!(Io, "cannot enter the alternate screen", @external: error))?;
        Ok(guard)
    }

    /// Match the pane's cursor shape. ratatui places and shows the cursor but
    /// leaves its shape alone.
    pub fn set_cursor(&mut self, cursor: Option<Cursor>) -> Result<()> {
        let Some(cursor) = cursor else {
            return Ok(());
        };
        let shape = (cursor.shape, cursor.blinking);
        if self.cursor == Some(shape) {
            return Ok(());
        }
        self.cursor = Some(shape);
        let style = match shape {
            (CursorShape::Block, true) => SetCursorStyle::BlinkingBlock,
            (CursorShape::Block, false) => SetCursorStyle::SteadyBlock,
            (CursorShape::Underline, true) => SetCursorStyle::BlinkingUnderScore,
            (CursorShape::Underline, false) => SetCursorStyle::SteadyUnderScore,
            (CursorShape::Bar, true) => SetCursorStyle::BlinkingBar,
            (CursorShape::Bar, false) => SetCursorStyle::SteadyBar,
        };
        execute!(io::stdout(), style)
            .map_err(|error| err!(Io, "cannot set the cursor shape", @external: error))
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        restore();
    }
}

/// Undo `enter`, ignoring failures: a closed terminal can't be restored.
fn restore() {
    execute!(
        io::stdout(),
        DisableBracketedPaste,
        LeaveAlternateScreen,
        SetCursorStyle::DefaultUserShape,
        Show
    )
    .ok();
    disable_raw_mode().ok();
}
