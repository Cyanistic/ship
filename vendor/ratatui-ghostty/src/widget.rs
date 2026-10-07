use libghostty_vt::render::{CellIterator, CursorVisualStyle, RenderState, RowIterator};
use libghostty_vt::screen::CellContentTag;
use libghostty_vt::style::RgbColor;
use libghostty_vt::terminal::Terminal;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::Color;
use ratatui::widgets::Widget;

use crate::convert::to_ratatui;

/// Cursor information extracted during rendering.
#[derive(Debug, Clone, Default)]
pub struct CursorState {
    /// Position relative to the widget's render area where the cursor should be drawn.
    /// `None` when the cursor is hidden or outside the visible area.
    pub position: Option<Position>,
    pub style: CursorStyle,
    pub blinking: bool,
}

/// Visual style of the terminal cursor.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CursorStyle {
    #[default]
    Block,
    Bar,
    Underline,
}

pub struct TerminalWidget<'a, 'alloc, 'cb> {
    terminal: &'a mut Terminal<'alloc, 'cb>,
    render_state: &'a mut RenderState<'alloc>,
    focused: bool,
    cursor: CursorState,
}

impl<'a, 'alloc, 'cb> TerminalWidget<'a, 'alloc, 'cb> {
    pub fn new(
        terminal: &'a mut Terminal<'alloc, 'cb>,
        render_state: &'a mut RenderState<'alloc>,
    ) -> Self {
        Self {
            terminal,
            render_state,
            focused: false,
            cursor: CursorState::default(),
        }
    }

    pub fn focused(mut self, f: bool) -> Self {
        self.focused = f;
        self
    }

    pub fn cursor(&self) -> &CursorState {
        &self.cursor
    }
}

fn rgb_to_color(c: RgbColor) -> Color {
    Color::Rgb(c.r, c.g, c.b)
}

impl Widget for &mut TerminalWidget<'_, '_, '_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let Ok(snapshot) = self.render_state.update(self.terminal) else {
            return;
        };

        let Ok(colors) = snapshot.colors() else {
            return;
        };

        let cursor_visible = snapshot.cursor_visible().unwrap_or(false);
        let cursor_viewport = snapshot.cursor_viewport().ok().flatten();
        let cursor_visual_style = snapshot.cursor_visual_style().ok();
        let cursor_blinking = snapshot.cursor_blinking().unwrap_or(false);

        self.cursor = CursorState::default();
        if self.focused
            && cursor_visible
            && let Some(ref cursor) = cursor_viewport
            && !cursor.at_wide_tail
            && cursor.x < area.width
            && cursor.y < area.height
        {
            self.cursor.position = Some(Position::new(cursor.x, cursor.y));
            self.cursor.style = match cursor_visual_style {
                Some(CursorVisualStyle::Bar) => CursorStyle::Bar,
                Some(CursorVisualStyle::Underline) => CursorStyle::Underline,
                _ => CursorStyle::Block,
            };
            self.cursor.blinking = cursor_blinking;
        }

        let mut row_iter = match RowIterator::new() {
            Ok(r) => r,
            Err(_) => return,
        };
        let mut cell_iter = match CellIterator::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        let Ok(mut row_iteration) = row_iter.update(&snapshot) else {
            return;
        };

        let mut row_idx: u16 = 0;
        while let Some(row) = row_iteration.next() {
            if row_idx >= area.height {
                break;
            }

            let Ok(mut cell_iteration) = cell_iter.update(row) else {
                row_idx += 1;
                continue;
            };

            let mut col_idx: u16 = 0;
            while let Some(cell) = cell_iteration.next() {
                if col_idx >= area.width {
                    break;
                }

                let symbol = match cell.graphemes_len() {
                    Ok(0) | Err(_) => " ".to_string(),
                    Ok(_) => match cell.graphemes() {
                        Ok(chars) => chars.into_iter().collect::<String>(),
                        Err(_) => " ".to_string(),
                    },
                };

                // Unset colors stay `Reset`, so the host terminal's defaults
                // show. A background from erasing with a color set lives in
                // the cell content rather than its style.
                let cell_style = cell.style().ok();
                let mut ratatui_style = cell_style
                    .as_ref()
                    .map(|s| to_ratatui::style(s, &colors.palette))
                    .unwrap_or_default();
                if ratatui_style.bg.is_none()
                    && let Ok(raw) = cell.raw_cell()
                {
                    ratatui_style.bg = match raw.content_tag() {
                        Ok(CellContentTag::BgColorPalette) => {
                            raw.bg_color_palette().ok().map(|idx| Color::Indexed(idx.0))
                        }
                        Ok(CellContentTag::BgColorRgb) => raw.bg_color_rgb().ok().map(rgb_to_color),
                        _ => None,
                    };
                }

                let buf_x = area.x + col_idx;
                let buf_y = area.y + row_idx;
                if buf_x < buf.area().right() && buf_y < buf.area().bottom() {
                    let buf_cell = &mut buf[(buf_x, buf_y)];
                    buf_cell.set_symbol(&symbol);
                    buf_cell.set_style(ratatui_style);
                }

                col_idx += 1;
            }

            row_idx += 1;
        }
    }
}
