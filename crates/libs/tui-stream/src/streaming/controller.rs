//! 助手消息与拟议计划使用的双区域流式控制器。
//!
//! 每个流把渲染结果拆成已进入动画队列的稳定区，以及由活动单元展示的可变尾区。
//! `StreamCore` 统一负责源码累积、增量重渲染、队列记账和终端缩放；两类外层控制器只加样式。
//! 表格新增行可能重塑全部列，因此从候选表头起暂留为尾部；非 Markdown 围栏内不扫描表格。
//! 缩放时按新宽度重建尚未发出的稳定区，定稿时再由源码驱动单元生成规范转录。
//! 核心不变量是已发出长度不超过已入队长度，且尾部精确从已入队边界开始。
//! Two-region streaming controllers for agent messages and proposed plans.
//!
//! Each stream partitions rendered markdown into a *stable region* (committed
//! to scrollback via the animation queue in `StreamState`) and a *tail region*
//! (mutable, displayed in the active-cell slot as a transient stream-tail cell).
//!
//! `StreamCore` owns the shared bookkeeping: source accumulation, re-rendering,
//! stable/tail partitioning, commit-animation queue management, and terminal
//! resize handling.  `StreamController` and `PlanStreamController` are thin
//! wrappers that add only their `emit()` styling and finalize return types.
//!
//! ## Table holdback
//!
//! Table rendering is inherently non-incremental: adding a new row can change
//! every column's width and reshape all prior rows.  The holdback mechanism
//! (`table_holdback_state`) detects pipe-table patterns (header + delimiter
//! pair) in the accumulated source and keeps content from the table header
//! onward as mutable tail until the stream finalizes. Holdback is enabled for
//! agent and proposed-plan streams. Lines in `Outside` and `Markdown` fence
//! contexts are scanned; lines inside non-markdown fences are skipped.
//!
//! ## Resize handling
//!
//! On terminal width change, `StreamCore::set_width` re-renders at the new
//! width and rebuilds the queued stable region from the current emitted line
//! count. This intentionally avoids byte-level remap complexity while the
//! stream is active; finalized content is canonicalized by transcript
//! consolidation into source-backed markdown cells.
//!
//! ## Invariants
//!
//! - `emitted_stable_len <= enqueued_stable_len <= render.lines.len()`.
//! - committed source is append-only until `reset()`; never modified mid-stream.
//! - Tail starts exactly at `enqueued_stable_len`.
//! - During confirmed table streaming, only lines from the table header onward
//!   are forced into tail; pre-table lines may remain stable.

use tui_markdown::markdown::render_markdown_agent_with_links_and_cwd;
use tui_theme::style::proposed_plan_style;
use tui_render::terminal_hyperlinks::HyperlinkLine;
use tui_render::terminal_hyperlinks::prefix_hyperlink_lines;
use ratatui::prelude::Stylize;
use ratatui::text::Line;
use std::path::Path;
use std::path::PathBuf;
use std::time::Duration;
use std::time::Instant;

use super::StreamRenderMode;
use super::StreamState;
use super::StreamingCell;
use super::render::StreamingRender;
use super::render::render_source;
use super::table_holdback::TableHoldbackScanner;
use super::table_holdback::TableHoldbackState;
#[cfg(test)]
use super::table_holdback::table_holdback_state;

// 以下为两类流式控制器共用的 `StreamCore` 记账逻辑。
// ---------------------------------------------------------------------------
// StreamCore — shared bookkeeping for both stream controllers
// ---------------------------------------------------------------------------

