use libghostty_vt::style::{self, PaletteIndex, RgbColor, StyleColor, Underline};
use ratatui::style::{Color, Modifier, Style};
use ratatui_ghostty::convert::to_ratatui;

fn default_palette() -> [RgbColor; 256] {
    [RgbColor { r: 0, g: 0, b: 0 }; 256]
}

fn make_style() -> style::Style {
    style::Style::default()
}

#[test]
fn rgb_color_converts() {
    let c = RgbColor {
        r: 255,
        g: 128,
        b: 0,
    };
    assert_eq!(to_ratatui::rgb_color(c), Color::Rgb(255, 128, 0));
}

#[test]
fn rgb_color_black() {
    let c = RgbColor { r: 0, g: 0, b: 0 };
    assert_eq!(to_ratatui::rgb_color(c), Color::Rgb(0, 0, 0));
}

#[test]
fn rgb_color_white() {
    let c = RgbColor {
        r: 255,
        g: 255,
        b: 255,
    };
    assert_eq!(to_ratatui::rgb_color(c), Color::Rgb(255, 255, 255));
}

#[test]
fn style_bold() {
    let mut s = make_style();
    s.bold = true;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn style_italic() {
    let mut s = make_style();
    s.italic = true;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::ITALIC));
}

#[test]
fn style_faint() {
    let mut s = make_style();
    s.faint = true;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::DIM));
}

#[test]
fn style_blink() {
    let mut s = make_style();
    s.blink = true;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::SLOW_BLINK));
}

#[test]
fn style_inverse() {
    let mut s = make_style();
    s.inverse = true;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::REVERSED));
}

#[test]
fn style_invisible() {
    let mut s = make_style();
    s.invisible = true;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::HIDDEN));
}

#[test]
fn style_strikethrough() {
    let mut s = make_style();
    s.strikethrough = true;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::CROSSED_OUT));
}

#[test]
fn style_underline_single() {
    let mut s = make_style();
    s.underline = Underline::Single;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn style_underline_double() {
    let mut s = make_style();
    s.underline = Underline::Double;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn style_underline_curly() {
    let mut s = make_style();
    s.underline = Underline::Curly;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn style_underline_dotted() {
    let mut s = make_style();
    s.underline = Underline::Dotted;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn style_underline_dashed() {
    let mut s = make_style();
    s.underline = Underline::Dashed;
    let result = to_ratatui::style(&s, &default_palette());
    assert!(result.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn style_underline_none() {
    let s = make_style();
    let result = to_ratatui::style(&s, &default_palette());
    assert!(!result.add_modifier.contains(Modifier::UNDERLINED));
}

#[test]
fn style_fg_rgb() {
    let mut s = make_style();
    s.fg_color = StyleColor::Rgb(RgbColor {
        r: 100,
        g: 200,
        b: 50,
    });
    let result = to_ratatui::style(&s, &default_palette());
    assert_eq!(result.fg, Some(Color::Rgb(100, 200, 50)));
}

#[test]
fn style_bg_rgb() {
    let mut s = make_style();
    s.bg_color = StyleColor::Rgb(RgbColor {
        r: 10,
        g: 20,
        b: 30,
    });
    let result = to_ratatui::style(&s, &default_palette());
    assert_eq!(result.bg, Some(Color::Rgb(10, 20, 30)));
}

#[test]
fn style_fg_none() {
    let s = make_style();
    let result = to_ratatui::style(&s, &default_palette());
    assert_eq!(result.fg, None);
}

#[test]
fn style_bg_none() {
    let s = make_style();
    let result = to_ratatui::style(&s, &default_palette());
    assert_eq!(result.bg, None);
}

#[test]
fn style_fg_palette_stays_indexed() {
    let mut palette = default_palette();
    palette[1] = RgbColor { r: 170, g: 0, b: 0 };
    let mut s = make_style();
    s.fg_color = StyleColor::Palette(PaletteIndex(1));
    let result = to_ratatui::style(&s, &palette);
    assert_eq!(result.fg, Some(Color::Indexed(1)));
}

#[test]
fn style_bg_palette_stays_indexed() {
    let mut palette = default_palette();
    palette[255] = RgbColor {
        r: 238,
        g: 238,
        b: 238,
    };
    let mut s = make_style();
    s.bg_color = StyleColor::Palette(PaletteIndex(255));
    let result = to_ratatui::style(&s, &palette);
    assert_eq!(result.bg, Some(Color::Indexed(255)));
}

#[test]
fn style_default_has_no_modifiers() {
    let s = make_style();
    let result = to_ratatui::style(&s, &default_palette());
    assert_eq!(result, Style::default());
}
