//! Throwaway probe: what libghostty-vt gives the scrollback change for free.
//! Each section prints what it observed; nothing here is Ship code.

use std::time::Instant;

use libghostty_vt::{
    Terminal,
    fmt::{Format, Formatter, FormatterOptions},
    selection::{Adjustment, FormatOptions, Selection},
    terminal::{Mode, Point, PointCoordinate, PointSpace, ScrollViewport},
};
use unicode_width::UnicodeWidthStr;

type R = Result<(), Box<dyn std::error::Error>>;

fn term(cols: u16, rows: u16, history: usize) -> Terminal<'static, 'static> {
    let mut t = Terminal::new(cols, rows).unwrap();
    t.set_scrollback_max_lines(Some(history)).unwrap();
    t
}

fn lines(t: &mut Terminal, from: usize, to: usize) {
    for i in from..to {
        t.vt_write(format!("line {i}\r\n").as_bytes());
    }
}

fn plain(t: &Terminal, unwrap: bool) -> String {
    let opts = FormatterOptions::new()
        .with_format(Format::Plain)
        .with_unwrap(unwrap)
        .with_trim(true);
    let mut f = Formatter::new(t, opts).unwrap();
    String::from_utf8(f.format_alloc(None).unwrap().to_vec()).unwrap()
}

fn sel_text(t: &Terminal, s: &Selection, unwrap: bool) -> String {
    let o = FormatOptions::new()
        .with_emit_format(Format::Plain)
        .with_unwrap(unwrap)
        .with_trim(true)
        .with_selection(s);
    String::from_utf8(t.format_selection_alloc(None, o).unwrap().unwrap().to_vec()).unwrap()
}

fn screen(x: u16, y: u32) -> Point {
    Point::Screen(PointCoordinate { x, y })
}

fn viewport_top(t: &Terminal) -> String {
    let opts = FormatterOptions::new().with_format(Format::Plain).with_trim(true);
    // Formatter covers the whole screen; read the viewport's first row directly.
    let _ = opts;
    let g = t.grid_ref(Point::Viewport(PointCoordinate { x: 0, y: 0 })).unwrap();
    let mut buf = ['\0'; 16];
    let mut s = String::new();
    for x in 0..12 {
        let g2 = t.grid_ref(Point::Viewport(PointCoordinate { x, y: 0 })).unwrap();
        let n = g2.graphemes(&mut buf).unwrap_or(0);
        s.extend(&buf[..n]);
    }
    let _ = g;
    s
}