/// 双区域流式模型共用的状态与逻辑。
///
/// 助手消息和拟议计划控制器都委托这里完成源码累积、重渲染、稳定/尾部分区、
/// 提交动画队列管理与终端缩放；外层只提供各自样式和定稿返回类型。
/// Shared state and logic for the two-region streaming model.
///
/// Both [`StreamController`] (agent messages) and [`PlanStreamController`]
/// (proposed plans) delegate their core bookkeeping here: source
/// accumulation, re-rendering, stable/tail partitioning, commit-animation
/// queue management, and terminal resize handling.
///
/// The wrapping controllers add only their own `emit()` styling and
/// finalize return types.
struct StreamCore {
    state: StreamState,
    /// 当前渲染宽度，即 Markdown 正文可用列数。
    /// Current rendering width (columns available for markdown content).
    width: Option<usize>,
    /// 已提交源码在当前宽度下的增量渲染。
    /// Incremental render of committed source at `width`.
    render: StreamingRender,
    /// 已进入提交动画队列的行数。
    /// Lines enqueued into the commit-animation queue.
    enqueued_stable_len: usize,
    /// 已实际发出到回滚历史的行数。
    /// Lines actually emitted to scrollback.
    emitted_stable_len: usize,
    /// 流重渲染期间用于稳定本地文件链接展示的会话目录。
    /// Session cwd used to keep local file-link display stable during stream re-renders.
    cwd: PathBuf,
    render_mode: StreamRenderMode,
    /// 按源码起点与宽度缓存的表格前缀渲染行数。
    /// Cached rendered line count for prefix-before-table keyed by source start and width.
    stable_prefix_len_cache: Option<StablePrefixLenCache>,
    /// 只追加源码更新使用的增量暂留扫描器状态。
    /// Incremental holdback scanner state for append-only source updates.
    holdback_scanner: TableHoldbackScanner,
}

struct StablePrefixLenCache {
    /// 已提交源码中候选表格或表头起点的字节偏移。
    /// Byte offset of the candidate table/header start in committed source.
    source_start: usize,
    /// 生成 `stable_prefix_len` 时使用的宽度。
    /// Width that produced `stable_prefix_len`.
    width: Option<usize>,
    /// 指定宽度下 `source_start` 之前已提交前缀的渲染行数。
    ///
    /// 实时表格尾部持续变化时复用该值，避免每条提交行都重渲染同一稳定前缀。
    /// Rendered line count for the committed prefix before `source_start` at `width`.
    ///
    /// The streaming controller uses this to avoid repeatedly re-rendering the
    /// same stable prefix while a live table tail is still mutating.
    stable_prefix_len: usize,
}

impl StreamCore {
    fn new(width: Option<usize>, cwd: &Path, render_mode: StreamRenderMode) -> Self {
        Self {
            state: StreamState::new(width, cwd),
            width,
            render: StreamingRender::new(),
            enqueued_stable_len: 0,
            emitted_stable_len: 0,
            cwd: cwd.to_path_buf(),
            render_mode,
            stable_prefix_len_cache: None,
            holdback_scanner: TableHoldbackScanner::new(),
        }
    }

    /// 推入流式增量，并把新近稳定的渲染行加入队列。
    ///
    /// 只有换行终止的源码才会提交。未终止的局部表格行必须留在收集器中，
    /// 否则用户可能短暂看到下一增量到达即消失的畸形列。
    /// Push a streaming delta and enqueue any newly-stable rendered lines.
    ///
    /// Only newline-terminated source is committed for rendering. This is
    /// important for tables because an unterminated partial row must stay out
    /// of both the stable queue and the live tail until its structure is
    /// unambiguous; otherwise the user can briefly see malformed columns that
    /// immediately disappear on the next delta.
    fn push_delta(&mut self, delta: &str) -> bool {
        if !delta.is_empty() {
            self.state.has_seen_delta = true;
        }
        self.state.collector.push_delta(delta);

        let mut enqueued = false;
        if delta.contains('\n')
            && let Some(range) = self.state.collector.commit_complete_source()
        {
            let source = self.state.collector.committed_source();
            let committed_source = &source[range];
            self.holdback_scanner.push_source_chunk(committed_source);
            self.render.append(
                source,
                committed_source,
                self.width,
                self.cwd.as_path(),
                self.render_mode,
            );
            enqueued = self.sync_stable_queue();
        }
        enqueued
    }

    /// 排空收集器，渲染最终源码快照，并返回尚未发出的行。
    ///
    /// 这里从完整原始源码重新渲染，而不拼接队列和尾部；最终结果是合并使用的规范转录。
    /// 调用者若跳过 `reset()`，可能把已完成流重放到下一条回答中。
    /// Drain the collector, render the final source snapshot, and return lines not yet emitted.
    ///
    /// This intentionally re-renders from the full raw source instead of
    /// trying to stitch together queued stable lines and the current tail. The
    /// final render is the canonical transcript representation used for
    /// consolidation, so callers that skip `reset()` can accidentally replay a
    /// finished stream into the next answer.
    fn finalize_remaining(&mut self) -> (Vec<HyperlinkLine>, String) {
        let source = self.state.collector.finalize_and_take_source();
        let mut rendered = render_source(&source, self.width, self.cwd.as_path(), self.render_mode);
        let remaining = rendered.split_off(self.emitted_stable_len.min(rendered.len()));
        (remaining, source)
    }

