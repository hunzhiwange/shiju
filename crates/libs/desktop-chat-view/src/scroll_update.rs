//! 虚拟列表的滚动与高度回执更新。
use crate::virtualizer::ChatVirtualizer;
use std::collections::HashMap;

pub struct ScrollUpdate {
    pub visible_range: (usize, usize),
    pub visible_range_changed: bool,
    pub viewport_changed: bool,
    pub measure_visible_messages: bool,
    pub viewport_or_offset_changed: bool,
}

pub fn apply_scroll_changed(virtualizer: &mut ChatVirtualizer, auto_scroll: &mut bool, generation: &mut u64, hold_until_ms: u64, rel_y: f32, absolute_offset_y: f32, viewport_h: f32, now_ms: fn() -> u64) -> ScrollUpdate {
    let next_offset_y = rel_y.clamp(0.0, 1.0);
    let next_viewport_h = viewport_h.max(0.0);
    let previous_window = virtualizer.window();
    let offset_changed =
        (virtualizer.scroll_top_px() - absolute_offset_y.max(0.0)).abs() > 0.5;
    let viewport_changed =
        (virtualizer.viewport_height_px() - next_viewport_h).abs() > 0.5;
    let previous_scroll_top_px = virtualizer.scroll_top_px();
    virtualizer.set_viewport_height(next_viewport_h);
    let matches_turn_focus = virtualizer.turn_focus_user_index().is_some()
        && (virtualizer.scroll_top_px() - absolute_offset_y.max(0.0)).abs() <= 1.0;
    let next_auto_scroll = desktop_session::message::chat::input::scroll::follows_tail(
        previous_scroll_top_px, next_offset_y, absolute_offset_y,
        viewport_changed, matches_turn_focus,
        now_ms() < hold_until_ms, *auto_scroll,
    );

    if !next_auto_scroll {
        *generation = generation.wrapping_add(1);
    }
    *auto_scroll = next_auto_scroll;
    if matches_turn_focus {
        // 内容增长会改变相对偏移，但绝对偏移仍然对齐本轮锚点。
        // 这类布局回执只确认视口，不释放本轮置顶状态。
    } else if next_auto_scroll {
        virtualizer.snap_to_tail();
    } else {
        // The widget's laid-out height can differ from the virtualizer's estimates.
        // Reapplying its relative offset against those estimates moves the render
        // window on each measurement, even when the user has not scrolled.
        virtualizer.set_scroll_top_px(absolute_offset_y);
    }

    let next_window = virtualizer.window();
    let visible_range = (next_window.visible_start_idx, next_window.visible_end_idx);
    let visible_range_changed =
        visible_range != (previous_window.visible_start_idx, previous_window.visible_end_idx);
    let rendered_range_changed = (next_window.render_start_idx, next_window.render_end_idx)
        != (previous_window.render_start_idx, previous_window.render_end_idx);
    ScrollUpdate {
        visible_range, visible_range_changed, viewport_changed,
        measure_visible_messages: rendered_range_changed || viewport_changed,
        viewport_or_offset_changed: offset_changed || viewport_changed,
    }
}

pub fn apply_message_measurements(virtualizer: &mut ChatVirtualizer, measured_heights: &mut HashMap<usize, f32>, message_count: usize, measurements: Vec<(usize, f32)>) -> bool {
    let mut changed = false;
    for (idx, measured) in measurements {
        if idx >= message_count || !measured.is_finite() {
            continue;
        }
        let current = measured_heights.get(&idx).copied().or_else(|| virtualizer.estimated_height(idx)).unwrap_or(0.0);
        let Some(height) = desktop_session::message::chat::input::scroll::changed_measured_height(current, measured) else { continue };
        measured_heights.insert(idx, height);
        virtualizer.apply_measured_height(idx, height);
        changed = true;
    }
    changed
}
