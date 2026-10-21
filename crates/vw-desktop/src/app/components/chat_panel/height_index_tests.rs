//! 验证聊天消息高度索引。
//! 测试覆盖高度累计、范围查询和更新边界，保护滚动定位稳定性。

use super::height_index::{CHAT_LIST_VERTICAL_PADDING, CHAT_MESSAGE_GAP};
use super::height_index::{CHAT_VIRTUALIZATION_OVERSCAN_PX, ChatHeightIndex};

fn assert_window_is_consistent(
    index: &ChatHeightIndex,
    window: &super::height_index::ChatHeightWindow,
) {
    assert!(window.render_start_idx <= window.visible_start_idx);
    assert!(window.visible_start_idx < window.visible_end_idx);
    assert!(window.visible_end_idx <= window.render_end_idx);
    assert!(window.render_end_idx <= index.len());
    assert!(window.top_spacer_h >= 0.0);
    assert!(window.bottom_spacer_h >= 0.0);

    let rendered_total = window.top_spacer_h + window.bottom_spacer_h;
    assert!(rendered_total <= index.total_height() + 0.01);
}

#[test]
fn compute_window_stays_bounded_for_extreme_scroll_offsets() {
    let heights: Vec<f32> = (0..96).map(|idx| 48.0 + (idx % 5) as f32 * 7.0).collect();
    let index = ChatHeightIndex::from_heights(&heights);

    let top_window = index.compute_window(-1.0, 180.0, CHAT_VIRTUALIZATION_OVERSCAN_PX);
    let bottom_window = index.compute_window(2.0, 180.0, CHAT_VIRTUALIZATION_OVERSCAN_PX);

    assert_eq!(top_window.visible_start_idx, 0);
    assert!(bottom_window.visible_end_idx <= heights.len());
    assert!(bottom_window.visible_start_idx < heights.len());
    assert_window_is_consistent(&index, &top_window);
    assert_window_is_consistent(&index, &bottom_window);
}

#[test]
fn compute_window_keeps_chunk_boundaries_stable_near_edges() {
    let heights: Vec<f32> = (0..96)
        .map(|idx| if idx % 32 == 31 { 180.0 } else { 36.0 + (idx % 3) as f32 * 8.0 })
        .collect();
    let index = ChatHeightIndex::from_heights(&heights);

    let near_top = index.compute_window(0.02, 220.0, 0.0);
    let near_bottom = index.compute_window(0.98, 220.0, 0.0);

    let top_chunk_start = crate::app::session::chat_ui_chunk_start_idx(near_top.visible_start_idx);
    let bottom_chunk_start =
        crate::app::session::chat_ui_chunk_start_idx(near_bottom.visible_start_idx);
    let (top_chunk_start, top_chunk_end) =
        crate::app::session::chat_ui_chunk_bounds(heights.len(), top_chunk_start);
    let (bottom_chunk_start, bottom_chunk_end) =
        crate::app::session::chat_ui_chunk_bounds(heights.len(), bottom_chunk_start);

    assert!(near_top.visible_start_idx >= top_chunk_start);
    assert!(near_top.visible_start_idx < top_chunk_end);
    assert!(near_bottom.visible_start_idx >= bottom_chunk_start);
    assert!(near_bottom.visible_start_idx < bottom_chunk_end);
    assert_window_is_consistent(&index, &near_top);
    assert_window_is_consistent(&index, &near_bottom);
}

#[test]
fn empty_and_cleared_index_have_zero_height_and_empty_window() {
    let mut index = ChatHeightIndex::from_heights(&[24.0, 48.0]);
    index.clear();

    let window = index.compute_window(0.5, 120.0, 10.0);

    assert_eq!(index.len(), 0);
    assert_eq!(index.total_height(), 0.0);
    assert_eq!(index.find_start_by_scroll_top(100.0), 0);
    assert_eq!(window.render_start_idx, 0);
    assert_eq!(window.render_end_idx, 0);
}