    /// 推进一步动画：出队一行并更新已发出计数。
    /// Step animation: dequeue one line, update the emitted count.
    fn tick(&mut self) -> Vec<HyperlinkLine> {
        let step = self.state.step();
        self.emitted_stable_len += step.len();
        step
    }

    /// 批量排空：最多出队 `max_lines` 行并更新已发出计数。
    /// Batch drain: dequeue up to `max_lines`, update the emitted count.
    fn tick_batch(&mut self, max_lines: usize) -> Vec<HyperlinkLine> {
        if max_lines == 0 {
            return Vec::new();
        }
        let step = self.state.drain_n(max_lines);
        if step.is_empty() {
            return step;
        }
        self.emitted_stable_len += step.len();
        step
    }

    // 简单 `StreamCore` 访问器内联，因为活动流的每个动画节拍和渲染帧都会调用。
    // Trivial StreamCore accessors inlined — called on every animation tick
    // and render frame during active streaming.

    #[inline]
    fn is_idle(&self) -> bool {
        self.state.is_idle()
    }

    #[inline]
    fn queued_lines(&self) -> usize {
        self.state.queued_len()
    }

    #[inline]
    fn oldest_queued_age(&self, now: Instant) -> Option<Duration> {
        self.state.oldest_queued_age(now)
    }

    /// 属于可变尾部、尚未排队稳定提交的行。
    ///
    /// 尾部从 `enqueued_stable_len` 开始；若错误地从已发出长度推导，
    /// 已入队但未发出的行会在活动单元中重现，造成屏幕重复。
    /// Lines that belong to the mutable tail, not yet queued for stable commit.
    ///
    /// The tail starts at `enqueued_stable_len`, so this returns the portion
    /// of the current render snapshot that is still allowed to change without
    /// violating scrollback ordering. If callers were to derive the tail from
    /// `emitted_stable_len` instead, queued-but-not-yet-emitted lines could
    /// reappear in the active cell and duplicate content on screen.
    #[inline]
    fn current_tail_lines(&self) -> Vec<HyperlinkLine> {
        let start = self.enqueued_stable_len.min(self.render.lines.len());
        self.render.lines[start..].to_vec()
    }

    #[inline]
    fn has_tail(&self) -> bool {
        self.enqueued_stable_len < self.render.lines.len()
    }

    /// 更新渲染宽度，并为新布局重建已排队稳定行。
    ///
    /// 新宽度只重渲染一次，然后从当前已发出行数重建队列。
    /// 缩放时既要保持已发出正文的回滚顺序，也允许实时表格尾部重新塑形，
    /// 此处维持该分界而不尝试逐字节映射行。
    /// Update rendering width and rebuild queued stable lines for the new layout.
    ///
    /// Re-renders once at the new width and rebuilds queue state from the
    /// current emitted line count.
    ///
    /// Resize is the point where source-backed rendering matters most:
    /// previously emitted prose must stay in scrollback order, while any live
    /// table tail is free to reshape at the new width. This method preserves
    /// that split without attempting byte-for-byte line remapping.
    fn set_width(&mut self, width: Option<usize>) {
        if self.width == width {
            return;
        }
        let had_pending_queue = self.state.queued_len() > 0;
        let had_live_tail = self.has_tail();
        self.width = width;
        self.state.collector.set_width(width);
        let source = self.state.collector.committed_source();
        if source.is_empty() {
            return;
        }

        self.render.recompute(source, self.width, self.cwd.as_path(), self.render_mode);
        self.emitted_stable_len = self.emitted_stable_len.min(self.render.lines.len());
        if had_pending_queue
            && self.emitted_stable_len == self.render.lines.len()
            && self.emitted_stable_len > 0
        {
            // 若折行余量在新宽度下压缩为更少行，至少保留一行未发出，避免永久跳过待处理内容。
            // If wrapped remainder compresses into fewer lines at the new width,
            // keep at least one line un-emitted so pre-resize pending content is
            // not skipped permanently.
            self.emitted_stable_len -= 1;
        }
        self.state.clear_queue();
        if self.emitted_stable_len > 0 && !had_pending_queue && !had_live_tail {
            // 没有待排队稳定行和可变尾部时，避免缩放后重放已发出内容。
            // Avoid replaying already-emitted content after resize when no
            // stable lines were waiting in the queue and there was no mutable
            // tail to preserve.
            self.enqueued_stable_len = self.render.lines.len();
            return;
        }
        self.rebuild_stable_queue_from_render();
    }

