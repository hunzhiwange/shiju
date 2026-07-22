//! 验证聊天虚拟列表领域模型。
//! 测试使用纯高度数据保护滚动锚点、尾部 sticky 和窗口 spacer 自洽。

use super::height_index::CHAT_MESSAGE_GAP;
use super::virtualizer::{ChatScrollAnchor, ChatScrollMode, ChatVirtualizer};

#[test]
fn chat_virtualizer_preserves_anchor_when_heights_change() {
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0, 120.0, 140.0, 90.0]);
    virtualizer.set_viewport_height(180.0);
    virtualizer.preserve_anchor(ChatScrollAnchor::new(2, 24.0));

    let before = virtualizer.scroll_top_px();
    virtualizer.apply_measured_height(0, 160.0);

    assert_eq!(virtualizer.scroll_mode(), ChatScrollMode::Anchored);
    assert_eq!(virtualizer.scroll_top_px(), before + 80.0);
    assert_eq!(virtualizer.anchor(), Some(ChatScrollAnchor::new(2, 24.0)));
}

#[test]
fn chat_virtualizer_tail_sticky_survives_height_growth() {
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[60.0, 70.0, 80.0, 90.0]);
    virtualizer.set_viewport_height(140.0);
    virtualizer.snap_to_tail();
    let first_tail = virtualizer.scroll_top_px();

    virtualizer.apply_measured_height(3, 180.0);

    assert_eq!(virtualizer.scroll_mode(), ChatScrollMode::TailSticky);
    assert!(virtualizer.scroll_top_px() > first_tail);
    assert_eq!(virtualizer.scroll_top_px(), virtualizer.max_scroll_top_px());
}

#[test]
fn chat_virtualizer_window_spacers_match_total_height() {
    let heights: Vec<f32> = (0..40).map(|idx| 48.0 + (idx % 4) as f32 * 12.0).collect();
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&heights);
    virtualizer.set_viewport_height(220.0);
    virtualizer.set_scroll_top_px(640.0);

    let window = virtualizer.compute_window(90.0);
    let rendered_height: f32 =
        heights[window.render_start_idx..window.render_end_idx].iter().sum::<f32>()
            + CHAT_MESSAGE_GAP
                * window.render_end_idx.saturating_sub(window.render_start_idx + 1) as f32;

    assert!(window.render_start_idx <= window.visible_start_idx);
    assert!(window.visible_start_idx < window.visible_end_idx);
    assert!(window.visible_end_idx <= window.render_end_idx);
    assert!(window.render_end_idx <= heights.len());
    assert_eq!(window.scroll_top_px, virtualizer.scroll_top_px());
    assert_eq!(window.relative_y, virtualizer.relative_y());
    assert!(
        (window.top_spacer_h + rendered_height + window.bottom_spacer_h
            - virtualizer.total_height_px())
        .abs()
            < 0.01
    );
}

#[test]
fn chat_virtualizer_empty_and_out_of_range_operations_preserve_default_state() {
    let mut virtualizer = ChatVirtualizer::default();

    assert!(virtualizer.is_empty());
    assert_eq!(virtualizer.len(), 0);
    assert!(virtualizer.estimated_heights().is_empty());
    assert_eq!(virtualizer.estimated_height(0), None);
    assert_eq!(virtualizer.preserve_current_anchor_around(), None);

    virtualizer.set_viewport_height(-10.0);
    virtualizer.set_scroll_top_px(25.0);
    virtualizer.apply_measured_height(0, 40.0);
    virtualizer.preserve_anchor(ChatScrollAnchor::new(0, -5.0));
    virtualizer.scroll_to_item_anchor(0, 0.5, 0.5);

    assert_eq!(virtualizer.viewport_height_px(), 0.0);
    assert_eq!(virtualizer.scroll_top_px(), 0.0);
    assert_eq!(virtualizer.total_height_px(), 0.0);
    assert_eq!(virtualizer.max_scroll_top_px(), 0.0);
    assert_eq!(virtualizer.relative_y(), 0.0);
    assert_eq!(virtualizer.scroll_mode(), ChatScrollMode::Free);
    assert_eq!(virtualizer.anchor(), None);
    assert_eq!(virtualizer.window().total_height_px, 0.0);
}

#[test]
fn chat_virtualizer_sync_len_drops_anchor_outside_truncated_range() {
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[40.0, 50.0, 60.0]);
    virtualizer.set_viewport_height(40.0);
    virtualizer.preserve_anchor(ChatScrollAnchor::new(2, 10.0));

    virtualizer.sync_len(2, 80.0);

    assert_eq!(virtualizer.estimated_heights(), &[40.0, 50.0]);
    assert_eq!(virtualizer.scroll_mode(), ChatScrollMode::Free);
    assert_eq!(virtualizer.anchor(), None);
    assert!(virtualizer.scroll_top_px() <= virtualizer.max_scroll_top_px());
}

#[test]
fn chat_virtualizer_sync_len_extends_with_non_negative_estimates() {
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[25.0]);

    virtualizer.sync_len(3, -10.0);

    assert_eq!(virtualizer.estimated_heights(), &[25.0, 0.0, 0.0]);
}
