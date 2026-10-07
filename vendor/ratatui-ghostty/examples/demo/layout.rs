use ratatui::layout::Rect;

/// Compute tall layout: pane 0 gets the left half, panes 1..N share the right half equally.
/// With a single pane, it gets the full area.
pub fn tall_layout(area: Rect, pane_count: usize) -> Vec<Rect> {
    if pane_count == 0 {
        return Vec::new();
    }
    if pane_count == 1 {
        return vec![area];
    }

    let left_width = area.width / 2;
    // Reserve 1 column for the vertical separator between left and right.
    let left_content_width = left_width.saturating_sub(1);
    let right_width = area.width - left_width;
    let right_x = area.x + left_width;

    let main_pane = Rect::new(area.x, area.y, left_content_width, area.height);

    let secondary_count = pane_count - 1;
    // Reserve 1 row per gap between secondary panes.
    let gaps = (secondary_count as u16).saturating_sub(1);
    let content_height = area.height.saturating_sub(gaps);
    let base_height = content_height / secondary_count as u16;
    let remainder = content_height % secondary_count as u16;

    let mut rects = Vec::with_capacity(pane_count);
    rects.push(main_pane);

    let mut y = area.y;
    for i in 0..secondary_count {
        let extra = if (i as u16) < remainder { 1 } else { 0 };
        let h = base_height + extra;
        rects.push(Rect::new(right_x, y, right_width, h));
        y += h + 1; // +1 skips the horizontal separator row
    }

    rects
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_panes() {
        assert!(tall_layout(Rect::new(0, 0, 80, 24), 0).is_empty());
    }

    #[test]
    fn single_pane_full_area() {
        let area = Rect::new(0, 0, 80, 24);
        let rects = tall_layout(area, 1);
        assert_eq!(rects, vec![area]);
    }

    #[test]
    fn two_panes_split_half() {
        let rects = tall_layout(Rect::new(0, 0, 80, 24), 2);
        assert_eq!(rects.len(), 2);
        // Left pane gives up 1 col for the vertical separator at x=39
        assert_eq!(rects[0], Rect::new(0, 0, 39, 24));
        assert_eq!(rects[1], Rect::new(40, 0, 40, 24));
    }

    #[test]
    fn three_panes() {
        let rects = tall_layout(Rect::new(0, 0, 80, 24), 3);
        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0], Rect::new(0, 0, 39, 24));
        // content_height = 24 - 1 gap = 23; 23/2=11 rem 1 → first gets 12, second 11
        assert_eq!(rects[1], Rect::new(40, 0, 40, 12));
        // gap row at y=12, pane starts at y=13
        assert_eq!(rects[2], Rect::new(40, 13, 40, 11));
    }

    #[test]
    fn four_panes_uneven_remainder() {
        let rects = tall_layout(Rect::new(0, 0, 80, 25), 4);
        assert_eq!(rects.len(), 4);
        assert_eq!(rects[0], Rect::new(0, 0, 39, 25));
        // content_height = 25 - 2 gaps = 23; 23/3=7 rem 2 → first two get 8, last 7
        assert_eq!(rects[1], Rect::new(40, 0, 40, 8));
        // gap at y=8, pane at y=9
        assert_eq!(rects[2], Rect::new(40, 9, 40, 8));
        // gap at y=17, pane at y=18
        assert_eq!(rects[3], Rect::new(40, 18, 40, 7));
    }

    #[test]
    fn odd_width() {
        let rects = tall_layout(Rect::new(0, 0, 81, 24), 2);
        // left_width=40, left_content=39, right starts at x=40 with width=41
        assert_eq!(rects[0], Rect::new(0, 0, 39, 24));
        assert_eq!(rects[1], Rect::new(40, 0, 41, 24));
    }

    #[test]
    fn nonzero_origin() {
        let rects = tall_layout(Rect::new(5, 3, 80, 24), 2);
        assert_eq!(rects[0], Rect::new(5, 3, 39, 24));
        assert_eq!(rects[1], Rect::new(45, 3, 40, 24));
    }
}
