use std::mem;

use ratatui::backend::Backend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};

use crate::region::Region;

/// A frame's layers, bottom to top, addressed by index. An index must name
/// the same layer from one draw to the next, since damage is tracked per
/// index.
///
/// The contract that makes partial repaint match a full repaint:
/// - `damage` reports every cell of the layer whose painted value may differ
///   from the last time it reported. Moving or hiding the layer is the
///   compositor's job.
/// - `paint` is deterministic, writes only inside `extent(area, want)`, and
///   each cell it writes depends only on the layer's state and that cell's
///   previous value. All ratatui widgets behave this way.
pub trait Stack {
    /// How many layers there are.
    fn layers(&self) -> usize;
    /// Where the layer sits, or `None` while hidden.
    fn area(&self, layer: usize) -> Option<Rect>;
    /// Adds the cells that changed since the last call to `out`.
    fn damage(&mut self, layer: usize, area: Rect, out: &mut Region);
    /// The rect `paint` may write when asked for `want`. Most widgets can't
    /// paint less than their whole area.
    fn extent(&self, layer: usize, area: Rect, want: Rect) -> Rect {
        let _ = (layer, want);
        area
    }
    /// Whether painting could change any cell of `damage`. Answering true
    /// is always safe; false lets the compositor skip the layer.
    fn touches(&mut self, layer: usize, area: Rect, damage: &Region) -> bool {
        let _ = (layer, area, damage);
        true
    }
    /// Paints at least the cells of `want`, which lies inside `area`.
    fn paint(&mut self, layer: usize, area: Rect, want: &Region, buf: &mut Buffer);
}

/// One layer that owns its state, for stacks built once and kept.
pub trait Layer {
    fn area(&self) -> Option<Rect>;
    fn damage(&mut self, area: Rect, out: &mut Region);
    fn extent(&self, area: Rect, want: Rect) -> Rect {
        let _ = want;
        area
    }
    fn touches(&mut self, area: Rect, damage: &Region) -> bool {
        let _ = (area, damage);
        true
    }
    fn paint(&mut self, area: Rect, want: &Region, buf: &mut Buffer);
}

impl Stack for [Box<dyn Layer>] {
    fn layers(&self) -> usize {
        <[_]>::len(self)
    }
    fn area(&self, layer: usize) -> Option<Rect> {
        self[layer].area()
    }
    fn damage(&mut self, layer: usize, area: Rect, out: &mut Region) {
        self[layer].damage(area, out);
    }
    fn extent(&self, layer: usize, area: Rect, want: Rect) -> Rect {
        self[layer].extent(area, want)
    }
    fn touches(&mut self, layer: usize, area: Rect, damage: &Region) -> bool {
        self[layer].touches(area, damage)
    }
    fn paint(&mut self, layer: usize, area: Rect, want: &Region, buf: &mut Buffer) {
        self[layer].paint(area, want, buf);
    }
}

/// What one `draw` did, to compare against a full ratatui frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Stats {
    /// Cells reset and repainted (overlapping damage counts twice).
    pub repainted: usize,
    /// Cells compared against what the terminal shows.
    pub compared: usize,
    /// Cells written to the backend.
    pub written: usize,
}

/// Keeps the last frame and repaints only damaged cells, then writes only
/// the damaged rows' changes.
pub struct Compositor<B: Backend> {
    backend: B,
    /// Each layer's area as last painted.
    painted: Vec<Option<Rect>>,
    /// The frame being built, kept between draws.
    back: Buffer,
    /// What the terminal shows.
    front: Buffer,
    scratch: Buffer,
    damage: Region,
    want: Region,
    reported: Region,
    /// Shown cursor, `None` until the first draw.
    cursor: Option<Option<Position>>,
    full: bool,
    /// `back` holds the frame before last, after a whole-screen draw
    /// swapped the buffers.
    stale: bool,
}

impl<B: Backend> Compositor<B> {
    pub fn new(backend: B, size: Rect) -> Self {
        Self {
            backend,
            painted: Vec::new(),
            back: Buffer::empty(size),
            front: Buffer::empty(size),
            scratch: Buffer::empty(Rect::default()),
            damage: Region::default(),
            want: Region::default(),
            reported: Region::default(),
            cursor: None,
            full: true,
            stale: false,
        }
    }

    pub fn backend_mut(&mut self) -> &mut B {
        &mut self.backend
    }

    /// The current frame.
    pub fn buffer(&self) -> &Buffer {
        if self.stale { &self.front } else { &self.back }
    }

    pub fn area(&self) -> Rect {
        self.back.area
    }

    /// Starts over at a new size, as ratatui does: clear the terminal and
    /// repaint everything on the next draw.
    pub fn resize(&mut self, size: Rect) -> Result<(), B::Error> {
        self.back = Buffer::empty(size);
        self.front = Buffer::empty(size);
        self.full = true;
        self.stale = false;
        self.backend.clear()
    }