    /// 清除当前流的全部累积状态。
    /// Clear all accumulated state for current stream.
    fn reset(&mut self) {
        self.state.clear();
        self.render.clear();
        self.enqueued_stable_len = 0;
        self.emitted_stable_len = 0;
        self.stable_prefix_len_cache = None;
        self.holdback_scanner.reset();
    }

    fn set_render_mode(&mut self, render_mode: StreamRenderMode) {
        if self.render_mode == render_mode {
            return;
        }

        let had_pending_queue = self.state.queued_len() > 0;
        let had_live_tail = self.has_tail();
        self.render_mode = render_mode;
        let source = self.state.collector.committed_source();
        if source.is_empty() {
            return;
        }

        self.render.recompute(source, self.width, self.cwd.as_path(), self.render_mode);
        self.emitted_stable_len = self.emitted_stable_len.min(self.render.lines.len());
        if had_pending_queue
            && self.emitted_stable_len == self.render.lines.len()
            && self.emitted_stable_len > 0
        {
            self.emitted_stable_len -= 1;
        }
        self.state.clear_queue();
        if self.emitted_stable_len > 0 && !had_pending_queue && !had_live_tail {
            self.enqueued_stable_len = self.render.lines.len();
            return;
        }
        self.rebuild_stable_queue_from_render();
    }

    /// 计算应进入稳定区的渲染行数。
    /// Compute how many rendered lines should be in the stable region.
    fn compute_target_stable_len(&mut self) -> usize {
        let tail_budget = self.active_tail_budget_lines();
        self.render.lines.len().saturating_sub(tail_budget).max(self.emitted_stable_len)
    }

    /// 向目标稳定边界推进 `enqueued_stable_len`，并把新稳定行入队；有新增时返回真。
    /// Advance `enqueued_stable_len` toward the target stable boundary and enqueue any
    /// newly-stable lines. Returns `true` if new lines were enqueued.
    fn sync_stable_queue(&mut self) -> bool {
        let target_stable_len = self.compute_target_stable_len();

        // 结构重写把稳定边界移回已入队但未发出的区域，需要从最新快照重建队列。
        // A structural rewrite moved the stable boundary backward into enqueue-but-unemitted
        // lines. Rebuild queue from the latest snapshot.
        if target_stable_len < self.enqueued_stable_len {
            self.state.clear_queue();
            if self.emitted_stable_len < target_stable_len {
                self.state.enqueue(
                    self.render.lines[self.emitted_stable_len..target_stable_len].to_vec(),
                );
            }
            self.enqueued_stable_len = target_stable_len;
            return self.state.queued_len() > 0;
        }

        if target_stable_len == self.enqueued_stable_len {
            return false;
        }

        self.state.enqueue(self.render.lines[self.enqueued_stable_len..target_stable_len].to_vec());
        self.enqueued_stable_len = target_stable_len;
        true
    }

    /// 根据当前渲染快照重建稳定队列。
    ///
    /// `set_width()` 后旧宽度生成的排队行不再可靠，因此使用该路径重新对齐当前渲染。
    /// Rebuild the stable queue from the current render snapshot.
    ///
    /// This is used after `set_width()`, where any queued lines were computed
    /// against the old width and can no longer be trusted to line up with the
    /// current render.
    fn rebuild_stable_queue_from_render(&mut self) {
        let target_stable_len = self.compute_target_stable_len();
        self.state.clear_queue();
        if self.emitted_stable_len < target_stable_len {
            self.state
                .enqueue(self.render.lines[self.emitted_stable_len..target_stable_len].to_vec());
        }
        self.enqueued_stable_len = target_stable_len;
    }

