use std::cell::{Cell, Ref, RefCell};
use std::rc::Rc;

use ratatui::backend::Backend;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::widgets::Widget;
use reactive_scope::{ReadSignal, Scope, Signal, create_effect, create_scope, create_signal};

use crate::compositor::{Compositor, Layer, Stats};
use crate::region::Region;

/// What an app hands the renderer: its layers bottom to top, and where the
/// cursor goes.
pub struct View {
    pub layers: Vec<Box<dyn Layer>>,
    pub cursor: Box<dyn FnMut() -> Option<Position>>,
}

/// A reactive app over a [`Compositor`]. Layers recompute when the signals
/// they read change; a recomputed value equal to the last one damages
/// nothing.
pub struct Ui<B: Backend> {
    compositor: Compositor<B>,
    size: Signal<Rect>,
    cursor: Memo<Option<Position>>,
    _scope: Scope,
}

impl<B: Backend> Ui<B> {
    pub fn new(backend: B, size: Rect, app: impl FnOnce(ReadSignal<Rect>) -> View) -> Self {
        let scope = create_scope();
        let (size_signal, layers, cursor) = scope.run_in(|| {
            let size_signal = create_signal(size);
            let View { layers, cursor } = app(size_signal.readonly());
            (size_signal, layers, Memo::new(cursor))
        });
        Self {
            compositor: Compositor::new(backend, size, layers),
            size: size_signal,
            cursor,
            _scope: scope,
        }
    }

    pub fn resize(&mut self, size: Rect) -> Result<(), B::Error> {
        self.size.set(size);
        self.compositor.resize(size)
    }

    /// Runs pending effects, then writes whatever changed. A draw with no
    /// changes writes nothing.
    pub fn draw(&mut self) -> Result<Stats, B::Error> {
        reactive_scope::flush();
        let cursor = *self.cursor.get();
        self.compositor.draw(cursor)
    }

    pub fn compositor(&mut self) -> &mut Compositor<B> {
        &mut self.compositor
    }

    pub fn cursor(&self) -> Option<Position> {
        *self.cursor.get()
    }
}

/// A value recomputed whenever the signals its function reads change, with
/// a counter that moves only when the new value counts as a change.
struct Memo<T> {
    value: Rc<RefCell<Option<T>>>,
    changes: Rc<Cell<u64>>,
}

impl<T: PartialEq + 'static> Memo<T> {
    fn new(f: impl FnMut() -> T + 'static) -> Self {
        Self::with(f, |old, new| old != new)
    }
}

impl<T: 'static> Memo<T> {
    fn with(mut f: impl FnMut() -> T + 'static, changed: fn(&T, &T) -> bool) -> Self {
        let value: Rc<RefCell<Option<T>>> = Rc::default();
        let changes = Rc::new(Cell::new(0));
        create_effect({
            let (value, changes) = (value.clone(), changes.clone());
            move || {
                let new = f();
                let mut value = value.borrow_mut();
                if value.as_ref().is_none_or(|old| changed(old, &new)) {
                    *value = Some(new);
                    changes.set(changes.get() + 1);
                }
            }
        });
        Self { value, changes }
    }

    /// The effect runs once on creation, so there is always a value.
    fn get(&self) -> Ref<'_, T> {
        Ref::map(self.value.borrow(), |v| {
            v.as_ref().expect("memo computed on creation")
        })
    }
}

/// A layer that paints one widget value over its whole area. The widget is
/// rebuilt when the signals `view` reads change, and damages its area only
/// when the rebuilt value differs.
pub fn widget<W>(
    area: impl FnMut() -> Option<Rect> + 'static,
    view: impl FnMut() -> W + 'static,
) -> Box<dyn Layer>
where
    W: PartialEq + 'static,
    for<'a> &'a W: Widget,
{
    Box::new(WidgetLayer {
        area: Memo::new(area),
        view: Memo::new(view),
        painted: 0,
        footprint: RefCell::default(),
    })
}

struct WidgetLayer<W> {
    area: Memo<Option<Rect>>,
    view: Memo<W>,
    painted: u64,
    /// Cells the current value writes, with the value's change count and
    /// area it was measured for.
    footprint: RefCell<Option<(u64, Rect, Region)>>,
}

