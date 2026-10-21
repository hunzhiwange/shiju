//! Incremental projection state for one streaming assistant message.

const MAX_MUTABLE_TAIL_BYTES: usize = 16 * 1024;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct StreamRenderWork {
    pub(crate) full_message_clones: usize,
    pub(crate) full_content_hashes: usize,
    pub(crate) full_content_parses: usize,
    pub(crate) height_index_rebuilds: usize,
    pub(crate) tail_parses: usize,
    pub(crate) canonical_parses: usize,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct StreamRenderState {
    session_id: String,
    generation: u64,
    message_idx: Option<usize>,
    stable_prefix_bytes: usize,
    mutable_tail: String,
    revision: u64,
    work: StreamRenderWork,
}

impl StreamRenderState {
    pub(crate) fn begin(
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
    pub(crate) fn append(
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

    pub(crate) fn append_delta(
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

    pub(crate) fn replace_from_stable_boundary(
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

    pub(crate) fn complete(&mut self, session_id: &str, generation: u64, content: &str) -> bool {
        if self.session_id != session_id || self.generation != generation {
            return false;
        }
        let _ = crate::app::components::chat_panel::message_view::build_render_cache_entry(
            content, content, 0, true,
        );
        self.work.canonical_parses = self.work.canonical_parses.saturating_add(1);
        self.mutable_tail.clear();
        self.stable_prefix_bytes = content.len();
        true
    }

    #[cfg(test)]
    pub(crate) fn mutable_tail(&self) -> &str {
        &self.mutable_tail
    }
    #[cfg(test)]
    pub(crate) fn stable_prefix_bytes(&self) -> usize {
        self.stable_prefix_bytes
    }
    #[cfg(test)]
    pub(crate) fn revision(&self) -> u64 {
        self.revision
    }
    #[cfg(test)]
    pub(crate) fn work(&self) -> StreamRenderWork {
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
