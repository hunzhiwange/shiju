//! 聊天滚动回执的跟随策略与局部结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatScrollUpdateOutcome {
    pub visible_range: (usize, usize),
    pub pending_chunk_starts: Vec<usize>,
    pub measure_visible_messages: bool,
    pub viewport_or_offset_changed: bool,
}

/// Relative geometry comes from the widget; absolute geometry comes from the previous window.
pub fn follows_tail(
    scroll_top_px: f32,
    next_offset_y: f32,
    absolute_offset_y: f32,
    viewport_changed: bool,
    matches_turn_focus: bool,
    within_hold_window: bool,
    auto_scroll: bool,
) -> bool {
    // A small upward movement is still an explicit exit from tail following,
    // even inside the relaxed follow threshold or a pending follow-up window.
    // Content growth changes relative position without moving the viewport up.
    let scrolled_up = !viewport_changed
        && absolute_offset_y.max(0.0) + 1.0 < scroll_top_px
        && next_offset_y < 0.999;
    let scrolled_down =
        !viewport_changed && absolute_offset_y > scroll_top_px + 1.0;
    // Use the widget's actual scroll geometry, which may differ from estimated heights.
    let remaining_px = if next_offset_y > 0.0 {
        absolute_offset_y.max(0.0) * (1.0 - next_offset_y) / next_offset_y
    } else {
        f32::INFINITY
    };
    let near_tail = next_offset_y >= 0.975 || remaining_px <= 48.0;
    if matches_turn_focus || scrolled_up {
        false
    } else if within_hold_window && auto_scroll {
        next_offset_y >= 0.82
    } else if auto_scroll {
        next_offset_y >= 0.90
    } else {
        // Resume on a downward gesture near the tail. Layout callbacks at the
        // position where the user just scrolled up must keep following paused.
        next_offset_y >= 0.999 || (scrolled_down && near_tail)
    }

}

/// Ignore non-finite and insignificant measurements before updating the transcript geometry.
pub fn changed_measured_height(current: f32, measured: f32) -> Option<f32> {
    if !measured.is_finite() {
        return None;
    }
    let height = measured.max(0.0);
    if (current - height).abs() <= 1.0 {
        None
    } else {
        Some(height)
    }
}
