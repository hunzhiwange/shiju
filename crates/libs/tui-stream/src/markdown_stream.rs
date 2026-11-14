//! 在换行边界收集 Markdown 流源码。
//!
//! 收集器缓冲令牌增量，并只在换行处暴露可提交前缀；未完成尾行留给下一增量。
//! 定稿时转移完整源码，包括可能没有尾随换行的最后一行。
//! 生产路径不在此解析 Markdown，只定义稳定源码边界；渲染由控制器按当前宽度完成。
//! Collects markdown stream source at newline boundaries.
//!
//! `MarkdownStreamCollector` buffers incoming token deltas and exposes a commit boundary at each
//! newline. The stream controllers (`streaming/controller.rs`) call `commit_complete_source()`
//! after each newline-bearing delta to obtain the completed prefix for re-rendering, leaving the
//! trailing incomplete line in the buffer for the next delta.
//!
//! On finalization, `finalize_and_take_source()` transfers the complete source buffer to the
//! controller (including a final line that may lack a trailing newline).

#[cfg(test)]
use ratatui::text::Line;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

#[cfg(test)]
use tui_markdown::markdown;

/// 以换行为门槛、只提交完整行的原始 Markdown 源码累加器。
///
/// `committed_source_len` 记录已返回的源码字节，因此每次提交只产生新完成的范围。
/// 控制器可重渲染完整累计源码，同时只追加新稳定内容；收集器本身不承担 Markdown 解析。
/// Newline-gated accumulator that buffers raw markdown source and commits only completed lines.
///
/// The buffer tracks how many source bytes have already been committed via
/// `committed_source_len`, so each `commit_complete_source()` call returns the range of the newly
/// completed portion. This design lets the stream controller re-render the entire accumulated
/// source while only appending new content.
///
/// The collector does not parse markdown in production. It only defines stable source boundaries;
/// rendering lives in the stream controllers so width changes can re-render from one accumulated
/// source string.
pub struct MarkdownStreamCollector {
    buffer: String,
    committed_source_len: usize,
    #[cfg(test)]
    committed_line_count: usize,
    width: Option<usize>,
    #[cfg(test)]
    cwd: PathBuf,
}

impl MarkdownStreamCollector {
    /// 创建累积原始 Markdown 增量的收集器。
    ///
    /// 宽度与工作目录只供测试渲染辅助使用；生产提交仅处理原始源码边界。
    /// 工作目录会被快照，使测试中的本地链接在增量提交间保持稳定。
    /// Create a collector that accumulates raw markdown deltas.
    ///
    /// `width` and `cwd` are only used by test-only rendering helpers; production stream commits
    /// operate on raw source boundaries. The collector snapshots `cwd` so test rendering keeps
    /// local file-link display stable across incremental commits.
    pub fn new(width: Option<usize>, cwd: &Path) -> Self {
        #[cfg(not(test))]
        let _ = cwd;

        Self {
            buffer: String::new(),
            committed_source_len: 0,
            #[cfg(test)]
            committed_line_count: 0,
            width,
            #[cfg(test)]
            cwd: cwd.to_path_buf(),
        }
    }

    /// 更新仅测试行提交辅助函数使用的渲染宽度。
    /// Update the rendering width used by test-only line-commit helpers.
    pub fn set_width(&mut self, width: Option<usize>) {
        self.width = width;
    }

    /// 重置全部已缓冲源码与提交记账状态。
    /// Reset all buffered source and commit bookkeeping.
    pub fn clear(&mut self) {
        self.buffer.clear();
        self.committed_source_len = 0;
        #[cfg(test)]
        {
            self.committed_line_count = 0;
        }
    }

    /// 向内部源码缓冲区追加原始流式增量。
    /// Append a raw streaming delta to the internal source buffer.
    pub fn push_delta(&mut self, delta: &str) {
        tracing::trace!("push_delta: {delta:?}");
        self.buffer.push_str(delta);
    }

    /// 提交到最后一个换行为止的新完成原始 Markdown 源码。
    ///
    /// 返回此前未提交的源码范围；没有换行的增量返回空，防止实时流提前渲染语义未定的块。
    /// Commit newly completed raw markdown source up to the last newline.
    ///
    /// This returns the range of source that has not been returned by a previous commit. Calling it
    /// after a delta without a newline returns `None`, which prevents the live stream from rendering
    /// incomplete markdown blocks that may change meaning when the rest of the line arrives.
    pub fn commit_complete_source(&mut self) -> Option<std::ops::Range<usize>> {
        let commit_end = self.buffer.rfind('\n').map(|idx| idx + 1)?;
        let commit_start = self.committed_source_len;
        if commit_end <= commit_start {
            return None;
        }

        self.committed_source_len = commit_end;
        Some(commit_start..commit_end)
    }

