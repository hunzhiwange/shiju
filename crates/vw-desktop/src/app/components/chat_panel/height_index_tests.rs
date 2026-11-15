//! 验证聊天消息高度索引。
//! 测试覆盖高度累计、范围查询和更新边界，保护滚动定位稳定性。

use super::height_index::ChatHeightIndex;

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
