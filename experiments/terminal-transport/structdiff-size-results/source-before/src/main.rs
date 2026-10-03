//! Throwaway RFC 6902 encoded-message probe. No proposed production model.
#![allow(deprecated)] // Preserve both legacy skip and current diff_option.
mod bench;
mod sizes;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use ratatui::{
    buffer::{Buffer, CellDiffOption},
    layout::Rect,
    style::{Color, Modifier},
};
use ratatui_ghostty::{
    session::{SessionHandle, SessionIo},
    widget::CursorStyle,
};
use serde::{Deserialize, Serialize};
use std::{
    io::Read,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use structdiff::{Difference, StructDiff};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct OwnedCell {
    text: String,
    fg: Color,
    bg: Color,
    underline: Color,
    modifiers: Modifier,
    skip: bool,
    diff_option: CellDiffOption,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Cursor {
    position: Option<(u16, u16)>,
    style: String,
    blinking: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Difference)]
struct Screen {
    cols: u16,
    rows: u16,
    cells: Vec<OwnedCell>, // Default whole-vector replacement, intentionally.
    cursor: Cursor,
}

fn capture(session: &SessionHandle) -> Screen {
    let (cols, rows) = session.size();
    let area = Rect::new(0, 0, cols, rows);
    let mut original = Buffer::empty(area);
    session.blit_to(&mut original, area);
    let c = session.cursor_state(); // Separate lock: not an atomic buffer/cursor capture.
    let cursor = Cursor {
        position: c.position.map(|p| (p.x, p.y)),
        style: match c.style {
            CursorStyle::Block => "block",
            CursorStyle::Bar => "bar",
            CursorStyle::Underline => "underline",
        }
        .into(),
        blinking: c.blinking,
    };
    let cells: Vec<_> = original
        .content
        .iter()
        .map(|c| OwnedCell {
            text: c.symbol().into(),
            fg: c.fg,
            bg: c.bg,
            underline: c.underline_color,
            modifiers: c.modifier,
            skip: c.skip,
            diff_option: c.diff_option,
        })
        .collect();
    let mut rebuilt = Buffer::empty(area);
    for (dest, src) in rebuilt.content.iter_mut().zip(&cells) {
        dest.set_symbol(&src.text);
        dest.fg = src.fg;
        dest.bg = src.bg;
        dest.underline_color = src.underline;
        dest.modifier = src.modifiers;
        dest.skip = src.skip;
        dest.diff_option = src.diff_option;
    }
    assert_eq!(original, rebuilt); // Ratatui normalizes unset symbol to space.
    let restored_style = match cursor.style.as_str() {
        "block" => CursorStyle::Block,
        "bar" => CursorStyle::Bar,
        "underline" => CursorStyle::Underline,
        _ => panic!("unknown experimental cursor style"),
    };
    assert_eq!(
        cursor
            .position
            .map(|(x, y)| ratatui::layout::Position::new(x, y)),
        c.position
    );
    assert_eq!(restored_style, c.style);
    assert_eq!(cursor.blinking, c.blinking);
    Screen {
        cols,
        rows,
        cells,
        cursor,
    }
}
fn text(screen: &Screen) -> String {
    screen
        .cells
        .chunks(screen.cols as usize)
        .map(|row| row.iter().map(|c| c.text.as_str()).collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}
fn settled(session: &SessionHandle, marker: &str, size: (u16, u16)) -> Screen {
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut previous = None;
    let mut stable_since = Instant::now();
    loop {
        let screen = capture(session);
        if previous.as_ref() != Some(&screen) {
            stable_since = Instant::now();
        }
        if (screen.cols, screen.rows) == size
            && text(&screen).contains(marker)
            && stable_since.elapsed() >= Duration::from_millis(120)
        {
            println!(
                "capture {marker} {}x{} cursor={:?}\n{}",
                screen.cols,
                screen.rows,
                screen.cursor,
                text(&screen)
            );
            return screen;
        }
        assert!(
            Instant::now() < deadline,
            "capture timeout: {}",
            text(&screen)
        );
        previous = Some(screen);
        thread::sleep(Duration::from_millis(20));
    }
}
// Observe reader destruction at EOF, because the wrapper does not expose a join handle.
struct ObservedReader(Box<dyn Read + Send>, mpsc::Sender<()>);
impl Read for ObservedReader {
    fn read(&mut self, bytes: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(bytes)
    }
}
impl Drop for ObservedReader {
    fn drop(&mut self) {
        let _ = self.1.send(());
    }
}
fn tiny_run() -> Result<(), Box<dyn std::error::Error>> {
    let captures = run_native(
        20,
        6,
        "stty -echo; printf '\\033[2J\\033[H\\033[1;31;44mRED\\033[0m café 界 é\r\nREADY'; read x; printf '\\033[2;1H\\033[3;4;38;2;10;20;30mEDIT\\033[0m'; read x; i=1; while [ $i -le 12 ]; do printf '\r\nscroll-%02d' $i; i=$((i+1)); done; read x; exit 0",
        |session| {
            let initial = settled(&session, "READY", (20, 6));
            assert!(text(&initial).contains("café 界") && text(&initial).contains("é"));
            assert!(
                initial
                    .cells
                    .iter()
                    .any(|c| c.modifiers.contains(Modifier::BOLD))
            );
            assert!(initial.cells.iter().any(
                |c| matches!(c.fg, Color::Rgb(_, _, _)) && matches!(c.bg, Color::Rgb(_, _, _))
            ));
            session.send_raw(b"\n".to_vec());
            let edited = settled(&session, "EDIT", (20, 6));
            assert!(
                edited.cells.iter().any(|c| c.fg == Color::Rgb(10, 20, 30)
                    && c.modifiers.contains(Modifier::UNDERLINED))
            );
            session.send_raw(b"\n".to_vec());
            let scrolled = settled(&session, "scroll-12", (20, 6));
            session.send_resize(24, 8);
            let resized = settled(&session, "scroll-12", (24, 8));
            let unchanged = settled(&session, "scroll-12", (24, 8));
            session.send_raw(b"\n".to_vec());
            vec![
                ("initial", initial),
                ("edit", edited),
                ("scroll-output", scrolled),
                ("resize", resized),
                ("unchanged", unchanged),
            ]
        },
    )?;
    bench::run(&captures);
    println!(
        "PASS: all actual capture JSON Patch/full snapshot/structdiff equality assertions and cleanup passed"
    );
    Ok(())
}

// Shared native lifecycle. Assertion panics still run bounded child/reader/session cleanup.
fn run_native<T>(
    cols: u16,
    rows: u16,
    script: &str,
    workflow: impl FnOnce(&SessionHandle) -> T,
) -> Result<T, Box<dyn std::error::Error>> {
    let size = |cols, rows| PtySize {
        cols,
        rows,
        pixel_width: 0,
        pixel_height: 0,
    };
    let pair = native_pty_system().openpty(size(cols, rows))?;
    let mut cmd = CommandBuilder::new("/bin/sh");
    cmd.args(["-c", script]);
    cmd.env("LC_ALL", "en_US.UTF-8");
    let mut child = pair.slave.spawn_command(cmd)?;
    drop(pair.slave);
    let (reader_tx, reader_rx) = mpsc::channel();
    let io = SessionIo {
        reader: Box::new(ObservedReader(pair.master.try_clone_reader()?, reader_tx)),
        writer: pair.master.take_writer()?,
        resizer: Box::new(move |cols, rows| Ok(pair.master.resize(size(cols, rows))?)),
    };
    let session = SessionHandle::builder(cols, rows, || {}).spawn(io)?;
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| workflow(&session)));
    // Reap our child and ensure detached reader/session terminate, including assertion failures.
    let deadline = Instant::now() + Duration::from_secs(5);
    while child.try_wait()?.is_none() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    if child.try_wait()?.is_none() {
        child.kill()?;
    }
    let status = child.wait()?;
    reader_rx.recv_timeout(Duration::from_secs(5))?;
    session.send_shutdown();
    let deadline = Instant::now() + Duration::from_secs(5);
    while session.is_alive() && Instant::now() < deadline {
        thread::sleep(Duration::from_millis(20));
    }
    assert!(!session.is_alive(), "session thread did not terminate");
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(event) = session.poll_event() {
            println!("session event: {event:?}");
            match event {
                ratatui_ghostty::session::SessionEvent::Exited => break,
                ratatui_ghostty::session::SessionEvent::Error(e) => panic!("session error: {e}"),
                _ => {}
            }
        }
        assert!(Instant::now() < deadline, "session Exited event missing");
        thread::sleep(Duration::from_millis(1));
    }
    let captures = match result {
        Ok(captures) => captures,
        Err(panic) => std::panic::resume_unwind(panic),
    };
    assert!(status.success());
    println!(
        "CLEANUP PASS: native buffer/cursor roundtrips, shell reaped ({status:?}), reader dropped, session stopped"
    );
    Ok(captures)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => tiny_run(),
        [mode] if mode == "--sizes" => sizes::run(),
        _ => Err("usage: ship-jsonpatch-prototype [--sizes]".into()),
    }
}
