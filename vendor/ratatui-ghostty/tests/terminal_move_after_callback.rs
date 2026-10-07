/// Regression test / bug reproducer for:
///   libghostty-vt: moving a Terminal after registering callbacks causes
///   a segfault when the callback is later invoked.
///
/// # Root cause
///
/// `Terminal::on_*` stores a raw pointer to `self.vtable` (a field *inside*
/// the `Terminal` struct) in the C library as userdata.  If the `Terminal` is
/// later moved to a different memory address the C library still holds the old
/// pointer.  The next VT sequence that fires a callback dereferences that stale
/// pointer → SIGSEGV.
///
/// # How to trigger a reliable move
///
/// Rust's NRVO (named return-value optimisation) often elides copies when a
/// value is returned from a function, so a plain `fn build() -> Terminal` may
/// not move anything at the machine-code level.  The most reliable way to
/// guarantee a real relocation is to `push` the terminal into a `Vec` that was
/// allocated at full capacity and then call `reserve` — this forces the Vec to
/// reallocate and `memcpy` every element to a new buffer, including the vtable
/// field, while the C library retains the old address.
///
/// # Trigger sequence
///
/// DECRQM (`CSI ? 7 $ p`) asks the terminal whether DECAWM (wrap mode) is set.
/// libghostty-vt responds by calling `on_pty_write` with the answer — the same
/// path that crashes in practice when `hx` sends an XTVERSION query (`CSI > q`)
/// from inside a spawned pane.
///
/// # Workaround
///
/// Heap-allocate the Terminal (`Box<Terminal>`) before registering any
/// callbacks.  Moving the `Box` does not move the `Terminal` itself, so the
/// vtable pointer stored in the C library stays valid.
use libghostty_vt::terminal::Terminal;

fn new_terminal() -> Terminal<'static, 'static> {
    let mut terminal = Terminal::new(80, 24).unwrap();
    terminal.set_scrollback_max_lines(Some(0)).unwrap();
    terminal
}

/// Reproduce the crash: register a callback, then move the Terminal via a Vec
/// reallocation, then trigger the callback.
#[test]
#[ignore = "crashes the process (SIGSEGV) — run with `cargo test -- --ignored` to confirm the bug"]
fn segfaults_after_vec_realloc_move() {
    let mut v: Vec<Terminal<'static, 'static>> = Vec::with_capacity(1);

    let mut t = new_terminal();
    t.on_pty_write(|_term, _data| {}).unwrap();
    v.push(t); // terminal now at heap address A; vtable ptr stored in C → A

    // Force reallocation: all elements are memcpy'd to a new buffer at address B.
    // The terminal (and its vtable field) is now at B, but the C library still
    // has the pointer to A.
    v.reserve(64);

    // DECRQM triggers on_pty_write — reads vtable through stale pointer → SIGSEGV.
    v[0].vt_write(b"\x1b[?7$p");
}

/// Confirm the workaround: boxing the terminal before registering callbacks
/// means the vtable pointer stored in C always tracks the heap address, which
/// never changes regardless of how the Box itself is moved.
#[test]
fn no_crash_when_boxed_before_callbacks() {
    let mut v: Vec<Box<Terminal<'static, 'static>>> = Vec::with_capacity(1);

    let mut t = Box::new(new_terminal());
    t.on_pty_write(|_term, _data| {}).unwrap();
    v.push(t); // Box moved; Terminal on heap stays at same address

    v.reserve(64); // Box pointers are memcpy'd; Terminal itself does not move

    v[0].vt_write(b"\x1b[?7$p"); // callback fires correctly, no crash
}