    /// 返回以换行终止、可安全渲染的源码。
    /// Return the newline-terminated source that is safe to render.
    pub fn committed_source(&self) -> &str {
        &self.buffer[..self.committed_source_len]
    }

    /// 定稿流并转移完整原始源码。
    ///
    /// 非空返回分块会确保换行终止，使调用者可安全执行 Markdown 块解析。
    /// 此操作清空收集器，只应在流真正结束或有意合并中断输出时调用。
    /// Finalize the stream and transfer its complete raw source.
    ///
    /// Ensures the returned source chunk is newline-terminated when non-empty so callers can
    /// safely run markdown block parsing on the final chunk. This method clears the collector;
    /// callers should not invoke it until the stream is truly complete or interrupted output is
    /// being intentionally consolidated.
    pub fn finalize_and_take_source(&mut self) -> String {
        let mut out = std::mem::take(&mut self.buffer);
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        self.clear();
        out
    }

    /// 渲染完整缓冲区，并只返回上次提交后新完成的逻辑行。
    /// 缓冲区没有换行结尾时，最后一条渲染行视为未完成且不发出。
    ///
    /// 该测试辅助有意使用 `append_markdown`，隔离收集器换行边界行为，不引入控制器暂留语义。
    /// Render the full buffer and return only the newly completed logical lines
    /// since the last commit. When the buffer does not end with a newline, the
    /// final rendered line is considered incomplete and is not emitted.
    ///
    /// This helper intentionally uses `append_markdown` (not
    /// `append_markdown_agent`) so tests can isolate collector newline boundary
    /// behavior without stream-controller holdback semantics.
    #[cfg(test)]
    pub fn commit_complete_lines(&mut self) -> Vec<Line<'static>> {
        let Some(commit_end) = self.buffer.rfind('\n').map(|idx| idx + 1) else {
            return Vec::new();
        };
        if commit_end <= self.committed_source_len {
            return Vec::new();
        }
        let source = self.buffer[..commit_end].to_string();
        let mut rendered: Vec<Line<'static>> = Vec::new();
        markdown::append_markdown(&source, self.width, Some(self.cwd.as_path()), &mut rendered);
        let mut complete_line_count = rendered.len();
        if complete_line_count > 0
            && crate::test_helpers::is_blank_line_spaces_only(
                &rendered[complete_line_count - 1],
            )
        {
            complete_line_count -= 1;
        }

        if self.committed_line_count >= complete_line_count {
            return Vec::new();
        }

        let out_slice = &rendered[self.committed_line_count..complete_line_count];

        let out = out_slice.to_vec();
        self.committed_source_len = commit_end;
        self.committed_line_count = complete_line_count;
        out
    }

    /// 定稿流：发出上次提交之后的全部剩余行。
    /// 缓冲区没有换行结尾时，会临时追加换行供渲染。
    /// Finalize the stream: emit all remaining lines beyond the last commit.
    /// If the buffer does not end with a newline, a temporary one is appended
    /// for rendering.
    #[cfg(test)]
    pub fn finalize_and_drain(&mut self) -> Vec<Line<'static>> {
        let mut source = self.buffer.clone();
        if source.is_empty() {
            self.clear();
            return Vec::new();
        }
        if !source.ends_with('\n') {
            source.push('\n');
        };
        tracing::debug!(
            raw_len = self.buffer.len(),
            source_len = source.len(),
            "markdown finalize (raw length: {}, rendered length: {})",
            self.buffer.len(),
            source.len()
        );
        tracing::trace!("markdown finalize (raw source):\n---\n{source}\n---");

        let mut rendered: Vec<Line<'static>> = Vec::new();
        markdown::append_markdown(&source, self.width, Some(self.cwd.as_path()), &mut rendered);

        let out = if self.committed_line_count >= rendered.len() {
            Vec::new()
        } else {
            rendered[self.committed_line_count..].to_vec()
        };

        // 为下一条流重置收集器状态。
        // Reset collector state for next stream.
        self.clear();
        out
    }
}

#[cfg(test)]
fn test_cwd() -> PathBuf {
    // 这些测试只需要稳定绝对目录；使用临时目录避免固化 Unix 或 Windows 根路径语义。
    // These tests only need a stable absolute cwd; using temp_dir() avoids baking Unix- or
    // Windows-specific root semantics into the fixtures.
    std::env::temp_dir()
}

#[cfg(test)]
pub(crate) fn simulate_stream_markdown_for_tests(
    deltas: &[&str],
    finalize: bool,
) -> Vec<Line<'static>> {
    let mut collector = MarkdownStreamCollector::new(/*width*/ None, &test_cwd());
    let mut out = Vec::new();
    for d in deltas {
        collector.push_delta(d);
        if d.contains('\n') {
            out.extend(collector.commit_complete_lines());
        }
    }
    if finalize {
        out.extend(collector.finalize_and_drain());
    }
    out
}

#[cfg(test)]
#[path = "markdown_stream_tests.rs"]
mod tests;
