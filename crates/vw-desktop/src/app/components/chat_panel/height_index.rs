//! 维护聊天消息高度索引。
//! 索引用于将滚动位置映射到消息范围，避免长列表中反复扫描全部项。

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ChatHeightWindow {
    pub(crate) render_start_idx: usize,
    pub(crate) render_end_idx: usize,
    pub(crate) visible_start_idx: usize,
    pub(crate) visible_end_idx: usize,
    pub(crate) top_spacer_h: f32,
    pub(crate) bottom_spacer_h: f32,
}

impl ChatHeightWindow {
    pub(crate) fn full(len: usize) -> Self {
        Self {
            render_start_idx: 0,
            render_end_idx: len,
            visible_start_idx: 0,
            visible_end_idx: len,
            top_spacer_h: 0.0,
            bottom_spacer_h: 0.0,
        }
    }
}

/// CHAT_MESSAGE_GAP 是该模块对外使用的常量值。
pub(crate) const CHAT_MESSAGE_GAP: f32 = 18.0;
pub(super) const CHAT_LIST_VERTICAL_PADDING: f32 = 20.0;
/// CHAT_VIRTUALIZATION_OVERSCAN_PX 是该模块对外使用的常量值。
pub(crate) const CHAT_VIRTUALIZATION_OVERSCAN_PX: f32 = 900.0;

/// ChatHeightIndex 表示该模块对外暴露的结构化状态。
#[derive(Debug, Clone, Default)]
pub(crate) struct ChatHeightIndex {
    heights: Vec<f32>,
    slot_values: Vec<f32>,
    fenwick_tree: Vec<f32>,
    total_slot_height: f32,
    trailing_runway: f32,
}

impl ChatHeightIndex {
    pub(crate) fn from_heights(heights: &[f32]) -> Self {
        let mut index = Self::default();
        index.set_heights(heights);
        index
    }

    pub(crate) fn clear(&mut self) {
        self.heights.clear();
        self.slot_values.clear();
        self.fenwick_tree.clear();
        self.total_slot_height = 0.0;
        self.trailing_runway = 0.0;
    }

    #[allow(dead_code)]
    pub(crate) fn len(&self) -> usize {
        self.heights.len()
    }

    pub(crate) fn total_height(&self) -> f32 {
        if self.heights.is_empty() {
            0.0
        } else {
            (self.total_slot_height - CHAT_MESSAGE_GAP).max(0.0)
                + CHAT_LIST_VERTICAL_PADDING
                + self.trailing_runway
        }
    }

    pub(crate) fn trailing_runway(&self) -> f32 {
        self.trailing_runway
    }

    pub(crate) fn set_trailing_runway(&mut self, trailing_runway: f32) {
        self.trailing_runway = trailing_runway.max(0.0);
    }

    pub(crate) fn set_heights(&mut self, heights: &[f32]) {
        self.heights = heights.iter().map(|height| height.max(0.0)).collect();
        self.slot_values = self.heights.iter().map(|height| height + CHAT_MESSAGE_GAP).collect();
        self.total_slot_height = self.slot_values.iter().sum();
        self.fenwick_tree = vec![0.0; self.slot_values.len() + 1];

        for (idx, value) in self.slot_values.iter().copied().enumerate() {
            let tree_idx = idx + 1;
            self.fenwick_tree[tree_idx] += value;
            let parent = tree_idx + least_significant_bit(tree_idx);
            if parent < self.fenwick_tree.len() {
                self.fenwick_tree[parent] += self.fenwick_tree[tree_idx];
            }
        }
    }

    pub(crate) fn update_height(&mut self, idx: usize, value: f32) {
        if idx >= self.heights.len() {
            return;
        }

        let next_height = value.max(0.0);
        let next_slot_value = next_height + CHAT_MESSAGE_GAP;
        let diff = next_slot_value - self.slot_values[idx];
        if diff.abs() <= f32::EPSILON {
            return;
        }

        self.heights[idx] = next_height;
        self.slot_values[idx] = next_slot_value;
        self.total_slot_height += diff;
        self.fenwick_add(idx + 1, diff);
    }

    #[cfg(test)]
    pub(crate) fn append_heights(&mut self, heights: &[f32]) {
        for height in heights {
            self.append_height(*height);
        }
    }

    pub(crate) fn find_start_by_scroll_top(&self, px: f32) -> usize {
        if self.heights.is_empty() {
            return 0;
        }

        let target = px.max(0.0);
        if target <= 0.0 {
            return 0;
        }
        if target > self.total_slot_height {
            return self.heights.len();
        }

        self.lower_bound_prefix(target)
    }

    pub(crate) fn scroll_top_for_item(&self, idx: usize) -> f32 {
        self.prefix_before_item(idx)
    }