    /// 应暂留为可变尾部的渲染行数。
    ///
    /// 确认表格或待定表头时，从表头起全部暂留，因为新增行可能改变列宽。
    /// 没有表格时全部内容进入稳定区；这是暂留机制的核心决定点。
    /// How many rendered lines to withhold as mutable tail.
    ///
    /// When a table is detected (`Confirmed` or `PendingHeader`), the entire
    /// table region is held as tail because adding a row can reshape table
    /// column widths. For `PendingHeader`, only content from the speculative
    /// header line onward is kept mutable so earlier prose can continue
    /// streaming. When no table is detected, everything flows directly to
    /// stable. This is the core decision point for the holdback mechanism.
    fn active_tail_budget_lines(&mut self) -> usize {
        if self.render_mode == StreamRenderMode::Raw {
            return 0;
        }
        let scan_start = Instant::now();
        let holdback_state = self.holdback_scanner.state();
        let tail_budget = match holdback_state {
            TableHoldbackState::Confirmed { table_start: start }
            | TableHoldbackState::PendingHeader { header_start: start } => {
                self.tail_budget_from_source_start(start)
            }
            TableHoldbackState::None => 0,
        };
        tracing::trace!(
            state = ?holdback_state,
            tail_budget,
            elapsed_us = scan_start.elapsed().as_micros(),
            "table holdback decision",
        );
        tail_budget
    }

    /// 把原始源码边界转换为渲染尾部行数。
    ///
    /// 暂留扫描器使用字节偏移，队列使用渲染行；该辅助函数是两种坐标系的唯一桥梁。
    /// Convert a raw-source boundary into the number of rendered tail lines.
    ///
    /// The important contract here is that the holdback scanner reasons in
    /// byte offsets while the queue operates in rendered lines. This helper is
    /// the only place where those coordinate systems are bridged.
    fn tail_budget_from_source_start(&mut self, source_start: usize) -> usize {
        if source_start == 0 {
            return self.render.lines.len();
        }
        let source_start = source_start.min(self.state.collector.committed_source().len());
        let stable_prefix_len = self.stable_prefix_len_for_source_start(source_start);
        self.render.lines.len().saturating_sub(stable_prefix_len)
    }

    /// 渲染 `source_start` 之前的稳定前缀并返回行数。
    ///
    /// 密集表格流会在每条行增量到达时调用此路径，因此按起点和宽度缓存结果。
    /// Render the stable prefix before `source_start` and return its line count.
    ///
    /// This value is cached because dense table streams can call into this path
    /// for every committed line while the header/delimiter/body are still
    /// arriving incrementally.
    fn stable_prefix_len_for_source_start(&mut self, source_start: usize) -> usize {
        if let Some(cache) = &self.stable_prefix_len_cache
            && cache.source_start == source_start
            && cache.width == self.width
        {
            tracing::trace!(
                source_start,
                width = ?self.width,
                stable_prefix_len = cache.stable_prefix_len,
                "table holdback stable-prefix cache hit",
            );
            return cache.stable_prefix_len;
        }

        let render_start = Instant::now();
        let source = self.state.collector.committed_source();
        let stable_prefix_render = render_markdown_agent_with_links_and_cwd(
            &source[..source_start.min(source.len())],
            self.width,
            Some(self.cwd.as_path()),
        );
        let stable_prefix_len = stable_prefix_render.len();
        tracing::trace!(
            source_start,
            width = ?self.width,
            stable_prefix_len,
            elapsed_us = render_start.elapsed().as_micros(),
            "table holdback stable-prefix render",
        );
        self.stable_prefix_len_cache =
            Some(StablePrefixLenCache { source_start, width: self.width, stable_prefix_len });
        stable_prefix_len
    }
}

/// 支持表格暂留的助手消息流式控制器。
///
/// 它封装 `StreamCore`，并添加 `AgentMessageCell` 的发出样式。
/// Controller for streaming agent message content with table-aware holdback.
///
/// Wraps [`StreamCore`] and adds `AgentMessageCell` emission styling.
pub struct StreamController {
    core: StreamCore,
    header_emitted: bool,
}

impl StreamController {
    /// 创建控制器，使 Markdown 渲染器相对 `cwd` 缩短本地文件链接。
    ///
    /// `width` 是 Markdown 正文宽度，不一定等于完整终端宽度；缩放后传入陈旧宽度会让实时输出继续按旧视口折行。
    /// Create a controller whose markdown renderer shortens local file links relative to `cwd`.
    ///
    /// `width` is the content width available to markdown rendering, not necessarily the full
    /// terminal width. Passing a stale width after resize will keep queued live output wrapped for
    /// the old viewport until app-level reflow repairs the finalized transcript.
    pub fn new(width: Option<usize>, cwd: &Path, render_mode: StreamRenderMode) -> Self {
        Self { core: StreamCore::new(width, cwd, render_mode), header_emitted: false }
    }

