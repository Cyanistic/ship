//! The user's terminal while the client runs: raw mode, the alternate screen,
//! bracketed paste and focus reporting, restored on every exit path the
//! process survives.

use std::{
    io::{self, BufWriter, Stdout},
    panic,
};

use crossterm::{
    clipboard::CopyToClipboard,
    cursor::{SetCursorStyle, Show},
    event::{
        DisableBracketedPaste, DisableFocusChange, DisableMouseCapture, EnableBracketedPaste,
        EnableFocusChange, EnableMouseCapture,
    },
    execute,
    style::Print,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, TerminalOptions, Viewport, backend::CrosstermBackend, layout::Rect};
use ship_core::{
    prelude::*,
    screen::{Cursor, CursorShape},
};

/// Room for a full-screen frame of a large terminal.
const FRAME_BUFFER: usize = 256 * 1024;

/// Restores the terminal on drop. `enter` also installs a panic hook that
/// restores it before the panic message prints.
pub(super) struct TerminalGuard {
    /// Buffered so a frame reaches the terminal in one write when ratatui
    /// flushes; `Stdout` alone flushes about every kilobyte.
    pub terminal: Terminal<CrosstermBackend<BufWriter<Stdout>>>,
    /// The cursor shape last set, so it's only written when it changes.
    cursor: Option<(CursorShape, bool)>,
    /// Whether mouse capture is on.
    mouse: bool,
}

impl TerminalGuard {
    pub fn enter() -> Result<Self> {
        let previous = panic::take_hook();
        panic::set_hook(Box::new(move |info| {
            restore();
            previous(info);
        }));
        enable_raw_mode().map_err(|error| err!(Io, "cannot enter raw mode", @external: error))?;
        // From here on, the guard restores whatever succeeded.
        // Fixed, so drawing doesn't reopen `/dev/tty` for the size every
        // frame; `drive` sizes it from resize events.
        let terminal = Terminal::with_options(
            CrosstermBackend::new(BufWriter::with_capacity(FRAME_BUFFER, io::stdout())),
            TerminalOptions {
                viewport: Viewport::Fixed(Rect::default()),
            },
        )
        .map_err(|error| err!(Io, "cannot open the terminal", @external: error));
        let guard = Self {
            terminal: terminal.inspect_err(|_| restore())?,
            cursor: None,
            mouse: false,
        };
        execute!(
            io::stdout(),
            EnterAlternateScreen,
            EnableBracketedPaste,
            EnableFocusChange
        )
        .map_err(|error| err!(Io, "cannot enter the alternate screen", @external: error))?;
        Ok(guard)
    }

    /// Draw at `cols` by `rows` from now on.
    pub fn resize(&mut self, (cols, rows): (u16, u16)) -> Result<()> {
        self.terminal
            .resize(Rect::new(0, 0, cols, rows))
            .map_err(|error| err!(Io, "cannot resize the terminal", @external: error))
    }

    pub fn mouse(&self) -> bool {
        self.mouse
    }

    pub fn set_mouse(&mut self, enabled: bool) -> Result<()> {
        if enabled {
            execute!(io::stdout(), EnableMouseCapture)
        } else {
            execute!(io::stdout(), DisableMouseCapture)
        }
        .map_err(|error| err!(Io, "cannot set mouse capture", @external: error))?;
        self.mouse = enabled;
        Ok(())
    }

    /// Put `text` on the clipboard through the terminal's OSC 52, which some
    /// terminals ignore or cap in size.
    pub fn copy(&self, text: &str) -> Result<()> {
        execute!(io::stdout(), CopyToClipboard::to_clipboard_from(text))
            .map_err(|error| err!(Io, "cannot write the clipboard", @external: error))
    }

    /// One OSC 9 notification to the outer terminal, with control
    /// characters removed so labels from window titles can't escape it.
    pub fn notify(&self, text: &str) -> Result<()> {
        let text: String = text.chars().filter(|c| !c.is_control()).collect();
        execute!(io::stdout(), Print(format!("\x1b]9;{text}\x1b\\")))
            .map_err(|error| err!(Io, "cannot write a notification", @external: error))
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
            (CursorShape::Default, _) => SetCursorStyle::DefaultUserShape,
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
        DisableFocusChange,
        DisableMouseCapture,
        DisableBracketedPaste,
        LeaveAlternateScreen,
        SetCursorStyle::DefaultUserShape,
        Show
    )
    .ok();
    disable_raw_mode().ok();
}
