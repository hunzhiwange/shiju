//! Incremental projection state for one streaming assistant message.

const MAX_MUTABLE_TAIL_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct StreamRenderWork {
    pub full_message_clones: usize,
    pub full_content_hashes: usize,
    pub full_content_parses: usize,
    pub height_index_rebuilds: usize,
    pub tail_parses: usize,
    pub canonical_parses: usize,
}

#[derive(Debug, Clone, Default)]
pub struct StreamRenderState {
    session_id: String,
    generation: u64,
    message_idx: Option<usize>,
    stable_prefix_bytes: usize,
    mutable_tail: String,
    revision: u64,
    work: StreamRenderWork,
}

impl StreamRenderState {
    pub fn begin(
        &mut self,
        session_id: &str,
        generation: u64,
        message_idx: usize,
        content: &str,
    ) {
        if self.session_id == session_id
            && self.generation == generation
            && self.message_idx == Some(message_idx)
        {
            return;
        }
        *self = Self {
            session_id: session_id.to_owned(),
            generation,
            message_idx: Some(message_idx),
            ..Self::default()
        };
        self.reproject_tail(content);
    }

    #[cfg(test)]
    pub fn append(
        &mut self,
        session_id: &str,
        generation: u64,
        message_idx: usize,
        content: &str,
    ) -> bool {
        if self.session_id != session_id
            || self.generation != generation
            || self.message_idx != Some(message_idx)
        {
            return false;
        }
        self.revision = self.revision.saturating_add(1);
        self.reproject_tail(content);
        true
    }

    pub fn append_delta(
        &mut self,
        session_id: &str,
        generation: u64,
        message_idx: usize,
        delta: &str,
    ) -> bool {
        if self.session_id != session_id
            || self.generation != generation
            || self.message_idx != Some(message_idx)
        {
            return false;
        }
        self.revision = self.revision.saturating_add(1);
        self.mutable_tail.push_str(delta);
        if self.mutable_tail.len() > MAX_MUTABLE_TAIL_BYTES {
            let boundary = stable_boundary(&self.mutable_tail);
            if boundary > 0 {
                self.mutable_tail.drain(..boundary);
                self.stable_prefix_bytes = self.stable_prefix_bytes.saturating_add(boundary);
            }
        }
        self.work.tail_parses = self.work.tail_parses.saturating_add(1);
        true
    }

    pub fn replace_from_stable_boundary(
        &mut self,
        session_id: &str,
        generation: u64,
        message_idx: usize,
        content: &str,
    ) -> bool {
        if self.session_id != session_id
            || self.generation != generation
            || self.message_idx != Some(message_idx)
        {
            return false;
        }
        self.revision = self.revision.saturating_add(1);
        self.reproject_tail(content);
        true
    }

    pub fn complete(&mut self, session_id: &str, generation: u64, content: &str) -> bool {
        if self.session_id != session_id || self.generation != generation {
            return false;
        }
        let _ = crate::message_view::render_cache::build_render_cache_entry(
            content, content, 0, true,
        );
        self.work.canonical_parses = self.work.canonical_parses.saturating_add(1);
        self.mutable_tail.clear();
        self.stable_prefix_bytes = content.len();
        true
    }

    #[cfg(test)]
    pub fn mutable_tail(&self) -> &str {
        &self.mutable_tail
    }
    #[cfg(test)]
    pub fn stable_prefix_bytes(&self) -> usize {
        self.stable_prefix_bytes
    }
    #[cfg(test)]
    pub fn revision(&self) -> u64 {
        self.revision
    }
    pub fn work(&self) -> StreamRenderWork {
        self.work
    }

    fn reproject_tail(&mut self, content: &str) {
        let search_start =
            next_char_boundary(content, content.len().saturating_sub(MAX_MUTABLE_TAIL_BYTES));
        let candidate = &content[search_start..];
        self.stable_prefix_bytes = search_start + stable_boundary(candidate);
        self.mutable_tail.clear();
        self.mutable_tail.push_str(&content[self.stable_prefix_bytes..]);
        self.work.tail_parses = self.work.tail_parses.saturating_add(1);
    }
}

fn next_char_boundary(text: &str, mut index: usize) -> usize {
    while index < text.len() && !text.is_char_boundary(index) {
        index += 1;
    }
    index
}

fn stable_boundary(text: &str) -> usize {
    let mut boundary = 0usize;
    let mut fenced = false;
    let mut thinking = false;
    let mut tool_payload = false;
    let mut offset = 0usize;
    for line in text.split_inclusive('\n') {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            fenced = !fenced;
        }
        if trimmed.contains("<think") && !trimmed.contains("</think") {
            thinking = true;
        }
        if trimmed.contains("</think") {
            thinking = false;
        }
        if trimmed.starts_with("tool ") {
            tool_payload = true;
        }
        offset += line.len();
        if !fenced && !thinking && !tool_payload {
            boundary = offset;
        }
        if tool_payload && serde_json::from_str::<serde_json::Value>(trimmed).is_ok() {
            tool_payload = false;
            if !fenced && !thinking {
                boundary = offset;
            }
        }
    }
    boundary
}

/// Whether a summary still needs animation at the supplied monotonic timestamp.
pub fn has_active_explore_summary_animation(
    states: &std::collections::HashMap<u128, desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState>,
    now_ms: u64,
    duration_ms: u64,
) -> bool {
    states.values().any(|state| state.changed_at_ms.is_some_and(|changed_at_ms| now_ms.saturating_sub(changed_at_ms) < duration_ms))
}

pub fn advance_explore_summary_animation(
    states: &mut std::collections::HashMap<u128, desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState>,
    now_ms: u64,
    duration_ms: u64,
) {
    for state in states.values_mut() {
        if let Some(changed_at_ms) = state.changed_at_ms
            && now_ms.saturating_sub(changed_at_ms) >= duration_ms
        {
            state.previous_summary_text = state.current_summary_text.clone();
            state.changed_at_ms = None;
        }
    }
}

pub fn advance_status_animation_frame(
    frame: &mut usize,
    states: &mut std::collections::HashMap<u128, desktop_chat_tools::tools::explore_input::ExploreSummaryAnimationState>,
    now_ms: u64, duration_ms: u64,
) {
    *frame = frame.wrapping_add(1);
    advance_explore_summary_animation(states, now_ms, duration_ms);
}
