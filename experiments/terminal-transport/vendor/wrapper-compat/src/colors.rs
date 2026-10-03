//! Query the host terminal's color scheme via OSC escape sequences.
//!
//! Sends OSC 10 (foreground), OSC 11 (background), and OSC 4 (indexed
//! palette) queries to `/dev/tty` and parses the `rgb:` responses.
//!
//! Must be called while the terminal is in raw mode so responses are
//! returned immediately without line-buffering.

use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::os::unix::io::AsRawFd;
use std::time::{Duration, Instant};

/// An 8-bit RGB color.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

/// Color scheme reported by the host terminal.
#[derive(Debug, Clone)]
pub struct HostColors {
    pub foreground: Rgb,
    pub background: Rgb,
    /// Indexed palette entries `[0, palette_count)` as requested.
    pub palette: Vec<Rgb>,
}

impl Default for HostColors {
    fn default() -> Self {
        Self {
            foreground: Rgb {
                r: 0xFF,
                g: 0xFF,
                b: 0xFF,
            },
            background: Rgb {
                r: 0x00,
                g: 0x00,
                b: 0x00,
            },
            palette: Vec::new(),
        }
    }
}

/// Query the host terminal's color scheme.
///
/// Queries OSC 10/11 (fg/bg) plus the first `palette_count` OSC 4 palette
/// entries.  Any entries the terminal does not respond to within `timeout`
/// are left as `Rgb { r: 0, g: 0, b: 0 }`.
///
/// Call this while the terminal is in raw mode (e.g. after
/// `crossterm::terminal::enable_raw_mode()`).
pub fn query_colors(palette_count: u16, timeout: Duration) -> io::Result<HostColors> {
    let mut tty = OpenOptions::new().read(true).write(true).open("/dev/tty")?;
    send_queries(&mut tty, palette_count)?;
    read_responses(&mut tty, palette_count, timeout)
}

fn send_queries(tty: &mut impl Write, palette_count: u16) -> io::Result<()> {
    let mut buf = Vec::new();
    buf.extend_from_slice(b"\x1b]10;?\x1b\\");
    buf.extend_from_slice(b"\x1b]11;?\x1b\\");
    for i in 0..palette_count {
        write!(buf, "\x1b]4;{i};?\x1b\\").unwrap();
    }
    tty.write_all(&buf)?;
    tty.flush()
}

fn read_responses(
    tty: &mut (impl Read + AsRawFd),
    palette_count: u16,
    timeout: Duration,
) -> io::Result<HostColors> {
    let mut fg = Rgb::default();
    let mut bg = Rgb::default();
    let mut palette = vec![Rgb::default(); palette_count as usize];
    let mut received = 0;
    let expected = 2 + palette_count as usize;
    let mut buf = Vec::new();
    let mut tmp = [0u8; 1024];
    let deadline = Instant::now() + timeout;

    while received < expected {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() || !poll_read(tty.as_raw_fd(), remaining)? {
            break;
        }
        match tty.read(&mut tmp) {
            Ok(0) | Err(_) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
        }
        received += consume_osc_responses(&mut buf, &mut fg, &mut bg, &mut palette);
    }

    Ok(HostColors {
        foreground: fg,
        background: bg,
        palette,
    })
}

/// Wait up to `timeout` for the fd to become readable.  Returns `true` if
/// data is available, `false` on timeout.
fn poll_read(fd: i32, timeout: Duration) -> io::Result<bool> {
    let ms = timeout.as_millis().min(i32::MAX as u128) as i32;
    let mut pfd = libc::pollfd {
        fd,
        events: libc::POLLIN,
        revents: 0,
    };
    match unsafe { libc::poll(&raw mut pfd, 1, ms) } {
        -1 => Err(io::Error::last_os_error()),
        0 => Ok(false),
        _ => Ok(true),
    }
}

/// Parse all complete OSC sequences out of `buf`, removing them.
/// Returns the number of recognised color responses consumed.
fn consume_osc_responses(
    buf: &mut Vec<u8>,
    fg: &mut Rgb,
    bg: &mut Rgb,
    palette: &mut [Rgb],
) -> usize {
    let mut count = 0;
    loop {
        // Locate ESC ]
        let Some(start) = buf.windows(2).position(|w| w == b"\x1b]") else {
            buf.clear();
            break;
        };
        let content_start = start + 2;

        // Locate terminator: BEL (0x07) or ST (ESC \)
        let term = buf[content_start..].iter().enumerate().find_map(|(i, &b)| {
            if b == 0x07 {
                Some((content_start + i, 1usize))
            } else if b == 0x1b && buf.get(content_start + i + 1) == Some(&b'\\') {
                Some((content_start + i, 2))
            } else {
                None
            }
        });

        let Some((term_start, term_len)) = term else {
            // Incomplete sequence — keep everything from ESC ] onward.
            buf.drain(..start);
            break;
        };

        let content = buf[content_start..term_start].to_vec();
        if parse_osc(&content, fg, bg, palette) {
            count += 1;
        }
        buf.drain(..term_start + term_len);
    }
    count
}