#[test]
fn negative_heights_are_clamped_and_updates_ignore_out_of_range() {
    let mut index = ChatHeightIndex::from_heights(&[-20.0, 10.0]);
    let initial_total = index.total_height();

    index.update_height(99, 500.0);
    assert_eq!(index.total_height(), initial_total);

    index.update_height(0, 30.0);
    assert!(index.total_height() > initial_total);
    assert_eq!(index.find_start_by_scroll_top(-1.0), 0);
}

#[test]
fn scroll_top_for_item_uses_slot_prefixes_and_total_height_removes_trailing_gap() {
    let index = ChatHeightIndex::from_heights(&[10.0, 20.0, 30.0]);

    assert_eq!(index.scroll_top_for_item(0), 0.0);
    assert_eq!(index.scroll_top_for_item(1), 10.0 + CHAT_MESSAGE_GAP);
    assert_eq!(index.scroll_top_for_item(3), 10.0 + 20.0 + 30.0 + CHAT_MESSAGE_GAP * 3.0);
    assert_eq!(
        index.total_height(),
        10.0 + 20.0 + 30.0 + CHAT_MESSAGE_GAP * 2.0 + CHAT_LIST_VERTICAL_PADDING
    );
}

#[test]
fn zero_viewport_returns_full_window() {
    let index = ChatHeightIndex::from_heights(&[20.0, 30.0, 40.0]);

    let window = index.compute_window(0.5, 0.0, 0.0);

    assert_eq!(window.render_start_idx, 0);
    assert_eq!(window.render_end_idx, 3);
    assert_eq!(window.visible_start_idx, 0);
    assert_eq!(window.visible_end_idx, 3);
    assert_eq!(window.top_spacer_h, 0.0);
    assert_eq!(window.bottom_spacer_h, 0.0);
}

#[test]
fn equivalent_update_is_a_noop_and_scroll_past_total_returns_len() {
    let mut index = ChatHeightIndex::from_heights(&[-10.0, 24.0]);
    let initial_total = index.total_height();

    index.update_height(0, -50.0);

    assert_eq!(index.total_height(), initial_total);
    assert_eq!(index.find_start_by_scroll_top(initial_total + 100.0), index.len());
}

#[test]
fn chat_height_index_includes_trailing_runway_in_bottom_spacer() {
    let heights = [80.0, 100.0, 40.0];
    let mut index = ChatHeightIndex::from_heights(&heights);
    index.set_trailing_runway(160.0);

    let window = index.compute_window(1.0, 180.0, 0.0);
    let rendered_height =
        heights[window.render_start_idx..window.render_end_idx].iter().sum::<f32>()
            + CHAT_MESSAGE_GAP
                * window.render_end_idx.saturating_sub(window.render_start_idx + 1) as f32;

    assert_eq!(index.trailing_runway(), 160.0);
    assert!(window.bottom_spacer_h >= index.trailing_runway());
    assert!(
        (window.top_spacer_h + rendered_height + window.bottom_spacer_h - index.total_height())
            .abs()
            < 0.01
    );
}

#[test]
fn height_index_updates_one_point() {
    let heights = vec![40.0; 10_000];
    let mut index = ChatHeightIndex::from_heights(&heights);
    let following_item_top = index.scroll_top_for_item(5_001);

    index.update_height(5_000, 64.0);

    assert_eq!(index.scroll_top_for_item(5_000), 5_000.0 * (40.0 + CHAT_MESSAGE_GAP));
    assert_eq!(index.scroll_top_for_item(5_001), following_item_top + 24.0);
}

#[test]
fn height_index_appends_without_changing_existing_prefixes() {
    let mut index = ChatHeightIndex::from_heights(&[40.0, 60.0]);
    let existing_prefix = index.scroll_top_for_item(2);

    index.append_heights(&[80.0, 100.0]);

    assert_eq!(index.len(), 4);
    assert_eq!(index.scroll_top_for_item(2), existing_prefix);
    assert_eq!(index.scroll_top_for_item(4), 40.0 + 60.0 + 80.0 + 100.0 + CHAT_MESSAGE_GAP * 4.0);
}