    pub fn push(&mut self, delta: &str) -> bool {
        self.core.push_delta(delta)
    }

    /// 定稿活动流，返回包含剩余行的最终单元（若有）以及供合并使用的原始 Markdown 源码。
    /// Finalize the active stream. Returns the final cell (if any remaining lines) and the raw
    /// markdown source for consolidation.
    pub fn finalize(&mut self) -> (Option<StreamingCell>, Option<String>) {
        let (remaining, source) = self.core.finalize_remaining();
        if source.is_empty() {
            self.core.reset();
            return (None, None);
        }

        let out = self.emit(remaining);
        self.core.reset();
        (out, Some(source))
    }

    pub fn on_commit_tick(&mut self) -> (Option<StreamingCell>, bool) {
        let step = self.core.tick();
        (self.emit(step), self.core.is_idle())
    }

    pub fn on_commit_tick_batch(
        &mut self,
        max_lines: usize,
    ) -> (Option<StreamingCell>, bool) {
        let step = self.core.tick_batch(max_lines);
        (self.emit(step), self.core.is_idle())
    }

    // 轻量 `StreamController` 访问器内联，因为每个渲染帧和动画节拍都会调用这些单行委托。
    // Thin StreamController accessors inlined — one-liner delegates called
    // on every render frame and animation tick.

    #[inline]
    pub fn queued_lines(&self) -> usize {
        self.core.queued_lines()
    }

    pub fn oldest_queued_age(&self, now: Instant) -> Option<Duration> {
        self.core.oldest_queued_age(now)
    }

    #[inline]
    pub fn current_tail_lines(&self) -> Vec<HyperlinkLine> {
        self.core.current_tail_lines()
    }

    #[inline]
    pub fn tail_starts_stream(&self) -> bool {
        !self.header_emitted && self.core.enqueued_stable_len == 0
    }

    #[inline]
    pub fn has_live_tail(&self) -> bool {
        self.core.has_tail()
    }

    pub fn clear_queue(&mut self) {
        self.core.state.clear_queue();
        self.core.enqueued_stable_len = self.core.emitted_stable_len;
    }

    pub fn set_width(&mut self, width: Option<usize>) {
        self.core.set_width(width);
    }

    pub fn set_render_mode(&mut self, render_mode: StreamRenderMode) {
        self.core.set_render_mode(render_mode);
    }

    fn emit(&mut self, lines: Vec<HyperlinkLine>) -> Option<StreamingCell> {
        if lines.is_empty() {
            return None;
        }
        Some(StreamingCell::agent(lines, {
            let header_emitted = self.header_emitted;
            self.header_emitted = true;
            !header_emitted
        }))
    }
}
// 以下为拟议计划专用的流式控制器外层。
// ---------------------------------------------------------------------------
// PlanStreamController — proposed plan streams
// ---------------------------------------------------------------------------

/// 把拟议计划 Markdown 流式写入带样式计划块的控制器。
///
/// 它封装 `StreamCore`，并添加计划专用标题、缩进和背景样式。
/// Controller that streams proposed plan markdown into a styled plan block.
///
/// Wraps [`StreamCore`] and adds plan-specific header, indentation, and
/// background styling.
pub struct PlanStreamController {
    core: StreamCore,
    header_emitted: bool,
    top_padding_emitted: bool,
}

impl PlanStreamController {
    /// 创建计划流控制器，使 Markdown 渲染器相对 `cwd` 缩短本地文件链接。
    ///
    /// 宽度含义与助手消息控制器相同，调用者必须在终端宽度改变时更新它。
    /// Create a plan-stream controller whose markdown renderer shortens local file links relative
    /// to `cwd`.
    ///
    /// The width has the same meaning as in `StreamController`: it is the markdown body width, and
    /// callers must update it when the terminal width changes.
    pub fn new(width: Option<usize>, cwd: &Path, render_mode: StreamRenderMode) -> Self {
        Self {
            core: StreamCore::new(width, cwd, render_mode),
            header_emitted: false,
            top_padding_emitted: false,
        }
    }

