//! 管理聊天面板的分块加载状态。
//! 该模块把大消息流的可见窗口控制在组件内部，降低渲染压力。

use std::collections::HashSet;

use crate::app::session::{CHAT_UI_CHUNK_SIZE, chat_ui_chunk_bounds, chat_ui_chunk_start_idx};

#[cfg(test)]
const CHAT_CHUNK_RETAIN_NEIGHBOR_RADIUS: usize = 1;
const CHAT_PROTECTED_CHUNK_RETAIN_RADIUS: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// 描述聊天 chunk 调度时滚动方向。
pub(crate) enum ChatChunkScrollDirection {
    Up,
    Down,
    Stationary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// 表示聊天 chunk 调度需要的统一输入。
pub(crate) struct ChatChunkScheduleInput {
    pub chat_len: usize,
    pub render_start_idx: usize,
    pub render_end_idx: usize,
    pub visible_start_idx: usize,
    pub visible_end_idx: usize,
    pub anchor_idx: Option<usize>,
    pub scroll_direction: ChatChunkScrollDirection,
    pub protected_chunk_starts: Vec<usize>,
}

fn normalized_window(chat_len: usize, start_idx: usize, end_idx: usize) -> Option<(usize, usize)> {
    if chat_len == 0 {
        return None;
    }

    let visible_start_idx = start_idx.min(chat_len.saturating_sub(1));
    let visible_end_idx = end_idx.max(visible_start_idx.saturating_add(1)).min(chat_len);
    Some((visible_start_idx, visible_end_idx))
}

fn chunk_starts_around(chat_len: usize, chunk_start_idx: usize, radius: usize) -> Vec<usize> {
    if chat_len == 0 {
        return Vec::new();
    }

    let normalized_chunk_start =
        chat_ui_chunk_start_idx(chunk_start_idx.min(chat_len.saturating_sub(1)));
    let center_chunk_idx = normalized_chunk_start / CHAT_UI_CHUNK_SIZE;
    let max_chunk_idx = chat_len.saturating_sub(1) / CHAT_UI_CHUNK_SIZE;
    let start_chunk_idx = center_chunk_idx.saturating_sub(radius);
    let end_chunk_idx = center_chunk_idx.saturating_add(radius).min(max_chunk_idx);

    (start_chunk_idx..=end_chunk_idx).map(|chunk_idx| chunk_idx * CHAT_UI_CHUNK_SIZE).collect()
}

fn chunk_starts_in_range(chat_len: usize, start_idx: usize, end_idx: usize) -> Vec<usize> {
    let Some((start_idx, end_idx)) = normalized_window(chat_len, start_idx, end_idx) else {
        return Vec::new();
    };

    let mut chunk_starts = Vec::new();
    let mut chunk_start_idx = chat_ui_chunk_start_idx(start_idx);
    while chunk_start_idx < end_idx {
        chunk_starts.push(chunk_start_idx);
        chunk_start_idx = chunk_start_idx.saturating_add(CHAT_UI_CHUNK_SIZE);
    }
    chunk_starts
}

fn schedule_anchor_idx(input: &ChatChunkScheduleInput) -> Option<usize> {
    if input.chat_len == 0 {
        return None;
    }

    if let Some(anchor_idx) = input.anchor_idx {
        return Some(anchor_idx.min(input.chat_len.saturating_sub(1)));
    }

    let (visible_start_idx, visible_end_idx) =
        normalized_window(input.chat_len, input.visible_start_idx, input.visible_end_idx)?;
    Some(visible_start_idx + (visible_end_idx - visible_start_idx - 1) / 2)
}

fn schedule_anchor_chunk_start(input: &ChatChunkScheduleInput) -> Option<usize> {
    schedule_anchor_idx(input).map(chat_ui_chunk_start_idx)
}

fn push_unique(chunk_starts: &mut Vec<usize>, chat_len: usize, chunk_start_idx: usize) {
    if chunk_start_idx < chat_len && !chunk_starts.contains(&chunk_start_idx) {
        chunk_starts.push(chunk_start_idx);
    }
}

/// 执行 prioritized_chunk_starts 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn prioritized_chunk_starts(
    input: &ChatChunkScheduleInput,
    include_neighbors: bool,
) -> Vec<usize> {
    if input.chat_len == 0 {
        return Vec::new();
    }

    let mut prioritized = Vec::new();
    if let Some(anchor_chunk_start) = schedule_anchor_chunk_start(input) {
        push_unique(&mut prioritized, input.chat_len, anchor_chunk_start);
    }

    let mut visible_chunk_starts =
        chunk_starts_in_range(input.chat_len, input.visible_start_idx, input.visible_end_idx);
    match input.scroll_direction {
        ChatChunkScrollDirection::Down => {
            visible_chunk_starts.sort_unstable_by(|left, right| right.cmp(left))
        }
        ChatChunkScrollDirection::Up => visible_chunk_starts.sort_unstable(),
        ChatChunkScrollDirection::Stationary => {
            if let Some(anchor_chunk_start) = schedule_anchor_chunk_start(input) {
                visible_chunk_starts.sort_by_key(|chunk_start_idx| {
                    (chunk_start_idx.abs_diff(anchor_chunk_start), *chunk_start_idx)
                });
            }
        }
    }
    for chunk_start_idx in visible_chunk_starts {
        push_unique(&mut prioritized, input.chat_len, chunk_start_idx);
    }

    if include_neighbors {
        let visible_chunks =
            chunk_starts_in_range(input.chat_len, input.visible_start_idx, input.visible_end_idx);
        if let (Some(first_visible), Some(last_visible)) =
            (visible_chunks.first().copied(), visible_chunks.last().copied())
        {
            let previous_visible = first_visible.checked_sub(CHAT_UI_CHUNK_SIZE);
            let next_visible = last_visible.saturating_add(CHAT_UI_CHUNK_SIZE);
            match input.scroll_direction {
                ChatChunkScrollDirection::Down => {
                    push_unique(&mut prioritized, input.chat_len, next_visible);
                    if let Some(previous_visible) = previous_visible {
                        push_unique(&mut prioritized, input.chat_len, previous_visible);
                    }
                }
                ChatChunkScrollDirection::Up => {
                    if let Some(previous_visible) = previous_visible {
                        push_unique(&mut prioritized, input.chat_len, previous_visible);
                    }
                    push_unique(&mut prioritized, input.chat_len, next_visible);
                }
                ChatChunkScrollDirection::Stationary => {
                    let mut neighbor_chunk_starts = Vec::new();
                    if let Some(previous_visible) = previous_visible {
                        neighbor_chunk_starts.push(previous_visible);
                    }
                    if next_visible < input.chat_len {
                        neighbor_chunk_starts.push(next_visible);
                    }
                    if let Some(anchor_chunk_start) = schedule_anchor_chunk_start(input) {
                        neighbor_chunk_starts.sort_by_key(|chunk_start_idx| {
                            (chunk_start_idx.abs_diff(anchor_chunk_start), *chunk_start_idx)
                        });
                    }
                    for chunk_start_idx in neighbor_chunk_starts {
                        push_unique(&mut prioritized, input.chat_len, chunk_start_idx);
                    }
                }
            }
        }
    }

    for chunk_start_idx in
        chunk_starts_in_range(input.chat_len, input.render_start_idx, input.render_end_idx)
    {
        push_unique(&mut prioritized, input.chat_len, chunk_start_idx);
    }

    if include_neighbors && let Some(anchor_chunk_start) = schedule_anchor_chunk_start(input) {
        let previous_anchor = anchor_chunk_start.checked_sub(CHAT_UI_CHUNK_SIZE);
        let next_anchor = anchor_chunk_start.saturating_add(CHAT_UI_CHUNK_SIZE);
        if let Some(previous_anchor) = previous_anchor {
            push_unique(&mut prioritized, input.chat_len, previous_anchor);
        }
        push_unique(&mut prioritized, input.chat_len, next_anchor);
    }

    for &chunk_start_idx in &input.protected_chunk_starts {
        push_unique(
            &mut prioritized,
            input.chat_len,
            chat_ui_chunk_start_idx(chunk_start_idx.min(input.chat_len.saturating_sub(1))),
        );
    }

    prioritized
}