    pub fn draw<S: Stack + ?Sized>(
        &mut self,
        stack: &mut S,
        cursor: Option<Position>,
    ) -> Result<Stats, B::Error> {
        let screen = self.back.area;
        self.damage.clear();
        if self.full {
            self.damage.add(screen);
            self.full = false;
        }
        self.painted.resize(stack.layers(), None);
        for (layer, painted) in self.painted.iter_mut().enumerate() {
            let area = stack
                .area(layer)
                .map(|a| a.intersection(screen))
                .filter(|a| !a.is_empty());
            if area != *painted {
                // The old area may lie outside a screen that shrank.
                self.damage
                    .add(painted.unwrap_or_default().intersection(screen));
                self.damage.add(area.unwrap_or_default());
                *painted = area;
            }
            if let Some(area) = area {
                self.reported.clear();
                stack.damage(layer, area, &mut self.reported);
                for r in self.reported.rects() {
                    self.damage.add(r.intersection(area));
                }
            }
        }

        let mut stats = Stats::default();
        let damaged: u32 = self.damage.rects().iter().map(|r| r.area()).sum();
        if damaged * 2 >= screen.area() {
            // Most of the screen changed: tracking which cells to keep costs
            // more than it saves, so draw a plain ratatui frame instead.
            stats = self.draw_whole(stack)?;
        } else if !self.damage.is_empty() {
            if self.stale {
                self.back.content.clone_from_slice(&self.front.content);
                self.stale = false;
            }
            stats.repainted = self.repaint(stack);
            (stats.compared, stats.written) = self.flush_rows()?;
        }
        let moved = self.cursor != Some(cursor);
        if moved {
            match cursor {
                Some(p) => {
                    self.backend.show_cursor()?;
                    self.backend.set_cursor_position(p)?;
                }
                None => self.backend.hide_cursor()?,
            }
            self.cursor = Some(cursor);
        } else if stats.written > 0
            && let Some(p) = cursor
        {
            // Writing cells moved the terminal's cursor.
            self.backend.set_cursor_position(p)?;
        }
        if moved || stats.written > 0 {
            self.backend.flush()?;
        }
        Ok(stats)
    }

    /// What `Terminal::draw` does: paint everything into an empty buffer,
    /// diff the whole screen, and swap buffers rather than copy.
    fn draw_whole<S: Stack + ?Sized>(&mut self, stack: &mut S) -> Result<Stats, B::Error> {
        self.back.reset();
        render_full(stack, &mut self.back);
        let updates = self.front.diff(&self.back);
        let written = updates.len();
        if !updates.is_empty() {
            self.backend.draw(updates.into_iter())?;
        }
        mem::swap(&mut self.front, &mut self.back);
        self.stale = true;
        let cells = self.front.area.area() as usize;
        Ok(Stats {
            repainted: cells,
            compared: cells,
            written,
        })
    }

    /// Resets the damaged cells, then replays every layer over them bottom to
    /// top, so each damaged cell ends as a full repaint would leave it.
    fn repaint<S: Stack + ?Sized>(&mut self, stack: &mut S) -> usize {
        let mut repainted = 0;
        for r in self.damage.rects() {
            repainted += r.area() as usize;
            for y in r.top()..r.bottom() {
                let i = self.back.index_of(r.x, y);
                self.back.content[i..i + r.width as usize].fill(Cell::EMPTY);
            }
        }
        for (layer, painted) in self.painted.iter().enumerate() {
            let Some(area) = *painted else { continue };
            self.damage.clip_into(area, &mut self.want);
            if self.want.is_empty()
                || !self.want.covers(area) && !stack.touches(layer, area, &self.want)
            {
                continue;
            }
            let extent = stack
                .extent(layer, area, self.want.bounds())
                .union(self.want.bounds())
                .intersection(area);
            if self.want.covers(extent) {
                stack.paint(layer, area, &self.want, &mut self.back);
                continue;
            }
            // The layer would paint cells outside the damage, which must keep
            // the layers above them. Paint into a scratch buffer and copy
            // back only the damaged cells.
            self.scratch.resize(extent);
            copy(&self.back, &mut self.scratch, &self.want);
            stack.paint(layer, area, &self.want, &mut self.scratch);
            copy(&self.scratch, &mut self.back, &self.want);
        }
        repainted
    }