    pub fn push(&mut self, delta: &str) -> bool {
        self.core.push_delta(delta)
    }

    /// 定稿活动计划流，返回包含剩余行的最终单元（若有）及供合并使用的原始 Markdown 源码。
    /// Finalize the active stream. Returns the final cell (if any remaining
    /// lines) plus raw markdown source for consolidation.
    pub fn finalize(&mut self) -> (Option<StreamingCell>, Option<String>) {
        let (remaining, source) = self.core.finalize_remaining();
        if source.is_empty() {
            self.core.reset();
            return (None, None);
        }

        let out = self.emit(remaining, /*include_bottom_padding*/ true);
        self.core.reset();
        (out, Some(source))
    }

    pub fn on_commit_tick(&mut self) -> (Option<StreamingCell>, bool) {
        let step = self.core.tick();
        (self.emit(step, /*include_bottom_padding*/ false), self.core.is_idle())
    }

    pub fn on_commit_tick_batch(
        &mut self,
        max_lines: usize,
    ) -> (Option<StreamingCell>, bool) {
        let step = self.core.tick_batch(max_lines);
        (self.emit(step, /*include_bottom_padding*/ false), self.core.is_idle())
    }

    #[inline]
    pub fn queued_lines(&self) -> usize {
        self.core.queued_lines()
    }

    #[inline]
    pub fn has_live_tail(&self) -> bool {
        self.core.has_tail()
    }

    #[inline]
    pub fn current_tail_lines(&self) -> Vec<HyperlinkLine> {
        self.core.current_tail_lines()
    }

    #[inline]
    pub fn tail_starts_stream(&self) -> bool {
        !self.header_emitted && self.core.enqueued_stable_len == 0
    }

    pub fn current_tail_display_lines(&self) -> Vec<HyperlinkLine> {
        let lines = self.current_tail_lines();
        if lines.is_empty() {
            return Vec::new();
        }
        self.render_display_lines(lines, /*include_bottom_padding*/ false)
    }

    pub fn oldest_queued_age(&self, now: Instant) -> Option<Duration> {
        self.core.oldest_queued_age(now)
    }

    pub fn clear_queue(&mut self) {
        self.core.state.clear_queue();
        self.core.enqueued_stable_len = self.core.emitted_stable_len;
    }

    pub fn set_width(&mut self, width: Option<usize>) {
        self.core.set_width(width);
    }

    pub fn set_render_mode(&mut self, render_mode: StreamRenderMode) {
        self.core.set_render_mode(render_mode);
    }

    fn emit(
        &mut self,
        lines: Vec<HyperlinkLine>,
        include_bottom_padding: bool,
    ) -> Option<StreamingCell> {
        if lines.is_empty() && !include_bottom_padding {
            return None;
        }

        let out_lines = self.render_display_lines(lines, include_bottom_padding);
        self.header_emitted = true;
        self.top_padding_emitted = true;

        Some(StreamingCell::plan(out_lines))
    }

    fn render_display_lines(
        &self,
        lines: Vec<HyperlinkLine>,
        include_bottom_padding: bool,
    ) -> Vec<HyperlinkLine> {
        let mut out_lines: Vec<HyperlinkLine> = Vec::with_capacity(/*capacity*/ 4);
        if !self.header_emitted {
            out_lines.push(HyperlinkLine::new(vec!["• ".dim(), "Proposed Plan".bold()].into()));
            out_lines.push(HyperlinkLine::new(Line::from(" ")));
        }

        let mut plan_lines: Vec<HyperlinkLine> = Vec::with_capacity(/*capacity*/ 4);
        if !self.top_padding_emitted {
            plan_lines.push(HyperlinkLine::new(Line::from(" ")));
        }
        plan_lines.extend(lines);
        if include_bottom_padding {
            plan_lines.push(HyperlinkLine::new(Line::from(" ")));
        }

        let plan_style = proposed_plan_style();
        let plan_lines = prefix_hyperlink_lines(plan_lines, "  ".into(), "  ".into())
            .into_iter()
            .map(|line| line.style(plan_style))
            .collect::<Vec<_>>();
        out_lines.extend(plan_lines);
        out_lines
    }
}

#[cfg(test)]
#[path = "controller_tests.rs"]
mod tests;