/// 执行 retained_chunk_starts_for_schedule 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn retained_chunk_starts_for_schedule(input: &ChatChunkScheduleInput) -> HashSet<usize> {
    let mut retained = HashSet::new();
    if input.chat_len == 0 {
        return retained;
    }

    for chunk_start_idx in prioritized_chunk_starts(input, true) {
        retained.insert(chunk_start_idx);
    }

    for &chunk_start_idx in &input.protected_chunk_starts {
        for retained_chunk_start in
            chunk_starts_around(input.chat_len, chunk_start_idx, CHAT_PROTECTED_CHUNK_RETAIN_RADIUS)
        {
            retained.insert(retained_chunk_start);
        }
    }

    retained
}

/// 执行 retained_chunk_starts 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
#[cfg(test)]
pub(crate) fn retained_chunk_starts(
    chat_len: usize,
    start_idx: usize,
    end_idx: usize,
    protected_chunk_starts: &[usize],
) -> HashSet<usize> {
    let mut retained = HashSet::new();
    let Some((visible_start_idx, visible_end_idx)) =
        normalized_window(chat_len, start_idx, end_idx)
    else {
        return retained;
    };

    let mut chunk_start_idx = chat_ui_chunk_start_idx(visible_start_idx);
    while chunk_start_idx < visible_end_idx {
        retained.insert(chunk_start_idx);
        chunk_start_idx = chunk_start_idx.saturating_add(CHAT_UI_CHUNK_SIZE);
    }

    let first_visible_chunk_start = chat_ui_chunk_start_idx(visible_start_idx);
    let last_visible_chunk_start = chat_ui_chunk_start_idx(visible_end_idx.saturating_sub(1));
    for chunk_start_idx in
        chunk_starts_around(chat_len, first_visible_chunk_start, CHAT_CHUNK_RETAIN_NEIGHBOR_RADIUS)
    {
        retained.insert(chunk_start_idx);
    }
    for chunk_start_idx in
        chunk_starts_around(chat_len, last_visible_chunk_start, CHAT_CHUNK_RETAIN_NEIGHBOR_RADIUS)
    {
        retained.insert(chunk_start_idx);
    }

    for &chunk_start_idx in protected_chunk_starts {
        for retained_chunk_start in
            chunk_starts_around(chat_len, chunk_start_idx, CHAT_PROTECTED_CHUNK_RETAIN_RADIUS)
        {
            retained.insert(retained_chunk_start);
        }
    }

    retained
}