fn parse_osc(content: &[u8], fg: &mut Rgb, bg: &mut Rgb, palette: &mut [Rgb]) -> bool {
    parse_osc_inner(content, fg, bg, palette).is_some()
}

fn parse_osc_inner(content: &[u8], fg: &mut Rgb, bg: &mut Rgb, palette: &mut [Rgb]) -> Option<()> {
    let s = std::str::from_utf8(content).ok()?;
    let (num_str, rest) = s.split_once(';')?;
    match num_str.parse::<u16>().ok()? {
        10 => *fg = parse_rgb(rest)?,
        11 => *bg = parse_rgb(rest)?,
        4 => {
            let (idx_str, color_str) = rest.split_once(';')?;
            let idx: usize = idx_str.parse().ok()?;
            if idx < palette.len() {
                palette[idx] = parse_rgb(color_str)?;
            }
        }
        _ => return None,
    }
    Some(())
}

/// Parse `rgb:RR/GG/BB` or `rgb:RRRR/GGGG/BBBB` into an `Rgb`.
fn parse_rgb(s: &str) -> Option<Rgb> {
    let hex = s.strip_prefix("rgb:")?;
    let mut parts = hex.splitn(3, '/');
    Some(Rgb {
        r: parse_channel(parts.next()?)?,
        g: parse_channel(parts.next()?)?,
        b: parse_channel(parts.next()?)?,
    })
}

/// Normalise a 1–4 hex-digit channel value to 8-bit.
fn parse_channel(s: &str) -> Option<u8> {
    let val = u32::from_str_radix(s.trim(), 16).ok()?;
    Some(match s.trim().len() {
        1 => (val * 17) as u8,
        2 => val as u8,
        3 => (val >> 4) as u8,
        4 => (val >> 8) as u8,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_rgb_4digit() {
        assert_eq!(
            parse_rgb("rgb:FFFF/0000/8080"),
            Some(Rgb {
                r: 0xFF,
                g: 0x00,
                b: 0x80
            })
        );
    }

    #[test]
    fn parse_rgb_2digit() {
        assert_eq!(
            parse_rgb("rgb:FF/00/80"),
            Some(Rgb {
                r: 0xFF,
                g: 0x00,
                b: 0x80
            })
        );
    }

    #[test]
    fn consume_fg_bg() {
        let mut buf = b"\x1b]10;rgb:FFFF/FFFF/FFFF\x1b\\\x1b]11;rgb:0000/0000/0000\x1b\\".to_vec();
        let mut fg = Rgb::default();
        let mut bg = Rgb::default();
        let mut palette = [];
        let n = consume_osc_responses(&mut buf, &mut fg, &mut bg, &mut palette);
        assert_eq!(n, 2);
        assert_eq!(
            fg,
            Rgb {
                r: 255,
                g: 255,
                b: 255
            }
        );
        assert_eq!(bg, Rgb { r: 0, g: 0, b: 0 });
        assert!(buf.is_empty());
    }

    #[test]
    fn consume_palette_bel() {
        let mut buf = b"\x1b]4;1;rgb:AA/11/22\x07".to_vec();
        let mut fg = Rgb::default();
        let mut bg = Rgb::default();
        let mut palette = [Rgb::default(); 2];
        let n = consume_osc_responses(&mut buf, &mut fg, &mut bg, &mut palette);
        assert_eq!(n, 1);
        assert_eq!(
            palette[1],
            Rgb {
                r: 0xAA,
                g: 0x11,
                b: 0x22
            }
        );
    }

    #[test]
    fn consume_partial_sequence_preserved() {
        // Incomplete sequence should stay in buffer
        let mut buf = b"\x1b]10;rgb:FF".to_vec();
        let mut fg = Rgb::default();
        let mut bg = Rgb::default();
        let mut palette = [];
        let n = consume_osc_responses(&mut buf, &mut fg, &mut bg, &mut palette);
        assert_eq!(n, 0);
        assert!(!buf.is_empty());
    }
}