fn main() -> R {
    // A. Does the formatter include history, and how do wraps and wide chars come out?
    println!("== A. text extraction");
    let mut t = term(20, 5, 100);
    lines(&mut t, 0, 8);
    t.vt_write("\x1b[31mred\x1b[0m 日本語 🙂 x\r\n".as_bytes());
    t.vt_write(b"0123456789ABCDEFGHIJ0123456789abcdefghij-wrapped\r\n");
    println!("scrollback_rows={} total_rows={}", t.scrollback_rows()?, t.total_rows()?);
    println!("-- plain, unwrap=false:\n{}", plain(&t, false));
    println!("-- plain, unwrap=true:\n{}", plain(&t, true));
    let vt = {
        let mut f = Formatter::new(&t, FormatterOptions::new().with_format(Format::Vt).with_style(true))?;
        String::from_utf8_lossy(&f.format_alloc(None)?).into_owned()
    };
    println!("-- vt contains SGR red: {}", vt.contains("31"));
    println!("-- vt head: {:?}", &vt[..vt.len().min(160)]);

    // H. Does lifting the byte cap make the line limit authoritative?
    println!("\n== H. byte cap vs line cap");
    for (lines_cap, bytes_cap) in [(10_000usize, None), (10_000, Some(64 << 20)), (1_000, Some(64 << 20)), (100_000, Some(64 << 20))] {
        let mut t = Terminal::new(200, 50)?;
        t.set_scrollback_max_bytes(bytes_cap)?;
        t.set_scrollback_max_lines(Some(lines_cap))?;
        for i in 0..lines_cap * 2 {
            t.vt_write(format!("{i:>8} cargo test some::module::path ... ok {}\r\n", "x".repeat(i % 120)).as_bytes());
        }
        let t1 = Instant::now();
        let text = plain(&t, false);
        let fmt = t1.elapsed();
        let t2 = Instant::now();
        let hits = search(&text, "needle-not-present");
        println!("lines={lines_cap} bytes={bytes_cap:?} -> read back bytes={:?} scrollback_rows={} format={fmt:?} ({} KB) full-scan search={:?} {}",
            t.scrollback_max_bytes()?, t.scrollback_rows()?, text.len() / 1024, t2.elapsed(), hits.len());
    }

    // B. "last N lines" via a selection from total_rows-N to the bottom.
    println!("\n== B. last 4 lines via selection");
    let total = t.total_rows()? as u32;
    let start = t.grid_ref(screen(0, total - 4))?;
    let end = t.grid_ref(screen(19, total - 1))?;
    let s = Selection::new(start, end, false);
    println!("{:?}", sel_text(&t, &s, true));

    // C. Scrolled viewport + new output: pinned or snapped to bottom?
    println!("\n== C. viewport pinning");
    let mut t = term(20, 5, 100);
    lines(&mut t, 0, 30);
    t.scroll_viewport(ScrollViewport::Delta(-10));
    println!("after scroll -10: active={} bar={:?} top={:?}", t.viewport_active()?, t.scrollbar()?, viewport_top(&t));
    lines(&mut t, 30, 33);
    println!("after 3 more lines: active={} bar={:?} top={:?}", t.viewport_active()?, t.scrollbar()?, viewport_top(&t));
    // Fill history to its limit while scrolled, so the pinned rows get pruned.
    lines(&mut t, 33, 200);
    println!("after history overflow: active={} bar={:?} top={:?}", t.viewport_active()?, t.scrollbar()?, viewport_top(&t));

    // D. Tracked refs (copy-mode cursor / selection anchors) when their row falls off history.
    println!("\n== D. tracked ref pruning");
    let mut t = term(20, 5, 20);
    lines(&mut t, 0, 30);
    let tr = t.track_grid_ref(screen(0, 2))?;
    println!("tracking row 2: has_value={} screen={:?}", tr.has_value(), tr.point(PointSpace::Screen)?);
    lines(&mut t, 30, 33);
    println!("after 3 lines: has_value={} screen={:?}", tr.has_value(), tr.point(PointSpace::Screen)?);
    lines(&mut t, 33, 80);
    println!("after overflow: has_value={} screen={:?}", tr.has_value(), tr.point(PointSpace::Screen)?);

    println!("-- again at 200 cols, byte cap lifted, 1000-line cap");
    let mut t = Terminal::new(200, 50)?;
    t.set_scrollback_max_bytes(None)?;
    t.set_scrollback_max_lines(Some(1000))?;
    lines(&mut t, 0, 1100);
    let tr = t.track_grid_ref(screen(0, 2))?;
    let pinned = t.track_grid_ref(screen(0, 500))?;
    println!("tracking rows 2 and 500: {:?} {:?}", tr.point(PointSpace::Screen)?, pinned.point(PointSpace::Screen)?);
    t.scroll_viewport(ScrollViewport::Row(400));
    lines(&mut t, 1100, 1400);
    println!("after 300 lines: row2 has_value={} {:?}; row500 {:?}; viewport top={:?} bar={:?}",
        tr.has_value(), tr.point(PointSpace::Screen)?, pinned.point(PointSpace::Screen)?, viewport_top(&t), t.scrollbar()?);
    lines(&mut t, 1400, 3000);
    println!("after 1600 more: row2 has_value={}; row500 has_value={}; viewport top={:?} bar={:?}",
        tr.has_value(), pinned.has_value(), viewport_top(&t), t.scrollbar()?);

    // E. Copy-mode style movement with Adjustment, then format.
    println!("\n== E. selection adjust");
    let mut t = term(20, 5, 100);
    lines(&mut t, 0, 10);
    let total = t.total_rows()? as u32;
    let a = t.grid_ref(screen(0, total - 3))?;
    let b = t.grid_ref(screen(0, total - 3))?;
    let mut s = Selection::new(a, b, false);
    s.adjust(&t, Adjustment::Up)?;
    s.adjust(&t, Adjustment::EndOfLine)?;
    println!("anchor row {} adjusted Up+EOL -> {:?}", total - 3, sel_text(&t, &s, true));
    // Word selection for `w`/`b`-style motions that Adjustment lacks.
    t.vt_write(b"foo bar-baz qux\r\n");
    let total = t.total_rows()? as u32;
    let row = total - 2;
    let w = t.select_word(libghostty_vt::selection::SelectWordOptions::new(t.grid_ref(screen(5, row))?))?;
    println!("select_word at col 5 of 'foo bar-baz qux': {:?}", w.map(|w| sel_text(&t, &w, true)));

    // F. Alternate screen: what history and formatter see.
    println!("\n== F. alternate screen");
    let mut t = term(20, 5, 100);
    lines(&mut t, 0, 20);
    let before = t.scrollback_rows()?;
    t.vt_write(b"\x1b[?1049h\x1b[Hnvim here");
    println!("primary scrollback={before}, in alt: active_screen={:?} scrollback_rows={} alt_scroll_mode={}",
        t.active_screen()?, t.scrollback_rows()?, t.mode(Mode::ALT_SCROLL)?);
    println!("formatter in alt: {:?}", plain(&t, true));
    t.scroll_viewport(ScrollViewport::Delta(-3));
    println!("scroll in alt: active={}", t.viewport_active()?);

    // G. Cost at realistic sizes, and a naive owned search over formatted rows.
    println!("\n== G. cost and search");
    for history in [1_000usize, 10_000, 100_000] {
        let mut t = term(200, 50, history);
        let t0 = Instant::now();
        for i in 0..history + 50 {
            t.vt_write(format!("{i:>8} cargo test some::module::path ... ok 日本 {}\r\n", "x".repeat(i % 120)).as_bytes());
        }
        let fill = t0.elapsed();
        println!("  set max_lines={history}: read back {:?}, max_bytes={:?}, scrollback_rows={} total_rows={}",
            t.scrollback_max_lines()?, t.scrollback_max_bytes()?, t.scrollback_rows()?, t.total_rows()?);
        let t1 = Instant::now();
        let text = plain(&t, false);
        let fmt = t1.elapsed();
        let t2 = Instant::now();
        let hits = search(&text, "99 cargo");
        let srch = t2.elapsed();
        // Check the first hit's mapped cell really holds the match's first char.
        let ok = hits.first().map(|&(y, x)| {
            let mut b = ['\0'; 4];
            let n = t.grid_ref(screen(x, y)).and_then(|g| g.graphemes(&mut b)).unwrap_or(0);
            b[..n].iter().collect::<String>()
        });
        println!("history={history:>6}: fill={fill:?} format={fmt:?} ({} KB) search={srch:?} hits={} first_cell={ok:?}",
            text.len() / 1024, hits.len());
    }
    // A wide-char hit, to check column mapping past double-width cells.
    let mut t = term(40, 5, 100);
    t.vt_write("日本語 needle\r\n".as_bytes());
    let text = plain(&t, false);
    let hits = search(&text, "needle");
    let (y, x) = hits[0];
    let mut b = ['\0'; 4];
    let n = t.grid_ref(screen(x, y))?.graphemes(&mut b)?;
    println!("wide-char hit at row {y} col {x}: cell={:?}", b[..n].iter().collect::<String>());
    // A match split across a soft wrap.
    let mut t = term(10, 5, 100);
    t.vt_write(b"aaaaaaneedle\r\n");
    println!("wrapped 'needle' across rows, row-wise search finds {} hits", search(&plain(&t, false), "needle").len());
    println!("unwrapped search finds {} hits", plain(&t, true).matches("needle").count());

    Ok(())
}

/// Naive owned search: one formatted line per screen row, substring match,
/// column from display width of the prefix. Returns (row, col) per hit.
fn search(text: &str, needle: &str) -> Vec<(u32, u16)> {
    let mut out = Vec::new();
    for (y, line) in text.split('\n').enumerate() {
        for (i, _) in line.match_indices(needle) {
            out.push((y as u32, line[..i].width() as u16));
        }
    }
    out
}
