//! 验证聊天虚拟列表领域模型。
//! 测试使用纯高度数据保护滚动锚点、尾部 sticky 和窗口 spacer 自洽。

use super::height_index::CHAT_MESSAGE_GAP;
use super::virtualizer::{
    CHAT_TURN_TOP_INSET_PX, ChatScrollAnchor, ChatScrollMode, ChatVirtualizer,
};

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

#[test]
fn chat_virtualizer_turn_focus_keeps_tail_user_below_viewport_top() {
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[120.0, 100.0, 0.0]);
    virtualizer.set_viewport_height(300.0);

    virtualizer.focus_turn_on_user(1);

    let user_top = 120.0 + CHAT_MESSAGE_GAP;
    assert_eq!(virtualizer.turn_focus_user_index(), Some(1));
    assert_eq!(
        virtualizer.anchor(),
        Some(ChatScrollAnchor { index: 1, offset_px: -CHAT_TURN_TOP_INSET_PX })
    );
    assert_eq!(virtualizer.scroll_top_px(), user_top - CHAT_TURN_TOP_INSET_PX);
    assert_eq!(virtualizer.max_scroll_top_px(), user_top - CHAT_TURN_TOP_INSET_PX);
    assert!(user_top - virtualizer.scroll_top_px() >= 48.0);
    assert!(virtualizer.trailing_runway_height_px() > 0.0);

    virtualizer.release_turn_focus();

    assert_eq!(virtualizer.turn_focus_user_index(), None);
    assert_eq!(virtualizer.trailing_runway_height_px(), 0.0);
    assert_eq!(virtualizer.scroll_mode(), ChatScrollMode::Free);
}

#[test]
fn chat_virtualizer_turn_runway_shrinks_without_moving_anchor() {
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[120.0, 100.0, 0.0]);
    virtualizer.set_viewport_height(300.0);
    virtualizer.focus_turn_on_user(1);
    let anchored_top = virtualizer.scroll_top_px();
    let initial_runway = virtualizer.trailing_runway_height_px();
    let initial_total = virtualizer.total_height_px();

    virtualizer.set_viewport_height(340.0);

    assert_eq!(virtualizer.scroll_top_px(), anchored_top);
    assert_eq!(virtualizer.trailing_runway_height_px(), initial_runway + 40.0);
    assert_eq!(virtualizer.total_height_px(), initial_total + 40.0);
    let resized_total = virtualizer.total_height_px();

    virtualizer.apply_measured_height(2, 80.0);

    assert_eq!(virtualizer.scroll_top_px(), anchored_top);
    assert_eq!(virtualizer.trailing_runway_height_px(), initial_runway - 40.0);
    assert_eq!(virtualizer.total_height_px(), resized_total);

    virtualizer.apply_measured_height(2, 240.0);

    assert_eq!(virtualizer.scroll_top_px(), anchored_top);
    assert_eq!(virtualizer.trailing_runway_height_px(), 0.0);
    assert!(virtualizer.total_height_px() > resized_total);
}

#[test]
fn chat_virtualizer_same_len_sync_does_not_rebuild() {
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&vec![40.0; 10_000]);
    virtualizer.reset_diagnostics();

    for index in 0..1_000 {
        virtualizer.sync_len(10_000, 40.0);
        virtualizer.apply_measured_height(index, 41.0);
    }

    let diagnostics = virtualizer.diagnostics();
    assert_eq!(diagnostics.full_rebuilds, 0);
    assert_eq!(diagnostics.appends, 0);
    assert_eq!(diagnostics.point_updates, 1_000);
    assert_eq!(diagnostics.prepend_rebuilds, 0);
    assert_eq!(diagnostics.truncate_rebuilds, 0);
    assert_eq!(virtualizer.len(), 10_000);
    assert_eq!(virtualizer.estimated_height(999), Some(41.0));
    assert_eq!(virtualizer.estimated_height(1_000), Some(40.0));
}

#[test]
fn chat_virtualizer_append_extends_index_without_rebuild() {
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[40.0, 50.0]);
    virtualizer.reset_diagnostics();

    virtualizer.sync_len(5, 60.0);

    let diagnostics = virtualizer.diagnostics();
    assert_eq!(diagnostics.full_rebuilds, 0);
    assert_eq!(diagnostics.appends, 3);
    assert_eq!(diagnostics.truncate_rebuilds, 0);
    assert_eq!(virtualizer.estimated_heights(), &[40.0, 50.0, 60.0, 60.0, 60.0]);
}

#[test]
fn chat_virtualizer_prepend_reports_anchor_delta() {
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[80.0, 100.0, 120.0]);
    virtualizer.set_viewport_height(120.0);
    virtualizer.preserve_anchor(ChatScrollAnchor::new(1, 12.0));
    let before = virtualizer.scroll_top_px();
    virtualizer.reset_diagnostics();

    let anchor_delta = virtualizer.prepend_estimated_heights(&[40.0, 60.0]);

    assert_eq!(anchor_delta, 40.0 + 60.0 + CHAT_MESSAGE_GAP * 2.0);
    assert_eq!(virtualizer.scroll_top_px(), before + anchor_delta);
    assert_eq!(virtualizer.anchor(), Some(ChatScrollAnchor::new(3, 12.0)));
    let diagnostics = virtualizer.diagnostics();
    assert_eq!(diagnostics.full_rebuilds, 1);
    assert_eq!(diagnostics.prepend_rebuilds, 1);
}

#[test]
fn chat_virtualizer_truncate_rebuilds_once() {
    let mut virtualizer = ChatVirtualizer::from_estimated_heights(&[40.0, 50.0, 60.0, 70.0]);
    virtualizer.reset_diagnostics();

    virtualizer.sync_len(2, 80.0);

    assert_eq!(virtualizer.estimated_heights(), &[40.0, 50.0]);
    let diagnostics = virtualizer.diagnostics();
    assert_eq!(diagnostics.full_rebuilds, 1);
    assert_eq!(diagnostics.prepend_rebuilds, 0);
    assert_eq!(diagnostics.truncate_rebuilds, 1);
}
