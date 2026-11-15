//! 验证聊天面板分块加载器。
//! 测试覆盖窗口扩展和边界条件，确保长对话渲染稳定。

use std::collections::HashSet;

use super::chunk_loader::{
    ChatChunkScheduleInput, ChatChunkScrollDirection, eviction_chunk_starts,
    heavy_cache_keep_bounds, heavy_cache_keep_bounds_for_schedule, prioritized_chunk_starts,
    retained_chunk_starts, retained_chunk_starts_for_schedule,
};

#[test]
fn chunk_scheduler_prioritizes_virtual_anchor() {
    let input = ChatChunkScheduleInput {
        chat_len: 260,
        render_start_idx: 32,
        render_end_idx: 128,
        visible_start_idx: 64,
        visible_end_idx: 96,
        anchor_idx: Some(170),
        scroll_direction: ChatChunkScrollDirection::Down,
        protected_chunk_starts: Vec::new(),
    };

    let chunk_starts = prioritized_chunk_starts(&input, true);

    assert_eq!(chunk_starts, vec![160, 64, 96, 32, 128, 192]);
}

#[test]
fn chunk_scheduler_orders_visible_chunks_by_scroll_direction() {
    let base = ChatChunkScheduleInput {
        chat_len: 260,
        render_start_idx: 0,
        render_end_idx: 0,
        visible_start_idx: 0,
        visible_end_idx: 192,
        anchor_idx: Some(100),
        scroll_direction: ChatChunkScrollDirection::Stationary,
        protected_chunk_starts: Vec::new(),
    };

    let mut input = base.clone();
    input.scroll_direction = ChatChunkScrollDirection::Up;
    assert_eq!(prioritized_chunk_starts(&input, false), vec![96, 0, 32, 64, 128, 160]);

    input.scroll_direction = ChatChunkScrollDirection::Down;
    assert_eq!(prioritized_chunk_starts(&input, false), vec![96, 160, 128, 64, 32, 0]);

    input.scroll_direction = ChatChunkScrollDirection::Stationary;
    assert_eq!(prioritized_chunk_starts(&input, false), vec![96, 64, 128, 32, 160, 0]);
}

#[test]
fn chunk_scheduler_orders_neighbor_chunks_by_scroll_direction() {
    let base = ChatChunkScheduleInput {
        chat_len: 260,
        render_start_idx: 64,
        render_end_idx: 96,
        visible_start_idx: 64,
        visible_end_idx: 96,
        anchor_idx: Some(70),
        scroll_direction: ChatChunkScrollDirection::Down,
        protected_chunk_starts: Vec::new(),
    };

    let mut input = base.clone();
    assert_eq!(prioritized_chunk_starts(&input, true), vec![64, 96, 32]);

    input.scroll_direction = ChatChunkScrollDirection::Up;
    assert_eq!(prioritized_chunk_starts(&input, true), vec![64, 32, 96]);
}

#[test]
fn chunk_scheduler_normalizes_anchor_and_protected_chunk_starts() {
    let input = ChatChunkScheduleInput {
        chat_len: 260,
        render_start_idx: 0,
        render_end_idx: 1,
        visible_start_idx: 260,
        visible_end_idx: 260,
        anchor_idx: Some(usize::MAX),
        scroll_direction: ChatChunkScrollDirection::Stationary,
        protected_chunk_starts: vec![255, usize::MAX],
    };

    let chunk_starts = prioritized_chunk_starts(&input, false);

    assert_eq!(chunk_starts, vec![256, 0, 224]);
}

#[test]
fn retained_chunk_starts_keeps_visible_neighbors_and_protected_chunks() {
    let retained = retained_chunk_starts(260, 70, 125, &[192]);

    assert!(retained.contains(&64));
    assert!(retained.contains(&128));
    assert!(retained.contains(&192));
}

#[test]
fn retained_chunk_starts_for_schedule_keeps_prioritized_and_protected_neighbors() {
    let input = ChatChunkScheduleInput {
        chat_len: 260,
        render_start_idx: 0,
        render_end_idx: 64,
        visible_start_idx: 128,
        visible_end_idx: 160,
        anchor_idx: None,
        scroll_direction: ChatChunkScrollDirection::Stationary,
        protected_chunk_starts: vec![192],
    };

    let retained = retained_chunk_starts_for_schedule(&input);

    for chunk_start in [0, 32, 96, 128, 160, 192, 224] {
        assert!(retained.contains(&chunk_start), "missing chunk {chunk_start}");
    }
}

#[test]
fn eviction_chunk_starts_returns_sorted_unretained_chunks() {
    let prepared = HashSet::from([0, 64, 128, 192]);
    let retained = HashSet::from([64, 128]);

    assert_eq!(eviction_chunk_starts(&prepared, &retained), vec![0, 192]);
}

#[test]
fn heavy_cache_keep_bounds_expands_for_protected_chunk() {
    let (keep_start, keep_end) = heavy_cache_keep_bounds(260, 120, 140, 24, &[192]);

    assert_eq!(keep_start, 96);
    assert_eq!(keep_end, 248);
}

#[test]
fn heavy_cache_keep_bounds_for_schedule_expands_for_anchor_and_clamps_to_chat_len() {
    let input = ChatChunkScheduleInput {
        chat_len: 150,
        render_start_idx: 0,
        render_end_idx: 0,
        visible_start_idx: 10,
        visible_end_idx: 20,
        anchor_idx: Some(149),
        scroll_direction: ChatChunkScrollDirection::Stationary,
        protected_chunk_starts: vec![64],
    };

    assert_eq!(heavy_cache_keep_bounds_for_schedule(&input, 12), (0, 150));
}

#[test]
fn schedule_helpers_return_empty_results_for_empty_chat() {
    let input = ChatChunkScheduleInput {
        chat_len: 0,
        render_start_idx: 0,
        render_end_idx: usize::MAX,
        visible_start_idx: usize::MAX,
        visible_end_idx: usize::MAX,
        anchor_idx: Some(usize::MAX),
        scroll_direction: ChatChunkScrollDirection::Stationary,
        protected_chunk_starts: vec![usize::MAX],
    };

    assert!(prioritized_chunk_starts(&input, true).is_empty());
    assert!(retained_chunk_starts(0, 0, usize::MAX, &[usize::MAX]).is_empty());
    assert!(retained_chunk_starts_for_schedule(&input).is_empty());
    assert_eq!(heavy_cache_keep_bounds(0, 0, usize::MAX, 8, &[0]), (0, 0));
    assert_eq!(heavy_cache_keep_bounds_for_schedule(&input, usize::MAX), (0, 0));
}