    /// Diffs whole damaged rows against the terminal with ratatui's own diff,
    /// so wide characters get the same handling as a full frame.
    fn flush_rows(&mut self) -> Result<(usize, usize), B::Error> {
        let screen = self.back.area;
        let rows: Vec<bool> = (screen.top()..screen.bottom())
            .map(|y| self.damage.has_row(y))
            .collect();
        let touched = rows.iter().filter(|r| **r).count() * screen.width as usize;
        let mut written = 0;
        if touched * 2 >= screen.area() as usize {
            // Copying most of the screen out to diff it costs more than
            // diffing the rest of it too.
            let updates = self.front.diff(&self.back);
            written = updates.len();
            if !updates.is_empty() {
                self.backend.draw(updates.into_iter())?;
            }
        } else {
            let mut y = 0;
            while y < rows.len() {
                if !rows[y] {
                    y += 1;
                    continue;
                }
                let top = y;
                while y < rows.len() && rows[y] {
                    y += 1;
                }
                let band = Rect::new(
                    screen.x,
                    screen.y + top as u16,
                    screen.width,
                    (y - top) as u16,
                );
                let start = self.back.index_of(band.x, band.y);
                let range = start..start + band.area() as usize;
                let band_of = |b: &Buffer| Buffer {
                    area: band,
                    content: b.content[range.clone()].to_vec(),
                };
                let (prev, next) = (band_of(&self.front), band_of(&self.back));
                let updates = prev.diff(&next);
                written += updates.len();
                if !updates.is_empty() {
                    self.backend.draw(updates.into_iter())?;
                }
            }
        }
        // Outside the damage the two already match.
        copy(&self.back, &mut self.front, &self.damage);
        Ok((touched.min(screen.area() as usize), written))
    }

    /// The first cell where the current frame differs from a from-scratch
    /// paint of `stack`, or `None` when they match. A mismatch means some
    /// layer broke the [`Stack`] contract.
    pub fn check<S: Stack + ?Sized>(&self, stack: &mut S) -> Option<Position> {
        let current = self.buffer();
        let mut reference = Buffer::empty(current.area);
        render_full(stack, &mut reference);
        let i = current
            .content
            .iter()
            .zip(&reference.content)
            .position(|(a, b)| a != b)?;
        Some(current.pos_of(i).into())
    }
}

/// Paints every visible layer from scratch, the way a plain ratatui frame
/// would. The reference partial repaints must match.
pub fn render_full<S: Stack + ?Sized>(stack: &mut S, buf: &mut Buffer) {
    let screen = buf.area;
    for layer in 0..stack.layers() {
        let Some(area) = stack
            .area(layer)
            .map(|a| a.intersection(screen))
            .filter(|a| !a.is_empty())
        else {
            continue;
        };
        let mut want = Region::default();
        want.add(area);
        stack.paint(layer, area, &want, buf);
    }
}

/// The cells `paint` writes in `area`, found by painting over two
/// backgrounds that differ in every field. Any write of a field changes at
/// least one of them, so a cell both leave alone is one `paint` never
/// touches. Answers [`Stack::touches`] for a layer whose writes aren't known
/// up front.
pub fn footprint(area: Rect, mut paint: impl FnMut(&mut Buffer)) -> Region {
    use ratatui::buffer::CellDiffOption;
    use ratatui::style::{Color, Modifier};
    let sentinel = |symbol, fg, bg, modifier, diff_option, skip| {
        let mut cell = Cell::new(symbol);
        cell.fg = fg;
        cell.bg = bg;
        cell.underline_color = fg;
        cell.modifier = modifier;
        cell.diff_option = diff_option;
        #[allow(deprecated)]
        {
            cell.skip = skip;
        }
        cell
    };
    let a = sentinel(
        "\u{E000}",
        Color::Rgb(1, 2, 3),
        Color::Rgb(4, 5, 6),
        Modifier::all(),
        CellDiffOption::None,
        false,
    );
    let b = sentinel(
        "\u{E001}",
        Color::Indexed(77),
        Color::Indexed(78),
        Modifier::empty(),
        CellDiffOption::AlwaysUpdate,
        true,
    );
    let (mut buf_a, mut buf_b) = (
        Buffer::filled(area, a.clone()),
        Buffer::filled(area, b.clone()),
    );
    paint(&mut buf_a);
    paint(&mut buf_b);
    let mut cells = Region::default();
    for y in area.top()..area.bottom() {
        let mut x = area.left();
        while x < area.right() {
            let written = |x| buf_a[(x, y)] != a || buf_b[(x, y)] != b;
            if !written(x) {
                x += 1;
                continue;
            }
            let start = x;
            while x < area.right() && written(x) {
                x += 1;
            }
            cells.add(Rect::new(start, y, x - start, 1));
        }
    }
    cells
}

fn copy(from: &Buffer, to: &mut Buffer, region: &Region) {
    let area = from.area.intersection(to.area);
    for r in region.rects() {
        let r = r.intersection(area);
        for y in r.top()..r.bottom() {
            let (i, j) = (from.index_of(r.x, y), to.index_of(r.x, y));
            let n = r.width as usize;
            to.content[j..j + n].clone_from_slice(&from.content[i..i + n]);
        }
    }
}
