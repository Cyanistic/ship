use ratatui::layout::Rect;

/// A set of cells, kept as a list of rects that may overlap.
///
/// Overlap only costs repeated work, never correctness: every operation here
/// is idempotent per cell.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Region {
    rects: Vec<Rect>,
}

impl Region {
    pub fn add(&mut self, mut rect: Rect) {
        if rect.is_empty() || self.rects.iter().any(|r| contains(*r, rect)) {
            return;
        }
        // Grow into neighbours that form a rect with it, so a run of damaged
        // rows stays one rect.
        while let Some(i) = self.rects.iter().position(|r| joins(*r, rect)) {
            rect = rect.union(self.rects.swap_remove(i));
        }
        self.rects.retain(|r| !contains(rect, *r));
        self.rects.push(rect);
    }

    /// Whether any cell of `rect` is in the region.
    pub fn intersects(&self, rect: Rect) -> bool {
        self.rects.iter().any(|r| r.intersects(rect))
    }

    /// Whether any cell is in both regions.
    pub fn overlaps(&self, other: &Region) -> bool {
        other.rects.iter().any(|r| self.intersects(*r))
    }

    pub fn clear(&mut self) {
        self.rects.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.rects.is_empty()
    }

    pub fn rects(&self) -> &[Rect] {
        &self.rects
    }

    /// Replaces `out` with the part of this region inside `area`.
    pub fn clip_into(&self, area: Rect, out: &mut Region) {
        out.clear();
        for r in &self.rects {
            out.add(r.intersection(area));
        }
    }

    /// The smallest rect holding every cell of the region.
    pub fn bounds(&self) -> Rect {
        self.rects
            .iter()
            .copied()
            .reduce(|a, b| a.union(b))
            .unwrap_or_default()
    }

    /// Whether one rect of the region holds all of `rect`. Conservative: a
    /// rect covered only by several pieces answers false.
    pub fn covers(&self, rect: Rect) -> bool {
        self.rects.iter().any(|r| contains(*r, rect))
    }

    /// Whether row `y` has any cell in the region.
    pub fn has_row(&self, y: u16) -> bool {
        self.rects.iter().any(|r| r.y <= y && y < r.bottom())
    }
}

fn contains(outer: Rect, inner: Rect) -> bool {
    outer.x <= inner.x
        && outer.y <= inner.y
        && inner.right() <= outer.right()
        && inner.bottom() <= outer.bottom()
}

/// Whether two rects touch or overlap along a full shared edge, so their
/// union holds no extra cells.
fn joins(a: Rect, b: Rect) -> bool {
    let columns = a.x == b.x && a.width == b.width && a.y <= b.bottom() && b.y <= a.bottom();
    let rows = a.y == b.y && a.height == b.height && a.x <= b.right() && b.x <= a.right();
    columns || rows
}
