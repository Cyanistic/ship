use std::io;
use std::os::unix::io::AsRawFd;
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use crossterm::cursor::SetCursorStyle;
use crossterm::event::{
    self, DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture,
    Event, KeyCode, KeyEventKind, KeyModifiers, MouseEventKind,
};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::{cursor, execute};
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Rect;
use ratatui::widgets::{Block, Borders, Widget};
use ratatui_ghostty::colors;
use ratatui_ghostty::input::IntoMouseInput;
use ratatui_ghostty::session::{SessionConfig, SessionEvent, SessionHandle, SessionIo};
use ratatui_ghostty::widget::CursorStyle;

enum AppEvent {
    Crossterm(Event),
    SessionUpdate,
}

fn redirect_stderr() {
    if let Ok(log) = std::fs::File::create("/tmp/ratatui-ghostty-example.log") {
        unsafe { libc::dup2(log.as_raw_fd(), libc::STDERR_FILENO) };
    }
}

fn spawn_session(
    cols: u16,
    rows: u16,
    colors: &colors::HostColors,
    wake: impl Fn() + Send + 'static,
) -> anyhow::Result<SessionHandle> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let shell = std::env::var("SHELL").unwrap_or_else(|_| "bash".to_string());
    let mut cmd = CommandBuilder::new(&shell);
    cmd.env("TERM", "xterm-256color");
    let _child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);

    let writer = pair.master.take_writer()?;
    let reader = pair.master.try_clone_reader()?;
    let master = pair.master;

    let io = SessionIo {
        reader: Box::new(reader),
        writer: Box::new(writer),
        resizer: Box::new(move |cols, rows| {
            master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| e.into())
        }),
    };

    let config = SessionConfig {
        colors: Some(colors.clone()),
        ..SessionConfig::default()
    };

    SessionHandle::spawn(config, io, cols, rows, wake).map_err(|e| anyhow::anyhow!(e))
}

fn inner_area(area: Rect) -> Rect {
    Rect {
        x: area.x + 1,
        y: area.y + 1,
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
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
    let term_size: Rect = term.size()?.into();

    let (tx, rx) = mpsc::channel::<AppEvent>();

    let input_tx = tx.clone();
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

    let wake_tx = tx.clone();
    let inner = inner_area(term_size);
    let session = spawn_session(inner.width, inner.height, &host_colors, move || {
        let _ = wake_tx.send(AppEvent::SessionUpdate);
    })?;

    let mut title = String::from("terminal");
    let mut running = true;
    let frame_dur = Duration::from_millis(16);
    let mut last_render = Instant::now()
        .checked_sub(frame_dur)
        .unwrap_or_else(Instant::now);
    let mut render_pending = false;

    while running {
        let event = if render_pending {
            let remaining = frame_dur.saturating_sub(last_render.elapsed());
            if remaining.is_zero() {
                None
            } else {
                match rx.recv_timeout(remaining) {
                    Ok(ev) => Some(ev),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => break,
                }
            }
        } else {
            match rx.recv() {
                Ok(ev) => Some(ev),
                Err(_) => break,
            }
        };

        let mut events = Vec::new();
        if let Some(ev) = event {
            events.push(ev);
        }
        while let Ok(ev) = rx.try_recv() {
            events.push(ev);
        }

        for ev in events {
            match ev {
                AppEvent::Crossterm(ct) => match ct {
                    Event::Key(key) if key.kind == KeyEventKind::Press => {
                        if key.code == KeyCode::Char('q')
                            && key.modifiers.contains(KeyModifiers::CONTROL)
                        {
                            running = false;
                        } else {
                            session.send_key(key);
                        }
                    }
                    Event::Mouse(mouse) => {
                        if let Some(mut input) = mouse.into_mouse_input() {
                            let area = inner_area(term.size()?.into());
                            input.col = input.col.saturating_sub(area.x);
                            input.row = input.row.saturating_sub(area.y);
                            session.send_mouse(input);
                        }

                        if matches!(mouse.kind, MouseEventKind::ScrollUp) {
                            session.send_scroll(-3);
                        } else if matches!(mouse.kind, MouseEventKind::ScrollDown) {
                            session.send_scroll(3);
                        }
                    }
                    Event::Paste(s) => session.send_paste(s.into_bytes()),
                    Event::FocusGained => session.send_focus(true),
                    Event::FocusLost => session.send_focus(false),
                    Event::Resize(w, h) => {
                        let area = inner_area(Rect::new(0, 0, w, h));
                        session.send_resize(area.width, area.height);
                    }
                    _ => {}
                },
                AppEvent::SessionUpdate => {}
            }
        }

        while let Some(ev) = session.poll_event() {
            match ev {
                SessionEvent::TitleChanged(t) => title = t,
                SessionEvent::Exited => running = false,
                _ => {}
            }
        }

        if !session.is_alive() {
            running = false;
        }

        if session.is_dirty() {
            session.mark_clean();
            let now = Instant::now();
            if now.duration_since(last_render) >= frame_dur {
                term.draw(|frame| {
                    let area = frame.area();
                    let block = Block::default().borders(Borders::ALL).title(title.as_str());
                    block.render(area, frame.buffer_mut());
                    let inner = inner_area(area);
                    session.blit_to(frame.buffer_mut(), inner);
                })?;

                let cursor = session.cursor_state();
                if let Some(pos) = cursor.position {
                    let area = inner_area(term.size()?.into());
                    let cx = area.x + pos.x;
                    let cy = area.y + pos.y;
                    if cx < area.right() && cy < area.bottom() {
                        execute!(term.backend_mut(), cursor::MoveTo(cx, cy), cursor::Show)?;
                        let ct_style = match (cursor.style, cursor.blinking) {
                            (CursorStyle::Block, false) => SetCursorStyle::SteadyBlock,
                            (CursorStyle::Block, true) => SetCursorStyle::BlinkingBlock,
                            (CursorStyle::Bar, false) => SetCursorStyle::SteadyBar,
                            (CursorStyle::Bar, true) => SetCursorStyle::BlinkingBar,
                            (CursorStyle::Underline, false) => SetCursorStyle::SteadyUnderScore,
                            (CursorStyle::Underline, true) => SetCursorStyle::BlinkingUnderScore,
                        };
                        execute!(term.backend_mut(), ct_style)?;
                    }
                }

                last_render = now;
                render_pending = false;
            } else {
                render_pending = true;
            }
        }
    }

    terminal::disable_raw_mode()?;
    execute!(
        term.backend_mut(),
        DisableMouseCapture,
        DisableBracketedPaste,
        LeaveAlternateScreen,
        cursor::Show,
        SetCursorStyle::DefaultUserShape
    )?;

    Ok(())
}
