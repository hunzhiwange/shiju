//! 任务执行网关输出过滤。

#[derive(Default)]
pub(super) struct GatewayVisibleOutputFilter {
    buffered: String,
    in_reasoning: bool,
    finished: bool,
}

impl GatewayVisibleOutputFilter {
    const REASONING_OPEN: &'static str = "<think>";
    const REASONING_CLOSE: &'static str = "</think>";

    pub(super) fn push(&mut self, delta: &str) -> String {
        if self.finished {
            return String::new();
        }
        self.buffered.push_str(delta);
        self.drain(false)
    }

    pub(super) fn finish(&mut self) -> String {
        if self.finished {
            return String::new();
        }
        self.finished = true;
        self.drain(true)
    }

    fn drain(&mut self, force: bool) -> String {
        let mut visible = String::new();

        loop {
            if self.in_reasoning {
                if let Some(close_start) = self.buffered.find(Self::REASONING_CLOSE) {
                    self.buffered.drain(..close_start + Self::REASONING_CLOSE.len());
                    self.in_reasoning = false;
                    continue;
                }
                if force {
                    self.buffered.clear();
                } else {
                    Self::retain_possible_tag_prefix(&mut self.buffered, Self::REASONING_CLOSE);
                }
                break;
            }

            if let Some(open_start) = self.buffered.find(Self::REASONING_OPEN) {
                visible.push_str(&self.buffered[..open_start]);
                self.buffered.drain(..open_start + Self::REASONING_OPEN.len());
                self.in_reasoning = true;
                continue;
            }

            if force {
                visible.push_str(&self.buffered);
                self.buffered.clear();
            } else {
                let retained = Self::possible_tag_prefix_len(&self.buffered, Self::REASONING_OPEN);
                let visible_end = self.buffered.len().saturating_sub(retained);
                visible.push_str(&self.buffered[..visible_end]);
                self.buffered.drain(..visible_end);
            }
            break;
        }

        visible
    }

    fn retain_possible_tag_prefix(buffer: &mut String, tag: &str) {
        let retained = Self::possible_tag_prefix_len(buffer, tag);
        let discard_end = buffer.len().saturating_sub(retained);
        buffer.drain(..discard_end);
    }

    fn possible_tag_prefix_len(value: &str, tag: &str) -> usize {
        let max_len = value.len().min(tag.len().saturating_sub(1));
        (1..=max_len)
            .rev()
            .find(|length| value.as_bytes()[value.len() - length..] == tag.as_bytes()[..*length])
            .unwrap_or_default()
    }
}
