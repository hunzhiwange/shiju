//! TUI 转录管线使用的流式原语。
//!
//! `StreamState` 管理以换行为提交门槛的 Markdown 收集器和先进先出渲染行队列。
//! 控制器把队列行转为历史单元，分块策略计算排空计划，提交节拍负责应用计划。
//! 所有排空只从队首取出；入队时间用于策略判断最老积压年龄而不检查正文。
//! Streaming primitives used by the TUI transcript pipeline.
//!
//! `StreamState` owns newline-gated markdown collection and a FIFO queue of committed render lines.
//! Higher-level modules build on top of this state:
//! - `controller` adapts queued lines into `HistoryCell` emission rules for message and plan streams.
//! - `chunking` computes adaptive drain plans from queue pressure.
//! - `commit_tick` binds policy decisions to concrete controller drains.
//!
//! The key invariant is queue ordering. All drains pop from the front, and enqueue records an
//! arrival timestamp so policy code can reason about oldest queued age without peeking into text.

use std::collections::VecDeque;
use std::path::Path;
use std::time::Duration;
use std::time::Instant;

use crate::markdown_stream::MarkdownStreamCollector;
use crate::terminal_hyperlinks::HyperlinkLine;
use crate::terminal_hyperlinks::prefix_hyperlink_lines;
use crate::terminal_hyperlinks::visible_lines;
pub(crate) mod chunking;
mod code_fence;
pub(crate) mod commit_tick;
pub(crate) mod controller;
mod render;
mod table_holdback;

/// 在历史单元接入前由隔离流式管线拥有的渲染模式。
/// Rendering mode owned by the isolated streaming pipeline until history cells
/// are activated in batch 6.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum StreamRenderMode {
    Rich,
    Raw,
}

/// 流排空产生的纯 UI 输出；它有意只拥有渲染行，不持有协议或网关状态。
/// Pure UI output from a stream drain. It intentionally owns rendered lines
/// and no protocol or Gateway state.
#[derive(Debug)]
pub(crate) struct StreamingCell {
    lines: Vec<HyperlinkLine>,
    agent_prefix: Option<bool>,
}

impl StreamingCell {
    fn agent(lines: Vec<HyperlinkLine>, is_first_line: bool) -> Self {
        Self { lines, agent_prefix: Some(is_first_line) }
    }

    fn plan(lines: Vec<HyperlinkLine>) -> Self {
        Self { lines, agent_prefix: None }
    }

    pub(crate) fn display_hyperlink_lines(&self, _width: u16) -> Vec<HyperlinkLine> {
        match self.agent_prefix {
            Some(is_first_line) => prefix_hyperlink_lines(
                self.lines.clone(),
                if is_first_line { "• ".into() } else { "  ".into() },
                "  ".into(),
            ),
            None => self.lines.clone(),
        }
    }

    pub(crate) fn transcript_lines(&self, width: u16) -> Vec<ratatui::text::Line<'static>> {
        visible_lines(self.display_hyperlink_lines(width))
    }
}

fn raw_lines_from_source(source: &str) -> Vec<ratatui::text::Line<'static>> {
    if source.is_empty() {
        return Vec::new();
    }
    let mut parts = source.split('\n').collect::<Vec<_>>();
    if source.ends_with('\n') {
        parts.pop();
    }
    parts.into_iter().map(|line| ratatui::text::Line::from(line.to_string())).collect()
}

struct QueuedLine {
    line: HyperlinkLine,
    enqueued_at: Instant,
}

/// 持有进行中的 Markdown 流状态以及排队等待提交的行。
/// Holds in-flight markdown stream state and queued committed lines.
pub(crate) struct StreamState {
    pub(crate) collector: MarkdownStreamCollector,
    queued_lines: VecDeque<QueuedLine>,
    pub(crate) has_seen_delta: bool,
}

impl StreamState {
    /// 创建流状态，使 Markdown 收集器相对 `cwd` 渲染本地文件链接。
    ///
    /// 控制器应在生命周期开始时传入一次会话目录，并在活动流期间保持稳定。
    /// Create stream state whose markdown collector renders local file links relative to `cwd`.
    ///
    /// Controllers are expected to pass the session cwd here once and keep it stable for the
    /// lifetime of the active stream.
    pub(crate) fn new(width: Option<usize>, cwd: &Path) -> Self {
        Self {
            collector: MarkdownStreamCollector::new(width, cwd),
            queued_lines: VecDeque::new(),
            has_seen_delta: false,
        }
    }
    /// 为下一个流生命周期重置收集器与队列状态。
    /// Resets collector and queue state for the next stream lifecycle.
    pub(crate) fn clear(&mut self) {
        self.collector.clear();
        self.queued_lines.clear();
        self.has_seen_delta = false;
    }
    /// 从队首排空一条已排队行。
    /// Drains one queued line from the front of the queue.
    pub(crate) fn step(&mut self) -> Vec<HyperlinkLine> {
        self.queued_lines.pop_front().map(|queued| queued.line).into_iter().collect()
    }
    /// 从队首最多排空 `max_lines` 条已排队行。
    ///
    /// 即使传入极大值，也会夹取到当前队列长度以保持有界行为。
    /// Drains up to `max_lines` queued lines from the front of the queue.
    ///
    /// Callers that pass very large values still get bounded behavior because this method clamps to
    /// the currently available queue length.
    pub(crate) fn drain_n(&mut self, max_lines: usize) -> Vec<HyperlinkLine> {
        let end = max_lines.min(self.queued_lines.len());
        self.queued_lines.drain(..end).map(|queued| queued.line).collect()
    }
    /// 清空排队行，但保留收集器和轮次生命周期状态。
    /// Clears queued lines while keeping collector/turn lifecycle state intact.
    pub(crate) fn clear_queue(&mut self) {
        self.queued_lines.clear();
    }
    /// 返回当前是否没有等待提交的行。
    /// Returns whether no lines are queued for commit.
    pub(crate) fn is_idle(&self) -> bool {
        self.queued_lines.is_empty()
    }
    /// 返回当前队列深度。
    /// Returns the current queue depth.
    pub(crate) fn queued_len(&self) -> usize {
        self.queued_lines.len()
    }
    /// 返回最老已排队行的等待时长。
    /// Returns the age of the oldest queued line.
    pub(crate) fn oldest_queued_age(&self, now: Instant) -> Option<Duration> {
        self.queued_lines.front().map(|queued| now.saturating_duration_since(queued.enqueued_at))
    }
    /// 使用同一入队时间戳把已提交行追加到队列。
    /// Appends committed lines to the queue with a shared enqueue timestamp.
    pub(crate) fn enqueue(&mut self, lines: Vec<HyperlinkLine>) {
        let now = Instant::now();
        self.queued_lines
            .extend(lines.into_iter().map(|line| QueuedLine { line, enqueued_at: now }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use ratatui::text::Line;
    use std::path::PathBuf;

    fn test_cwd() -> PathBuf {
        // 这些测试只需要稳定的绝对目录；使用临时目录可避免固化 Unix 或 Windows 根路径语义。
        // These tests only need a stable absolute cwd; using temp_dir() avoids baking Unix- or
        // Windows-specific root semantics into the fixtures.
        std::env::temp_dir()
    }

    #[test]
    fn drain_n_clamps_to_available_lines() {
        let mut state = StreamState::new(/*width*/ None, &test_cwd());
        state.enqueue(vec![HyperlinkLine::new(Line::from("one"))]);

        let drained = state.drain_n(/*max_lines*/ 8);
        assert_eq!(drained, vec![HyperlinkLine::new(Line::from("one"))]);
        assert!(state.is_idle());
    }
}