/// 执行 eviction_chunk_starts 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn eviction_chunk_starts(
    prepared_chunk_starts: &HashSet<usize>,
    retained_chunk_starts: &HashSet<usize>,
) -> Vec<usize> {
    let mut eviction_chunk_starts: Vec<_> = prepared_chunk_starts
        .iter()
        .copied()
        .filter(|chunk_start_idx| !retained_chunk_starts.contains(chunk_start_idx))
        .collect();
    eviction_chunk_starts.sort_unstable();
    eviction_chunk_starts
}

/// 执行 heavy_cache_keep_bounds 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
#[cfg(test)]
pub(crate) fn heavy_cache_keep_bounds(
    chat_len: usize,
    start_idx: usize,
    end_idx: usize,
    margin: usize,
    protected_chunk_starts: &[usize],
) -> (usize, usize) {
    let Some((visible_start_idx, visible_end_idx)) =
        normalized_window(chat_len, start_idx, end_idx)
    else {
        return (0, 0);
    };

    let mut keep_start = visible_start_idx.saturating_sub(margin);
    let mut keep_end = visible_end_idx.saturating_add(margin).min(chat_len);

    for &chunk_start_idx in protected_chunk_starts {
        let (chunk_start_idx, chunk_end_idx) = chat_ui_chunk_bounds(chat_len, chunk_start_idx);
        keep_start = keep_start.min(chunk_start_idx.saturating_sub(margin));
        keep_end = keep_end.max(chunk_end_idx.saturating_add(margin).min(chat_len));
    }

    (keep_start, keep_end)
}

/// 执行 heavy_cache_keep_bounds_for_schedule 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub(crate) fn heavy_cache_keep_bounds_for_schedule(
    input: &ChatChunkScheduleInput,
    margin: usize,
) -> (usize, usize) {
    let Some((visible_start_idx, visible_end_idx)) =
        normalized_window(input.chat_len, input.visible_start_idx, input.visible_end_idx)
    else {
        return (0, 0);
    };

    let mut keep_start = visible_start_idx.saturating_sub(margin);
    let mut keep_end = visible_end_idx.saturating_add(margin).min(input.chat_len);

    if let Some(anchor_idx) = schedule_anchor_idx(input) {
        let (chunk_start_idx, chunk_end_idx) =
            chat_ui_chunk_bounds(input.chat_len, chat_ui_chunk_start_idx(anchor_idx));
        keep_start = keep_start.min(chunk_start_idx.saturating_sub(margin));
        keep_end = keep_end.max(chunk_end_idx.saturating_add(margin).min(input.chat_len));
    }

    for &chunk_start_idx in &input.protected_chunk_starts {
        let (chunk_start_idx, chunk_end_idx) =
            chat_ui_chunk_bounds(input.chat_len, chunk_start_idx);
        keep_start = keep_start.min(chunk_start_idx.saturating_sub(margin));
        keep_end = keep_end.max(chunk_end_idx.saturating_add(margin).min(input.chat_len));
    }

    (keep_start, keep_end)
}
