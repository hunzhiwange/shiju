const STREAM_TAIL_BASE_HEIGHT: f32 = 78.0;

#[derive(Debug, Clone)]
pub struct StreamTailState {
    message_idx: Option<usize>,
    processed_bytes: usize,
    explicit_lines: usize,
    think_count: usize,
    thinking_open: bool,
    incomplete_tag: String,
    tool_match_tail: String,
    tool_count: usize,
    estimated_height: f32,
    ui_generation: u64,
    ui_preparing_generation: Option<u64>,
}

impl Default for StreamTailState {
    fn default() -> Self {
        Self {
            message_idx: None,
            processed_bytes: 0,
            explicit_lines: 1,
            think_count: 0,
            thinking_open: false,
            incomplete_tag: String::new(),
            tool_match_tail: String::new(),
            tool_count: 0,
            estimated_height: STREAM_TAIL_BASE_HEIGHT,
            ui_generation: 0,
            ui_preparing_generation: None,
        }
    }
}

impl StreamTailState {
    pub fn begin_message(&mut self, message_idx: usize) {
        if self.message_idx == Some(message_idx) {
            return;
        }
        let ui_generation = self.ui_generation;
        *self = Self { message_idx: Some(message_idx), ui_generation, ..Self::default() };
    }

    pub fn apply_delta(&mut self, delta: &str) {
        if delta.is_empty() {
            return;
        }
        self.processed_bytes = self.processed_bytes.saturating_add(delta.len());
        self.explicit_lines =
            self.explicit_lines.saturating_add(delta.bytes().filter(|byte| *byte == b'\n').count());
        self.scan_think_tags(delta);
        self.scan_tool_markers(delta);
        self.recalculate_height();
    }

    pub fn message_idx(&self) -> Option<usize> {
        self.message_idx
    }

    #[cfg(test)]
    pub(crate) fn processed_bytes(&self) -> usize {
        self.processed_bytes
    }

    pub fn think_count(&self) -> usize {
        self.think_count
    }

    pub fn thinking_open(&self) -> bool {
        self.thinking_open
    }

    pub fn text_block_boundary(&self) -> (usize, bool, usize) {
        (self.think_count, self.thinking_open, self.tool_count)
    }

    pub fn estimated_height(&self) -> f32 {
        self.estimated_height
    }

    pub fn ui_generation(&self) -> u64 {
        self.ui_generation
    }

    pub fn advance_ui_generation(&mut self) -> u64 {
        self.ui_generation = self.ui_generation.wrapping_add(1);
        self.ui_generation
    }

    pub fn begin_ui_preparation(&mut self) -> Option<u64> {
        if self.ui_preparing_generation.is_some() {
            return None;
        }
        self.ui_preparing_generation = Some(self.ui_generation);
        self.ui_preparing_generation
    }

    pub fn finish_ui_preparation(&mut self, generation: u64) {
        if self.ui_preparing_generation == Some(generation) {
            self.ui_preparing_generation = None;
        }
    }

    pub fn calibrate(
        &mut self,
        message_idx: usize,
        processed_bytes: usize,
        think_count: usize,
        thinking_open: bool,
        estimated_height: f32,
    ) {
        self.message_idx = Some(message_idx);
        self.processed_bytes = processed_bytes;
        self.think_count = think_count;
        self.thinking_open = thinking_open;
        self.estimated_height = estimated_height;
        self.incomplete_tag.clear();
    }

    pub fn clear_tail(&mut self) {
        let ui_generation = self.ui_generation;
        *self = Self { ui_generation, ..Self::default() };
    }

    fn scan_think_tags(&mut self, delta: &str) {
        let mut input = String::with_capacity(self.incomplete_tag.len() + delta.len());
        input.push_str(&self.incomplete_tag);
        input.push_str(delta);
        self.incomplete_tag.clear();

        let mut search_from = 0;
        while let Some(relative_start) = input[search_from..].find('<') {
            let start = search_from + relative_start;
            let candidate = &input[start..];
            let Some(relative_end) = candidate.find('>') else {
                if possible_incomplete_think_tag(candidate) {
                    self.incomplete_tag.push_str(candidate);
                }
                break;
            };
            let end = start + relative_end + 1;
            match classify_think_tag(&input[start..end]) {
                Some(true) if !self.thinking_open => {
                    self.think_count = self.think_count.saturating_add(1);
                    self.thinking_open = true;
                }
                Some(false) if self.thinking_open => self.thinking_open = false,
                _ => {}
            }
            search_from = end;
        }
    }

    fn scan_tool_markers(&mut self, delta: &str) {
        let previous_tail_len = self.tool_match_tail.len();
        let mut input = String::with_capacity(previous_tail_len + delta.len());
        input.push_str(&self.tool_match_tail);
        input.push_str(delta);
        self.tool_count = self.tool_count.saturating_add(
            input
                .match_indices("tool ")
                .filter(|(start, _)| start.saturating_add("tool ".len()) > previous_tail_len)
                .count(),
        );
        self.tool_match_tail = trailing_chars(&input, "tool ".len() - 1).to_string();
    }

    fn recalculate_height(&mut self) {
        let wrapped_lines = self.processed_bytes.div_ceil(96).max(1);
        let line_count = self.explicit_lines.max(wrapped_lines) as f32;
        self.estimated_height = (line_count * 22.0
            + self.think_count as f32 * 28.0
            + self.tool_count as f32 * 36.0
            + 56.0)
            .clamp(64.0, 720.0);
    }
}

fn possible_incomplete_think_tag(candidate: &str) -> bool {
    ["<think", "<thinking", "</think", "</thinking"].iter().any(|tag| tag.starts_with(candidate))
        || candidate.starts_with("<think")
        || candidate.starts_with("</think")
}

fn classify_think_tag(tag: &str) -> Option<bool> {
    let inner = tag.strip_suffix('>')?;
    if let Some(remainder) = inner.strip_prefix("</think") {
        return think_tag_remainder_is_valid(remainder).then_some(false);
    }
    inner
        .strip_prefix("<think")
        .filter(|remainder| think_tag_remainder_is_valid(remainder))
        .map(|_| true)
}

fn think_tag_remainder_is_valid(remainder: &str) -> bool {
    remainder.is_empty()
        || remainder.chars().next().is_some_and(char::is_whitespace)
        || remainder.strip_prefix("ing").is_some_and(|after| {
            after.is_empty() || after.chars().next().is_some_and(char::is_whitespace)
        })
}

fn trailing_chars(value: &str, max_chars: usize) -> &str {
    value
        .char_indices()
        .rev()
        .nth(max_chars.saturating_sub(1))
        .map_or(value, |(start, _)| &value[start..])
}

impl StreamTailState {
    /// Complete one background preparation and schedule the latest tail if it changed meanwhile.
    pub fn finish_session_open_ui(&mut self, generation: u64, active: bool) -> Option<(usize, u64)> {
        self.finish_ui_preparation(generation);
        if self.ui_generation() == generation || !active { None }
        else { self.message_idx().zip(self.begin_ui_preparation()) }
    }
}
