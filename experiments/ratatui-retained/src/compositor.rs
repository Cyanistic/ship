use ratatui::backend::Backend;
use ratatui::buffer::{Buffer, Cell};
use ratatui::layout::{Position, Rect};

use crate::region::Region;

/// One independently changing piece of the screen, painted bottom to top.
///
/// The contract that makes partial repaint match a full repaint:
/// - `damage` reports every cell whose painted value may differ from the last
///   time it reported, and nothing else needs to be reported. Moving or
///   hiding the layer is the compositor's job.
/// - `paint` is deterministic, writes only inside `extent(area, want)`, and
///   each cell it writes depends only on the layer's state and that cell's
///   previous value. All ratatui widgets behave this way.
pub trait Layer {
    /// Where the layer sits, or `None` while hidden.
    fn area(&self) -> Option<Rect>;
    /// Adds the cells that changed since the last call to `out`.
    fn damage(&mut self, area: Rect, out: &mut Region);
    /// The rect `paint` may write when asked for `want`. Most widgets can't
    /// paint less than their whole area.
    fn extent(&self, area: Rect, want: Rect) -> Rect {
        let _ = want;
        area
    }
    /// Whether painting could change any cell of `damage`. Answering true
    /// is always safe; false lets the compositor skip the layer.
    fn touches(&self, area: Rect, damage: &Region) -> bool {
        let _ = (area, damage);
        true
    }
    /// Paints at least the cells of `want`, which lies inside `area`.
    fn paint(&self, area: Rect, want: &Region, buf: &mut Buffer);
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

struct Slot {
    layer: Box<dyn Layer>,
    painted: Option<Rect>,
}

/// Keeps the last frame and repaints only damaged cells, then writes only
/// the damaged rows' changes.
pub struct Compositor<B: Backend> {
    backend: B,
    slots: Vec<Slot>,
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
    pub fn new(backend: B, size: Rect, layers: Vec<Box<dyn Layer>>) -> Self {
        Self {
            backend,
            slots: layers
                .into_iter()
                .map(|layer| Slot {
                    layer,
                    painted: None,
                })
                .collect(),
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

    /// Starts over at a new size, as ratatui does: clear the terminal and
    /// repaint everything on the next draw.
    pub fn resize(&mut self, size: Rect) -> Result<(), B::Error> {
        self.back = Buffer::empty(size);
        self.front = Buffer::empty(size);
        self.full = true;
        self.stale = false;
        self.backend.clear()
    }

    pub fn draw(&mut self, cursor: Option<Position>) -> Result<Stats, B::Error> {
        let screen = self.back.area;
        self.damage.clear();
        if self.full {
            self.damage.add(screen);
            self.full = false;
        }
        for slot in &mut self.slots {
            let area = slot
                .layer
                .area()
                .map(|a| a.intersection(screen))
                .filter(|a| !a.is_empty());
            if area != slot.painted {
                // The old area may lie outside a screen that shrank.
                self.damage
                    .add(slot.painted.unwrap_or_default().intersection(screen));
                self.damage.add(area.unwrap_or_default());
                slot.painted = area;
            }
            if let Some(area) = area {
                self.reported.clear();
                slot.layer.damage(area, &mut self.reported);
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
            stats = self.draw_whole()?;
        } else if !self.damage.is_empty() {
            if self.stale {
                self.back.content.clone_from_slice(&self.front.content);
                self.stale = false;
            }
            stats.repainted = self.repaint();
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
    fn draw_whole(&mut self) -> Result<Stats, B::Error> {
        self.back.reset();
        self.render_into_back();
        let updates = self.front.diff(&self.back);
        let written = updates.len();
        if !updates.is_empty() {
            self.backend.draw(updates.into_iter())?;
        }
        std::mem::swap(&mut self.front, &mut self.back);
        self.stale = true;
        let cells = self.front.area.area() as usize;
        Ok(Stats {
            repainted: cells,
            compared: cells,
            written,
        })
    }

    fn render_into_back(&mut self) {
        let mut back = std::mem::take(&mut self.back);
        self.render_full(&mut back);
        self.back = back;
    }

    /// Resets the damaged cells, then replays every layer over them bottom to
    /// top, so each damaged cell ends as a full repaint would leave it.
    fn repaint(&mut self) -> usize {
        let mut repainted = 0;
        for r in self.damage.rects() {
            repainted += r.area() as usize;
            for y in r.top()..r.bottom() {
                let i = self.back.index_of(r.x, y);
                self.back.content[i..i + r.width as usize].fill(Cell::EMPTY);
            }
        }
        for slot in &self.slots {
            let Some(area) = slot.painted else { continue };
            self.damage.clip_into(area, &mut self.want);
            if self.want.is_empty()
                || !self.want.covers(area) && !slot.layer.touches(area, &self.want)
            {
                continue;
            }
            let extent = slot
                .layer
                .extent(area, self.want.bounds())
                .union(self.want.bounds())
                .intersection(area);
            if self.want.covers(extent) {
                slot.layer.paint(area, &self.want, &mut self.back);
                continue;
            }
            // The layer would paint cells outside the damage, which must keep
            // the layers above them. Paint into a scratch buffer and copy
            // back only the damaged cells.
            self.scratch.resize(extent);
            copy(&self.back, &mut self.scratch, &self.want);
            slot.layer.paint(area, &self.want, &mut self.scratch);
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

    /// Paints every visible layer from scratch, the way a plain ratatui frame
    /// would. The reference partial repaints must match.
    pub fn render_full(&self, buf: &mut Buffer) {
        let screen = buf.area;
        for slot in &self.slots {
            let Some(area) = slot
                .layer
                .area()
                .map(|a| a.intersection(screen))
                .filter(|a| !a.is_empty())
            else {
                continue;
            };
            let mut want = Region::default();
            want.add(area);
            slot.layer.paint(area, &want, buf);
        }
    }
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
