//! 聊天虚拟列表领域模型。
//!
//! 该模块只维护纯数据滚动状态，不依赖 Iced 或 App 状态。

use super::height_index::{CHAT_VIRTUALIZATION_OVERSCAN_PX, ChatHeightIndex, ChatHeightWindow};

pub(crate) const CHAT_TURN_TOP_INSET_PX: f32 = 48.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChatScrollMode {
    Free,
    Anchored,
    TailSticky,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChatScrollAnchor {
    pub index: usize,
    pub offset_px: f32,
}

impl ChatScrollAnchor {
    pub fn new(index: usize, offset_px: f32) -> Self {
        Self { index, offset_px: offset_px.max(0.0) }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ChatVirtualWindow {
    pub render_start_idx: usize,
    pub render_end_idx: usize,
    pub visible_start_idx: usize,
    pub visible_end_idx: usize,
    pub top_spacer_h: f32,
    pub bottom_spacer_h: f32,
    pub scroll_top_px: f32,
    pub relative_y: f32,
    pub viewport_h: f32,
    pub total_height_px: f32,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct ChatVirtualizerDiagnostics {
    pub(crate) full_rebuilds: usize,
    pub(crate) appends: usize,
    pub(crate) point_updates: usize,
    pub(crate) prepend_rebuilds: usize,
    pub(crate) truncate_rebuilds: usize,
}

impl ChatVirtualWindow {
    fn from_height_window(
        height_window: ChatHeightWindow,
        scroll_top_px: f32,
        relative_y: f32,
        viewport_h: f32,
        total_height_px: f32,
    ) -> Self {
        Self {
            render_start_idx: height_window.render_start_idx,
            render_end_idx: height_window.render_end_idx,
            visible_start_idx: height_window.visible_start_idx,
            visible_end_idx: height_window.visible_end_idx,
            top_spacer_h: height_window.top_spacer_h,
            bottom_spacer_h: height_window.bottom_spacer_h,
            scroll_top_px,
            relative_y,
            viewport_h,
            total_height_px,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ChatVirtualizer {
    heights: Vec<f32>,
    height_index: ChatHeightIndex,
    scroll_top_px: f32,
    viewport_h: f32,
    scroll_mode: ChatScrollMode,
    anchor: Option<ChatScrollAnchor>,
    turn_focus_user_index: Option<usize>,
    #[cfg(test)]
    diagnostics: ChatVirtualizerDiagnostics,
}

impl Default for ChatVirtualizer {
    fn default() -> Self {
        Self {
            heights: Vec::new(),
            height_index: ChatHeightIndex::default(),
            scroll_top_px: 0.0,
            viewport_h: 0.0,
            scroll_mode: ChatScrollMode::Free,
            anchor: None,
            turn_focus_user_index: None,
            #[cfg(test)]
            diagnostics: ChatVirtualizerDiagnostics::default(),
        }
    }
}

impl ChatVirtualizer {
    pub fn from_estimated_heights(heights: &[f32]) -> Self {
        let mut virtualizer = Self::default();
        virtualizer.set_estimated_heights(heights);
        virtualizer
    }

    pub fn len(&self) -> usize {
        self.heights.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heights.is_empty()
    }

    pub fn estimated_heights(&self) -> &[f32] {
        &self.heights
    }

    pub fn estimated_height(&self, idx: usize) -> Option<f32> {
        self.heights.get(idx).copied()
    }

    pub fn scroll_top_px(&self) -> f32 {
        self.scroll_top_px
    }

    pub fn viewport_height_px(&self) -> f32 {
        self.viewport_h
    }

    pub fn total_height_px(&self) -> f32 {
        self.height_index.total_height()
    }

    pub fn max_scroll_top_px(&self) -> f32 {
        (self.total_height_px() - self.viewport_h).max(0.0)
    }

    pub fn relative_y(&self) -> f32 {
        let max_scroll_top_px = self.max_scroll_top_px();
        if max_scroll_top_px <= f32::EPSILON {
            0.0
        } else {
            (self.scroll_top_px / max_scroll_top_px).clamp(0.0, 1.0)
        }
    }

    pub fn scroll_mode(&self) -> ChatScrollMode {
        self.scroll_mode
    }

    pub fn anchor(&self) -> Option<ChatScrollAnchor> {
        self.anchor
    }

    pub fn turn_focus_user_index(&self) -> Option<usize> {
        self.turn_focus_user_index
    }

    pub fn trailing_runway_height_px(&self) -> f32 {
        self.height_index.trailing_runway()
    }

    pub fn sync_len(&mut self, len: usize, estimated_height: f32) {
        let previous_len = self.heights.len();
        match len.cmp(&previous_len) {
            std::cmp::Ordering::Equal => return,
            std::cmp::Ordering::Greater => {
                let appended = len - previous_len;
                let estimated_height = estimated_height.max(0.0);
                self.heights.resize(len, estimated_height);
                for _ in 0..appended {
                    self.height_index.append_height(estimated_height);
                }
                #[cfg(test)]
                {
                    self.diagnostics.appends += appended;
                }
            }
            std::cmp::Ordering::Less => {
                self.heights.truncate(len);
                self.height_index.set_heights(&self.heights);
                #[cfg(test)]
                {
                    self.diagnostics.full_rebuilds += 1;
                    self.diagnostics.truncate_rebuilds += 1;
                    super::performance::record_height_index_rebuild();
                }
            }
        }
        if self.turn_focus_user_index.is_some_and(|index| index >= len) {
            self.clear_turn_focus();
        }
        if self.anchor.is_some_and(|anchor| anchor.index >= len) {
            self.anchor = None;
            self.scroll_mode = ChatScrollMode::Free;
        }
        self.refresh_turn_runway();
        self.reconcile_scroll_after_height_change();
    }

    pub fn set_estimated_heights(&mut self, heights: &[f32]) {
        self.heights = heights.iter().map(|height| height.max(0.0)).collect();
        self.height_index.set_heights(&self.heights);
        #[cfg(test)]
        {
            self.diagnostics.full_rebuilds += 1;
            super::performance::record_height_index_rebuild();
        }
        if self.turn_focus_user_index.is_some_and(|index| index >= self.heights.len()) {
            self.clear_turn_focus();
        }
        if self.anchor.is_some_and(|anchor| anchor.index >= self.heights.len()) {
            self.anchor = None;
            self.scroll_mode = ChatScrollMode::Free;
        }
        self.refresh_turn_runway();
        self.reconcile_scroll_after_height_change();
    }

    pub fn prepend_estimated_heights(&mut self, heights: &[f32]) -> f32 {
        if heights.is_empty() {
            return 0.0;
        }

        let prepended: Vec<f32> = heights.iter().map(|height| height.max(0.0)).collect();
        let prepended_len = prepended.len();
        let anchor_delta = prepended.iter().sum::<f32>()
            + super::height_index::CHAT_MESSAGE_GAP * prepended_len as f32;
        let mut combined = Vec::with_capacity(prepended_len + self.heights.len());
        combined.extend_from_slice(&prepended);
        combined.extend_from_slice(&self.heights);
        self.heights = combined;
        self.height_index.set_heights(&self.heights);
        self.scroll_top_px += anchor_delta;
        if let Some(anchor) = &mut self.anchor {
            anchor.index += prepended_len;
        }
        if let Some(user_index) = &mut self.turn_focus_user_index {
            *user_index += prepended_len;
        }
        #[cfg(test)]
        {
            self.diagnostics.full_rebuilds += 1;
            self.diagnostics.prepend_rebuilds += 1;
            super::performance::record_height_index_rebuild();
        }
        self.refresh_turn_runway();
        self.reconcile_scroll_after_height_change();
        anchor_delta
    }

    pub fn set_viewport_height(&mut self, viewport_h: f32) {
        self.viewport_h = viewport_h.max(0.0);
        self.refresh_turn_runway();
        self.reconcile_scroll_after_height_change();
    }

    pub fn set_scroll_top_px(&mut self, scroll_top_px: f32) {
        self.clear_turn_focus();
        self.scroll_mode = ChatScrollMode::Free;
        self.anchor = None;
        self.scroll_top_px = self.clamp_scroll_top(scroll_top_px);
    }

    pub fn set_relative_y(&mut self, relative_y: f32) {
        let scroll_top_px = relative_y.clamp(0.0, 1.0) * self.max_scroll_top_px();
        self.set_scroll_top_px(scroll_top_px);
    }

    pub fn apply_measured_height(&mut self, idx: usize, height: f32) {
        if idx >= self.heights.len() {
            return;
        }

        let next_height = height.max(0.0);
        if (self.heights[idx] - next_height).abs() <= f32::EPSILON {
            return;
        }

        self.heights[idx] = next_height;
        self.height_index.update_height(idx, next_height);
        #[cfg(test)]
        {
            self.diagnostics.point_updates += 1;
            super::performance::record_height_point_update();
        }
        self.refresh_turn_runway();
        self.reconcile_scroll_after_height_change();
    }

    pub fn preserve_current_anchor_around(&mut self) -> Option<ChatScrollAnchor> {
        let anchor = self.current_anchor()?;
        self.preserve_anchor(anchor);
        Some(anchor)
    }

    pub fn preserve_anchor(&mut self, anchor: ChatScrollAnchor) {
        if anchor.index >= self.heights.len() {
            return;
        }

        self.clear_turn_focus();
        self.scroll_mode = ChatScrollMode::Anchored;
        self.anchor = Some(anchor);
        self.scroll_top_px = self.clamp_scroll_top(self.scroll_top_for_anchor(anchor));
    }

    pub fn scroll_to_item_anchor(
        &mut self,
        index: usize,
        item_anchor_fraction: f32,
        viewport_anchor_fraction: f32,
    ) {
        if index >= self.heights.len() {
            return;
        }

        self.clear_turn_focus();
        let item_top = self.height_index.scroll_top_for_item(index);
        let item_height = self.heights.get(index).copied().unwrap_or(0.0).max(0.0);
        let desired_top = item_top + item_height * item_anchor_fraction.clamp(0.0, 1.0)
            - self.viewport_h * viewport_anchor_fraction.clamp(0.0, 1.0);
        let scroll_top = self.clamp_scroll_top(desired_top);
        let anchor = ChatScrollAnchor::new(index, scroll_top - item_top);
        self.scroll_mode = ChatScrollMode::Anchored;
        self.anchor = Some(anchor);
        self.scroll_top_px = scroll_top;
    }

    pub fn focus_turn_on_user(&mut self, user_index: usize) {
        if user_index >= self.heights.len() {
            return;
        }

        let anchor = ChatScrollAnchor { index: user_index, offset_px: -CHAT_TURN_TOP_INSET_PX };
        self.turn_focus_user_index = Some(user_index);
        self.scroll_mode = ChatScrollMode::Anchored;
        self.anchor = Some(anchor);
        self.refresh_turn_runway();
        self.scroll_top_px = self.clamp_scroll_top(self.scroll_top_for_anchor(anchor));
    }

    pub fn release_turn_focus(&mut self) {
        if self.clear_turn_focus() {
            self.scroll_mode = ChatScrollMode::Free;
            self.anchor = None;
            self.scroll_top_px = self.clamp_scroll_top(self.scroll_top_px);
        }
    }

    pub fn snap_to_tail(&mut self) {
        self.clear_turn_focus();
        self.scroll_mode = ChatScrollMode::TailSticky;
        self.anchor = None;
        self.scroll_top_px = self.max_scroll_top_px();
    }

    pub fn should_follow_tail_after_growth(&self) -> bool {
        matches!(self.scroll_mode, ChatScrollMode::TailSticky)
    }

    pub fn compute_window(&self, overscan_px: f32) -> ChatVirtualWindow {
        let height_window =
            self.height_index.compute_window(self.relative_y(), self.viewport_h, overscan_px);
        ChatVirtualWindow::from_height_window(
            height_window,
            self.scroll_top_px,
            self.relative_y(),
            self.viewport_h,
            self.total_height_px(),
        )
    }

    pub fn window(&self) -> ChatVirtualWindow {
        self.compute_window(CHAT_VIRTUALIZATION_OVERSCAN_PX)
    }

    #[cfg(test)]
    pub(crate) fn diagnostics(&self) -> ChatVirtualizerDiagnostics {
        self.diagnostics
    }

    #[cfg(test)]
    pub(crate) fn reset_diagnostics(&mut self) {
        self.diagnostics = ChatVirtualizerDiagnostics::default();
    }

    fn reconcile_scroll_after_height_change(&mut self) {
        match (self.scroll_mode, self.anchor) {
            (ChatScrollMode::TailSticky, _) => self.scroll_top_px = self.max_scroll_top_px(),
            (ChatScrollMode::Anchored, Some(anchor)) if anchor.index < self.heights.len() => {
                self.scroll_top_px = self.clamp_scroll_top(self.scroll_top_for_anchor(anchor));
            }
            _ => self.scroll_top_px = self.clamp_scroll_top(self.scroll_top_px),
        }
    }

    fn refresh_turn_runway(&mut self) {
        let Some(user_index) = self.turn_focus_user_index else {
            return;
        };

        self.height_index.set_trailing_runway(0.0);
        let user_top = self.height_index.scroll_top_for_item(user_index);
        let turn_height = (self.height_index.total_height() - user_top).max(0.0);
        self.height_index
            .set_trailing_runway((self.viewport_h - CHAT_TURN_TOP_INSET_PX - turn_height).max(0.0));
    }

    fn clear_turn_focus(&mut self) -> bool {
        let had_turn_focus = self.turn_focus_user_index.take().is_some();
        if had_turn_focus {
            self.height_index.set_trailing_runway(0.0);
        }
        had_turn_focus
    }

    fn current_anchor(&self) -> Option<ChatScrollAnchor> {
        if self.heights.is_empty() {
            return None;
        }

        let index = self
            .height_index
            .find_start_by_scroll_top(self.scroll_top_px)
            .min(self.heights.len().saturating_sub(1));
        let item_top = self.height_index.scroll_top_for_item(index);
        Some(ChatScrollAnchor::new(index, self.scroll_top_px - item_top))
    }

    fn scroll_top_for_anchor(&self, anchor: ChatScrollAnchor) -> f32 {
        self.height_index.scroll_top_for_item(anchor.index) + anchor.offset_px
    }

    fn clamp_scroll_top(&self, scroll_top_px: f32) -> f32 {
        scroll_top_px.max(0.0).min(self.max_scroll_top_px())
    }
}