impl<W> Layer for WidgetLayer<W>
where
    W: PartialEq + 'static,
    for<'a> &'a W: Widget,
{
    fn area(&self) -> Option<Rect> {
        *self.area.get()
    }

    fn damage(&mut self, area: Rect, out: &mut Region) {
        let changes = self.view.changes.get();
        if changes != self.painted {
            self.painted = changes;
            out.add(area);
        }
    }

    fn touches(&self, area: Rect, damage: &Region) -> bool {
        let changes = self.view.changes.get();
        let mut footprint = self.footprint.borrow_mut();
        if !footprint
            .as_ref()
            .is_some_and(|(c, a, _)| (*c, *a) == (changes, area))
        {
            *footprint = Some((changes, area, measure(&*self.view.get(), area)));
        }
        footprint
            .as_ref()
            .is_some_and(|(_, _, cells)| cells.overlaps(damage))
    }

    fn paint(&self, area: Rect, _want: &Region, buf: &mut Buffer) {
        (&*self.view.get()).render(area, buf);
    }
}

/// A layer of independent rows, such as a terminal pane. Each row is
/// compared with the one last painted, and only rows that differ are
/// damaged and repainted. `paint_row` gets a one-row rect, must paint only
/// that row, and must not touch signals.
pub fn rows<T>(
    area: impl FnMut() -> Option<Rect> + 'static,
    rows: ReadSignal<Vec<T>>,
    paint_row: impl Fn(&T, Rect, &mut Buffer) + 'static,
) -> Box<dyn Layer>
where
    T: PartialEq + Clone + 'static,
{
    let changes = Rc::new(Cell::new(0));
    create_effect({
        let changes = changes.clone();
        move || {
            rows.track();
            changes.set(changes.get() + 1);
        }
    });
    Box::new(RowsLayer {
        area: Memo::new(area),
        rows,
        changes,
        seen: 0,
        painted: Vec::new(),
        paint_row: Box::new(paint_row),
    })
}

type PaintRow<T> = Box<dyn Fn(&T, Rect, &mut Buffer)>;

struct RowsLayer<T> {
    area: Memo<Option<Rect>>,
    rows: ReadSignal<Vec<T>>,
    changes: Rc<Cell<u64>>,
    seen: u64,
    painted: Vec<T>,
    paint_row: PaintRow<T>,
}

impl<T: PartialEq + Clone + 'static> Layer for RowsLayer<T> {
    fn area(&self) -> Option<Rect> {
        *self.area.get()
    }

    fn damage(&mut self, area: Rect, out: &mut Region) {
        if self.changes.get() == self.seen {
            return;
        }
        self.seen = self.changes.get();
        let painted = &mut self.painted;
        self.rows.with_untracked(|rows| {
            for (i, row) in rows.iter().enumerate() {
                match painted.get_mut(i) {
                    Some(p) if p == row => continue,
                    Some(p) => p.clone_from(row),
                    None => painted.push(row.clone()),
                }
                damage_row(area, i, out);
            }
            for i in rows.len()..painted.len() {
                damage_row(area, i, out);
            }
            painted.truncate(rows.len());
        });
    }

    fn extent(&self, area: Rect, want: Rect) -> Rect {
        Rect::new(area.x, want.y, area.width, want.height)
    }

    fn paint(&self, area: Rect, want: &Region, buf: &mut Buffer) {
        self.rows.with_untracked(|rows| {
            for (i, row) in rows.iter().enumerate().take(area.height as usize) {
                let y = area.y + i as u16;
                if want.has_row(y) {
                    (self.paint_row)(row, Rect::new(area.x, y, area.width, 1), buf);
                }
            }
        });
    }
}

fn damage_row(area: Rect, i: usize, out: &mut Region) {
    if i < area.height as usize {
        out.add(Rect::new(area.x, area.y + i as u16, area.width, 1));
    }
}

/// The cells a widget writes, found by rendering it over two backgrounds
/// that differ in every field. Any write of a field changes at least one of
/// them, so a cell both leave alone is one the widget never touches.
fn measure<W>(view: &W, area: Rect) -> Region
where
    for<'a> &'a W: Widget,
{
    let sentinel = |symbol, fg, bg, modifier, diff_option, skip| {
        let mut cell = ratatui::buffer::Cell::new(symbol);
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
    use ratatui::buffer::CellDiffOption;
    use ratatui::style::{Color, Modifier};
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
    view.render(area, &mut buf_a);
    view.render(area, &mut buf_b);
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
