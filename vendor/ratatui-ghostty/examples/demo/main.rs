mod app;
mod keybinds;
mod layout;
mod ui;

use std::io;
use std::os::unix::io::AsRawFd;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::cursor::SetCursorStyle;
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyEventKind, MouseEventKind,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui_ghostty::colors;
use ratatui_ghostty::input::IntoMouseInput;
use ratatui_ghostty::widget::CursorStyle;

use app::App;

enum AppEvent {
    Crossterm(Event),
    PaneUpdate,
}

/// Required as ghostty zig lib logs to stderr, see readme
fn redirect_stderr() {
    if let Ok(log) = std::fs::File::create("/tmp/ratatui-ghostty.log") {
        unsafe { libc::dup2(log.as_raw_fd(), libc::STDERR_FILENO) };
    }
}

fn main() -> anyhow::Result<()> {
    redirect_stderr();
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = terminal::disable_raw_mode();
        let _ = execute!(
            io::stdout(),
            DisableMouseCapture,
            DisableBracketedPaste,
            LeaveAlternateScreen,
            cursor::Show,
            SetCursorStyle::DefaultUserShape
        );
        default_hook(info);
    }));

    terminal::enable_raw_mode()?;
    let host_colors = colors::query_colors(16, Duration::from_millis(500))
        .unwrap_or_else(|_| colors::HostColors::default());
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste,
        cursor::Hide
    )?;

    let backend = CrosstermBackend::new(stdout);
    let mut term = Terminal::new(backend)?;

    let (wake_tx, wake_rx) = mpsc::channel::<AppEvent>();

    let input_tx = wake_tx.clone();
    thread::spawn(move || {
        loop {
            match event::poll(Duration::from_secs(1)) {
                Ok(true) => {
                    if let Ok(ev) = event::read()
                        && input_tx.send(AppEvent::Crossterm(ev)).is_err()
                    {
                        break;
                    }
                }
                Ok(false) => {}
                Err(_) => break,
            }
        }
    });

    let pane_wake_tx = wake_tx.clone();
    let wake: Arc<dyn Fn() + Send + Sync> = Arc::new(move || {
        let _ = pane_wake_tx.send(AppEvent::PaneUpdate);
    });

    let pa = pane_area(term.size()?.into());
    let mut app = App::new(pa.width, pa.height, host_colors, wake)?;

    let result = run(&mut term, &mut app, wake_rx);

    terminal::disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        DisableMouseCapture,
        DisableBracketedPaste,
        LeaveAlternateScreen,
        cursor::Show,
        SetCursorStyle::DefaultUserShape
    )?;

    result
}

fn pane_area(term_size: Rect) -> Rect {
    Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(term_size)[0]
}

const FRAME_DURATION: Duration = Duration::from_millis(16);

fn run(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    wake_rx: mpsc::Receiver<AppEvent>,
) -> anyhow::Result<()> {
    let mut last_render = Instant::now()
        .checked_sub(FRAME_DURATION)
        .unwrap_or_else(Instant::now);
    let mut render_pending = false;

    while app.running {
        let event = if render_pending {
            let remaining = FRAME_DURATION.saturating_sub(last_render.elapsed());
            if remaining.is_zero() {
                None
            } else {
                match wake_rx.recv_timeout(remaining) {
                    Ok(ev) => Some(ev),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        } else {
            match wake_rx.recv() {
                Ok(ev) => Some(ev),
                Err(_) => break,
            }
        };

        let mut events = Vec::new();
        if let Some(ev) = event {
            events.push(ev);
        }
        while let Ok(ev) = wake_rx.try_recv() {
            events.push(ev);
        }

        for ev in events {
            match ev {
                AppEvent::Crossterm(ct) => handle_crossterm(app, term, ct)?,
                AppEvent::PaneUpdate => {}
            }
        }

        let any_dirty = app.drain_and_check_dirty();

        if any_dirty {
            let now = Instant::now();
            if now.duration_since(last_render) >= FRAME_DURATION {
                term.draw(|frame| ui::draw(frame, app))?;
                apply_cursor_style(term, app)?;
                last_render = now;
                render_pending = false;
            } else {
                render_pending = true;
            }
        }
    }

    Ok(())
}

fn apply_cursor_style(
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &App,
) -> anyhow::Result<()> {
    let Some(pane) = app.active_pane() else {
        return Ok(());
    };
    let cursor = pane.cursor_state();
    if cursor.position.is_none() {
        return Ok(());
    }
    let ct_style = match (cursor.style, cursor.blinking) {
        (CursorStyle::Block, false) => SetCursorStyle::SteadyBlock,
        (CursorStyle::Block, true) => SetCursorStyle::BlinkingBlock,
        (CursorStyle::Bar, false) => SetCursorStyle::SteadyBar,
        (CursorStyle::Bar, true) => SetCursorStyle::BlinkingBar,
        (CursorStyle::Underline, false) => SetCursorStyle::SteadyUnderScore,
        (CursorStyle::Underline, true) => SetCursorStyle::BlinkingUnderScore,
        (CursorStyle::Default, _) => SetCursorStyle::DefaultUserShape,
    };
    execute!(term.backend_mut(), ct_style)?;
    Ok(())
}

fn handle_crossterm(
    app: &mut App,
    term: &mut Terminal<CrosstermBackend<io::Stdout>>,
    event: Event,
) -> anyhow::Result<()> {
    match event {
        Event::Key(key) if key.kind == KeyEventKind::Press => {
            let action = app.prefix.handle_key(key);
            let pa = pane_area(term.size()?.into());
            app.handle_action(action, pa);
        }
        Event::Mouse(mouse) => {
            if let MouseEventKind::Down(_) = mouse.kind
                && let Some(idx) = app.pane_index_at(mouse.column, mouse.row)
            {
                app.set_focus(idx);
            }
            let tab = &app.tabs[app.active_tab];
            let pane_idx = match mouse.kind {
                MouseEventKind::ScrollUp | MouseEventKind::ScrollDown => {
                    app.pane_index_at(mouse.column, mouse.row)
                }
                _ => Some(tab.active),
            };
            if let Some(idx) = pane_idx {
                let tab = &app.tabs[app.active_tab];
                if let (Some(pane), Some(rect)) = (tab.panes.get(idx), tab.rects.get(idx))
                    && let Some(mut input) = mouse.into_mouse_input()
                {
                    input.col = input.col.saturating_sub(rect.x);
                    input.row = input.row.saturating_sub(rect.y);
                    pane.send_mouse(input);
                }
            }
        }
        Event::Paste(s) => {
            if let Some(pane) = app.active_pane() {
                pane.send_paste(s.into_bytes());
            }
        }
        Event::FocusGained => {
            if let Some(pane) = app.active_pane() {
                pane.send_focus(true);
            }
        }
        Event::FocusLost => {
            if let Some(pane) = app.active_pane() {
                pane.send_focus(false);
            }
        }
        Event::Resize(w, h) => {
            let pa = pane_area(Rect::new(0, 0, w, h));
            app.resize_all(pa);
        }
        _ => {}
    }
    Ok(())
}
