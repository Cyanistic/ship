use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;

use crate::app::App;
use crate::layout::tall_layout;

const ACCENT: Color = Color::Blue;
const BORDER_DIM: Color = Color::DarkGray;

pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);
    draw_panes(frame, app, chunks[0]);
    draw_status_bar(frame, app, chunks[1]);
}

fn draw_panes(frame: &mut Frame, app: &mut App, area: Rect) {
    let tab = &mut app.tabs[app.active_tab];
    let active = tab.active;
    let pane_count = tab.panes.len();
    let rects = tall_layout(area, pane_count);

    for (i, (pane, &rect)) in tab.panes.iter().zip(rects.iter()).enumerate() {
        pane.blit_to(frame.buffer_mut(), rect);
        pane.mark_clean();
        tab.rects[i] = rect;

        if i == active {
            let cursor = pane.cursor_state();
            if let Some(pos) = cursor.position {
                frame.set_cursor_position(Position::new(rect.x + pos.x, rect.y + pos.y));
            }
        }
    }

    draw_separators(frame, area, &rects, active);
}

fn draw_separators(frame: &mut Frame, area: Rect, rects: &[Rect], active: usize) {
    if rects.len() <= 1 {
        return;
    }

    let active_style = Style::default().fg(ACCENT);
    let dim_style = Style::default().fg(BORDER_DIM);

    let sep_x = rects[0].right();
    let h_seps: Vec<(u16, usize, usize)> = (1..rects.len().saturating_sub(1))
        .map(|i| (rects[i].bottom(), i, i + 1))
        .collect();

    let buf = frame.buffer_mut();

    for y in area.y..area.bottom() {
        let adjacent_active = if active == 0 {
            true
        } else {
            let r = rects[active];
            let in_pane = y >= r.y && y < r.bottom();
            let in_gap = h_seps
                .iter()
                .any(|&(sy, above, below)| sy == y && (active == above || active == below));
            in_pane || in_gap
        };
        let style = if adjacent_active {
            active_style
        } else {
            dim_style
        };
        let sym = if h_seps.iter().any(|&(sy, _, _)| sy == y) {
            "├"
        } else {
            "│"
        };
        buf.set_string(sep_x, y, sym, style);
    }

    for &(sep_y, above, below) in &h_seps {
        let adjacent_active = active == above || active == below;
        let style = if adjacent_active {
            active_style
        } else {
            dim_style
        };
        for x in (sep_x + 1)..area.right() {
            buf.set_string(x, sep_y, "─", style);
        }
    }
}

fn draw_status_bar(frame: &mut Frame, app: &App, area: Rect) {
    let tab = &app.tabs[app.active_tab];
    let term_size = app
        .active_pane()
        .map(|p| {
            let (c, r) = p.size();
            format!("{c}x{r}")
        })
        .unwrap_or_default();
    let pane_info = format!("pane {}/{}", tab.active + 1, tab.panes.len());

    let dim = Style::default().fg(Color::DarkGray).bg(Color::Black);
    let inactive = Style::default().fg(Color::White).bg(Color::Black);
    let active = Style::default()
        .fg(Color::Black)
        .bg(ACCENT)
        .add_modifier(Modifier::BOLD);

    let mut left: Vec<Span> = Vec::new();
    left.push(Span::styled(" [ratatui-ghostty] ", active));
    for (i, t) in app.tabs.iter().enumerate() {
        let title = t
            .titles
            .get(t.active)
            .filter(|s| !s.is_empty())
            .cloned()
            .unwrap_or_else(|| "shell".to_string());
        let is_active = i == app.active_tab;
        let label = if is_active {
            format!(" {}:{}* ", i + 1, title)
        } else {
            format!(" {}:{} ", i + 1, title)
        };
        left.push(Span::styled(label, if is_active { inactive } else { dim }));
    }

    let mut right_parts = vec![pane_info, term_size];
    if app.prefix.active {
        right_parts.push("[PREFIX]".to_string());
    }
    let right_text = format!(" {} ", right_parts.join(" | "));
    let right_width = right_text.len() as u16;

    let left_text_width: u16 = left.iter().map(|s| s.content.len() as u16).sum();
    let padding = area.width.saturating_sub(left_text_width + right_width);
    left.push(Span::styled(" ".repeat(padding as usize), inactive));
    left.push(Span::styled(right_text, dim));

    let bar = Paragraph::new(Line::from(left)).style(Style::default().bg(Color::Black));
    frame.render_widget(bar, area);
}
