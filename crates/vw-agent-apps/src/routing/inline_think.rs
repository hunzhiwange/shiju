//! Chat 兼容上游把思考内容内联进 content 时的剥离逻辑。
//!
//! Claude 路径（Chat → Anthropic）与 Codex 路径（Chat → Responses）共用：
//! 流式走 [`InlineThinkSplitter`]，非流式走 [`split_leading_think_block`]。
//! 只识别正文开头的块。

/// 内联思考标签：`<think>`（MiniMax M3 等）与 `<thinking>`（DeepSeek 系等）。
const INLINE_THINK_TAG_PAIRS: [(&str, &str); 2] =
    [("<think>", "</think>"), ("<thinking>", "</thinking>")];

/// 标签与内容之间的分隔符（开标签之后、闭合标签前后）。
/// 只认换行：空格、制表符可能是正文的缩进。
const THINK_SEPARATORS: [char; 2] = ['\r', '\n'];

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum InlineThinkMode {
    #[default]
    Detecting,
    Reasoning,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ThinkPrefixDecision {
    NeedMore,
    Reasoning,
    Text,
}

/// 判断缓冲区开头是否最终会构成某个 think 开标签。
fn leading_think_prefix_decision(buffer: &str) -> ThinkPrefixDecision {
    let trimmed = buffer.trim_start();
    if trimmed.is_empty() {
        return ThinkPrefixDecision::NeedMore;
    }

    if INLINE_THINK_TAG_PAIRS.iter().any(|(open_tag, _)| trimmed.starts_with(open_tag)) {
        return ThinkPrefixDecision::Reasoning;
    }

    if INLINE_THINK_TAG_PAIRS.iter().any(|(open_tag, _)| open_tag.starts_with(trimmed)) {
        return ThinkPrefixDecision::NeedMore;
    }

    ThinkPrefixDecision::Text
}

/// 剥掉开头的空白与开标签，返回标签之后的内容；开头不是开标签时返回 None。
fn strip_leading_think_open_tag(text: &str) -> Option<&str> {
    let after_ws = text.trim_start();
    INLINE_THINK_TAG_PAIRS.iter().find_map(|(open_tag, _)| after_ws.strip_prefix(open_tag))
}

/// 在文本里找最早出现的闭合标签。容忍开闭不配对（如 `<think>` 开、`</thinking>` 闭）：
/// 只认配对闭合会让整段连正文一起被当成推理吞掉。
fn find_think_close_tag(text: &str) -> Option<(usize, &'static str)> {
    INLINE_THINK_TAG_PAIRS
        .iter()
        .filter_map(|(_, close_tag)| text.find(close_tag).map(|index| (index, *close_tag)))
        .min_by_key(|(index, _)| *index)
}

/// Reasoning 态下需扣住不下发的尾部长度：末尾可能是某个闭合标签真前缀的
/// 最长后缀，连同它前面的换行（闭合标签到了就是分隔符，没到再补发）。
fn reasoning_holdback_len(buffer: &str) -> usize {
    let partial_close_tag = INLINE_THINK_TAG_PAIRS
        .iter()
        .filter_map(|(_, close_tag)| {
            (1..close_tag.len()).rev().find(|len| buffer.ends_with(&close_tag[..*len]))
        })
        .max()
        .unwrap_or(0);
    let body = &buffer[..buffer.len() - partial_close_tag];
    buffer.len() - body.trim_end_matches(THINK_SEPARATORS).len()
}

fn non_empty(text: &str) -> Option<String> {
    if text.is_empty() { None } else { Some(text.to_string()) }
}

/// 非流式：把开头的 think 块拆成 (思考, 正文)。开头不是 think 块或块未闭合时返回 None。
pub fn split_leading_think_block(text: &str) -> Option<(String, String)> {
    let body = strip_leading_think_open_tag(text)?;
    let (close_start, close_tag) = find_think_close_tag(body)?;
    let answer = body[close_start + close_tag.len()..].trim_start_matches(THINK_SEPARATORS);

    let thinking = body[..close_start].trim_matches(THINK_SEPARATORS);

    Some((thinking.to_string(), answer.to_string()))
}

/// 流式：跨 chunk 剥离正文开头的 think 块。
///
/// Detecting 扣住可能构成开标签的前缀；Reasoning 即时下发思考内容、只扣住
/// 可能是闭合标签前缀的短尾；Text 透传正文。紧挨标签的换行是分隔符，不下发。
/// 思考内容不能整段缓冲到闭合标签才下发：外层的静默超时量的是转换后的流，
/// 长思考期间没有输出会被判成上游卡死。
#[derive(Debug, Default)]
pub struct InlineThinkSplitter {
    mode: InlineThinkMode,
    /// Detecting: 流首前缀缓冲；Reasoning: 扣住的短尾。
    buffer: String,
    /// 标签刚结束，后续增量开头的换行仍是分隔符。
    /// 跨增量保持，保证同一响应不随 SSE 分块方式产生不同输出。
    strip_separator: bool,
}

impl InlineThinkSplitter {
    /// 喂入一个 content 增量，返回 (thinking 增量, 正文增量)。
    pub fn push(&mut self, delta: &str) -> (Option<String>, Option<String>) {
        match self.mode {
            InlineThinkMode::Text => (None, self.push_text(delta)),
            InlineThinkMode::Detecting => {
                self.buffer.push_str(delta);
                match leading_think_prefix_decision(&self.buffer) {
                    ThinkPrefixDecision::NeedMore => (None, None),
                    ThinkPrefixDecision::Reasoning => {
                        self.mode = InlineThinkMode::Reasoning;
                        self.strip_separator = true;
                        let rest = strip_leading_think_open_tag(&self.buffer)
                            .unwrap_or_default()
                            .to_string();
                        self.buffer = rest;
                        self.drain_reasoning()
                    }
                    ThinkPrefixDecision::Text => {
                        self.mode = InlineThinkMode::Text;
                        let text = std::mem::take(&mut self.buffer);
                        (None, non_empty(&text))
                    }
                }
            }
            InlineThinkMode::Reasoning => {
                self.buffer.push_str(delta);
                self.drain_reasoning()
            }
        }
    }

    fn push_text(&mut self, delta: &str) -> Option<String> {
        if !self.strip_separator {
            return non_empty(delta);
        }
        let rest = delta.trim_start_matches(THINK_SEPARATORS);
        if rest.is_empty() {
            // 整个增量都是分隔符，正文还没开始
            return None;
        }
        self.strip_separator = false;
        Some(rest.to_string())
    }

    /// Reasoning 态推进：闭合标签完整出现 → 立即拆块转 Text；
    /// 否则除扣住的短尾外，思考内容全部即时下发。
    fn drain_reasoning(&mut self) -> (Option<String>, Option<String>) {
        if self.strip_separator {
            let separators =
                self.buffer.len() - self.buffer.trim_start_matches(THINK_SEPARATORS).len();
            self.buffer.drain(..separators);
            if self.buffer.is_empty() {
                return (None, None);
            }
            self.strip_separator = false;
        }

        if let Some((close_start, close_tag)) = find_think_close_tag(&self.buffer) {
            let buffered = std::mem::take(&mut self.buffer);
            self.mode = InlineThinkMode::Text;
            self.strip_separator = true;
            let thinking = non_empty(buffered[..close_start].trim_end_matches(THINK_SEPARATORS));
            let text = self.push_text(&buffered[close_start + close_tag.len()..]);
            (thinking, text)
        } else {
            let holdback = reasoning_holdback_len(&self.buffer);
            let split_at = self.buffer.len() - holdback;
            let thinking: String = self.buffer.drain(..split_at).collect();
            (non_empty(&thinking), None)
        }
    }

    /// 边界（工具调用 / finish_reason / [DONE] / 截断 EOF / 错误）冲刷残留，幂等：
    /// 未闭合的 think 块按思考内容原样下发（保住已收到的载荷），
    /// Detecting 缓冲按正文下发。
    pub fn flush(&mut self) -> (Option<String>, Option<String>) {
        let buffered = std::mem::take(&mut self.buffer);
        let was_reasoning = self.mode == InlineThinkMode::Reasoning;
        self.mode = InlineThinkMode::Text;
        self.strip_separator = false;
        if was_reasoning {
            // push 已即时拆块，此处残留不含完整闭合标签
            (non_empty(&buffered), None)
        } else {
            (None, non_empty(&buffered))
        }
    }

    /// 是否还扣着未下发的内容（包括未闭合尾部的换行）。
    pub fn has_pending_content(&self) -> bool {
        !self.buffer.is_empty()
    }
}