    pub(crate) fn compute_window(
        &self,
        scroll_offset_y: f32,
        viewport_h: f32,
        overscan: f32,
    ) -> ChatHeightWindow {
        if self.heights.is_empty() {
            return ChatHeightWindow::default();
        }

        let viewport_h = viewport_h.max(0.0);
        if viewport_h <= 0.0 {
            return ChatHeightWindow::full(self.heights.len());
        }

        let total_height = self.total_height();
        let max_scroll = (total_height - viewport_h).max(0.0);
        let scroll_top = scroll_offset_y.clamp(0.0, 1.0) * max_scroll;
        let visible_top = scroll_top;
        let visible_bottom = (scroll_top + viewport_h).min(total_height);
        let render_top = (scroll_top - overscan.max(0.0)).max(0.0);
        let render_bottom = (scroll_top + viewport_h + overscan.max(0.0)).min(total_height);

        let visible_start_idx = self.find_start_by_scroll_top(visible_top);
        let visible_end_idx =
            self.normalize_end(visible_start_idx, self.find_end_by_scroll_bottom(visible_bottom));
        let render_start_idx = self.find_start_by_scroll_top(render_top);
        let render_end_idx =
            self.normalize_end(render_start_idx, self.find_end_by_scroll_bottom(render_bottom));
        let top_spacer_h = self.prefix_before_item(render_start_idx);
        let rendered_height = self.rendered_height(render_start_idx, render_end_idx);
        let bottom_spacer_h = (total_height - top_spacer_h - rendered_height).max(0.0);

        ChatHeightWindow {
            render_start_idx,
            render_end_idx,
            visible_start_idx,
            visible_end_idx,
            top_spacer_h,
            bottom_spacer_h,
        }
    }

    fn find_end_by_scroll_bottom(&self, px: f32) -> usize {
        if self.heights.is_empty() {
            return 0;
        }

        let target = px.max(0.0);
        if target <= 0.0 {
            return 1.min(self.heights.len());
        }
        if target > self.total_slot_height {
            return self.heights.len();
        }

        self.lower_bound_prefix(target).saturating_add(1)
    }

    fn normalize_end(&self, start_idx: usize, end_idx: usize) -> usize {
        let len = self.heights.len();
        if start_idx >= len {
            return len;
        }

        end_idx.max(start_idx.saturating_add(1)).min(len)
    }

    fn rendered_height(&self, start_idx: usize, end_idx: usize) -> f32 {
        if end_idx <= start_idx {
            return 0.0;
        }

        (self.prefix_before_item(end_idx) - self.prefix_before_item(start_idx) - CHAT_MESSAGE_GAP)
            .max(0.0)
    }

    fn prefix_before_item(&self, idx: usize) -> f32 {
        self.prefix_sum(idx.min(self.heights.len()))
    }

    fn prefix_sum(&self, count: usize) -> f32 {
        let mut idx = count.min(self.heights.len());
        let mut sum = 0.0;

        while idx > 0 {
            sum += self.fenwick_tree[idx];
            idx -= least_significant_bit(idx);
        }

        sum
    }

    fn fenwick_add(&mut self, idx: usize, diff: f32) {
        let mut tree_idx = idx;
        while tree_idx < self.fenwick_tree.len() {
            self.fenwick_tree[tree_idx] += diff;
            tree_idx += least_significant_bit(tree_idx);
        }
    }

    pub(crate) fn append_height(&mut self, height: f32) {
        let height = height.max(0.0);
        let slot_value = height + CHAT_MESSAGE_GAP;
        let tree_idx = self.heights.len() + 1;
        let range_start = tree_idx - least_significant_bit(tree_idx);
        let covered_prefix = self.prefix_sum(tree_idx - 1) - self.prefix_sum(range_start);

        self.heights.push(height);
        self.slot_values.push(slot_value);
        self.fenwick_tree.push(covered_prefix + slot_value);
        self.total_slot_height += slot_value;
    }

    fn lower_bound_prefix(&self, target: f32) -> usize {
        if self.heights.is_empty() {
            return 0;
        }

        let mut idx = 0usize;
        let mut bit = highest_power_of_two_at_most(self.heights.len());
        let mut sum = 0.0;

        while bit > 0 {
            let next = idx + bit;
            if next <= self.heights.len() && sum + self.fenwick_tree[next] < target {
                idx = next;
                sum += self.fenwick_tree[next];
            }
            bit >>= 1;
        }

        idx.min(self.heights.len().saturating_sub(1))
    }
}

fn least_significant_bit(value: usize) -> usize {
    value & value.wrapping_neg()
}

fn highest_power_of_two_at_most(value: usize) -> usize {
    if value == 0 {
        return 0;
    }

    1usize << (usize::BITS as usize - value.leading_zeros() as usize - 1)
}
