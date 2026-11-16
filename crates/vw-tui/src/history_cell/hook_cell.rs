//! Hook 执行的历史单元。
//!
//! Hook 有意比普通工具调用更安静：无输出的快速成功不留下转录痕迹，也不闪烁。
//! 每次运行通过隐藏、可见运行、短暂驻留和持久完成四类状态控制展示生命周期。
//! 相邻运行只在生成展示行时合并，底层协议标识和完成转换始终保持独立。
//! History cell for hook execution.
//!
//! Hooks are intentionally quieter than normal tool calls. A hook that starts and finishes
//! successfully without output should not leave a transcript artifact, and very fast hooks should
//! not flash in the viewport. This cell keeps that policy local by treating each hook run as a
//! small rendering state machine:
//!
//! 1. New runs begin hidden in `PendingReveal`.
//! 2. Runs that outlive the reveal delay become visible and may be coalesced with adjacent runs.
//! 3. Visible quiet successes linger briefly so they do not disappear in the same frame they were
//!    first drawn.
//! 4. Completed runs only persist when they have output or a non-success status.
use super::HistoryCell;
use super::plain_lines;
use crate::line_truncation::truncate_line_with_ellipsis_if_overflow;
use crate::motion::MotionMode;
use crate::motion::ReducedMotionIndicator;
use crate::motion::activity_indicator;
use crate::motion::shimmer_text;
use crate::render::line_utils::push_owned_lines;
use crate::render::renderable::Renderable;
use crate::ui_consts::TRANSCRIPT_HINT;
use crate::wrapping::RtOptions;
use crate::wrapping::word_wrap_line;
use codex_app_server_protocol::HookEventName;
use codex_app_server_protocol::HookOutputEntry;
use codex_app_server_protocol::HookOutputEntryKind;
use codex_app_server_protocol::HookRunStatus;
use codex_app_server_protocol::HookRunSummary;
use ratatui::prelude::*;
use ratatui::style::Stylize;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use std::time::Duration;
use std::time::Instant;

#[derive(Debug)]
pub(crate) struct HookCell {
    /// 当前活动、短暂驻留或带持久输出的 Hook 运行。
    /// Hook runs that are active, lingering, or have persistent output to render.
    runs: Vec<HookRunCell>,
    /// 镜像全局动画设置，确保转录与主视口的渲染一致。
    /// Mirrors the global animation setting so transcript rendering and viewport rendering agree.
    animations_enabled: bool,
}

/// Hook 获准绘制前必须持续的最短运行时间。
///
/// 该延迟避免几乎瞬时完成的工作闪出一行文本。
/// Minimum runtime before a hook is allowed to draw.
///
/// Helps avoids a flash of text forwork that was effectively instant.
const HOOK_RUN_REVEAL_DELAY: Duration = Duration::from_millis(300);

/// 安静成功在变为可见后保留在屏幕上的最短时间。
///
/// 它与揭示延迟配合：用户一旦看到 Hook 行，就应有足够时间阅读，而非成功事件到达即消失。
/// Minimum time a quiet success remains on screen after becoming visible.
///
/// This pairs with `HOOK_RUN_REVEAL_DELAY`: once the user has seen a hook row, keep it stable long
/// enough to read instead of removing it immediately when the success event arrives.
const QUIET_HOOK_MIN_VISIBLE: Duration = Duration::from_millis(600);

const HOOK_OUTPUT_INDENT: &str = "  ";
const HOOK_OUTPUT_BODY_INDENT: &str = "    ";
const HOOK_CONTEXT_MAX_DISPLAY_ROWS: usize = 3;

#[derive(Debug)]
struct HookRunCell {
    /// 用于匹配同一 Hook 调用开始与结束更新的稳定协议标识。
    /// Stable protocol id used to match begin/end updates for the same hook invocation.
    id: String,
    /// Hook 事件类型；独立于 `state`，以便开始更新能原地刷新元数据。
    /// Hook event kind, kept outside `state` so a begin update can refresh metadata in place.
    event_name: HookEventName,
    /// Hook 提供的可选详情，显示在运行标题旁。
    /// Optional hook-supplied detail shown next to the running header.
    status_message: Option<String>,
    /// 本次运行的渲染生命周期。
    /// Rendering lifecycle for this run.
    state: HookRunState,
}

#[derive(Debug)]
enum HookRunState {
    /// 刚开始且仍活动，但在 `reveal_deadline` 前有意隐藏的运行。
    /// A newly-started run that is active but deliberately hidden until `reveal_deadline`.
    PendingReveal {
        /// 原始开始时间，在揭示后用于旋转指示相位与分组。
        /// The original start time, used for spinner phase and grouping once revealed.
        start_time: Instant,
        /// 运行最早可见的时刻。
        /// First instant at which the run may become visible.
        reveal_deadline: Instant,
    },
    /// 已超过揭示延迟、当前显示为运行中的任务。
    /// A run that survived the reveal delay and is currently shown as running.
    VisibleRunning {
        /// 原始开始时间，用于在状态转换间保持动画计时稳定。
        /// The original start time, used to keep animation timing stable across transitions.
        start_time: Instant,
        /// 首次实际绘制的时刻，供安静成功的驻留逻辑使用。
        /// First instant the run was actually rendered, used by quiet-success linger.
        visible_since: Instant,
    },
    /// 已可见且无输出成功完成，但仍在短暂驻留的运行。
    /// A visible run that completed successfully without output but is still lingering briefly.
    QuietLinger {
        /// 保留原始开始时间，防止驻留帧中的指示器相位跳变。
        /// The original start time, retained so the spinner does not jump during the linger frame.
        start_time: Instant,
        /// 安静成功可被彻底移除的时刻。
        /// Instant after which the quiet success can be removed entirely.
        removal_deadline: Instant,
    },
    /// 带输出或值得保留状态的已完成运行。
    /// A completed run with output or a status worth preserving in history.
    Completed {
        /// Hook 调用的最终协议状态。
        /// Final protocol status for the hook invocation.
        status: HookRunStatus,
        /// 在完成标题下方渲染的 Hook 输出条目。
        /// Hook output entries rendered below the completed header.
        entries: Vec<HookOutputEntry>,
    },
}

#[derive(Debug, PartialEq, Eq)]
struct RunningHookGroupKey {
    event_name: HookEventName,
    status_message: Option<String>,
}

/// 可共享一条状态行的相邻运行中 Hook 的累加器。
///
/// 分组仅在构建展示行时发生；底层运行保持分离，其协议标识和完成转换互不影响。
/// Accumulator for adjacent running hooks that can share one status line.
///
/// Grouping happens only while building display lines, the underlying runs stay separate so their
/// protocol ids and completion transitions remain independent.
struct RunningHookGroup {
    /// 展示组中所有运行共享的事件与状态对。
    /// Shared event/status pair for every run in this display group.
    key: RunningHookGroupKey,
    /// 组内最早开始时间，使合并指示器反映最早启动的工作。
    /// Earliest start time in the group, so the combined spinner reflects the oldest work.
    start_time: Option<Instant>,
    /// 该分组行代表的相邻运行数量。
    /// Number of adjacent runs represented by the group line.
    count: usize,
}

impl HookCell {
    /// 为刚开始的 Hook 创建活动单元。
    /// Creates a cell around a hook that has just started.
    fn new_active(run: HookRunSummary, animations_enabled: bool) -> Self {
        let mut cell = Self {
            runs: Vec::new(),
            animations_enabled,
        };
        cell.start_run(run);
        cell
    }

    /// 根据转录或历史数据为已完成 Hook 创建单元。
    /// Creates a cell around an already-completed hook from transcript/history data.
    fn new_completed(run: HookRunSummary, animations_enabled: bool) -> Self {
        let mut cell = Self {
            runs: Vec::new(),
            animations_enabled,
        };
        cell.add_completed_run(run);
        cell
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.runs.is_empty()
    }

    /// 任一运行仍可能因结束事件或计时器改变时返回真。
    /// Returns true while any run can still change due to an end event or timer.
    pub(crate) fn is_active(&self) -> bool {
        self.runs.iter().any(|run| run.state.is_active())
    }

    /// 已完成 Hook 不再有计时器后会从活动槽刷新出去。
    /// Completed hook cells are flushed out of the active slot once no timers remain.
    pub(crate) fn should_flush(&self) -> bool {
        !self.is_active() && !self.is_empty()
    }

    /// 返回该单元当前是否至少有一行值得绘制。
    /// Returns whether this cell has at least one line worth drawing right now.
    pub(crate) fn should_render(&self) -> bool {
        self.runs.iter().any(|run| run.state.should_render())
    }

    /// 将持久的已完成运行与短暂的活动单元记账状态分离。
    ///
    /// 安静成功留在活动侧以便消失；失败、阻塞、停止或带输出的 Hook 转为持久历史。
    /// Splits durable completed runs from ephemeral active-cell bookkeeping.
    ///
    /// Quiet successes are left behind so they can disappear from the active cell, while failures,
    /// blocked/stopped hooks, and hooks with emitted output become a persistent history cell.
    pub(crate) fn take_completed_persistent_runs(&mut self) -> Option<Self> {
        let mut completed = Vec::new();
        let mut remaining = Vec::new();
        for run in self.runs.drain(..) {
            if run.state.has_persistent_output() {
                completed.push(run);
            } else {
                remaining.push(run);
            }
        }
        self.runs = remaining;
        (!completed.is_empty()).then_some(Self {
            runs: completed,
            animations_enabled: self.animations_enabled,
        })
    }

    /// 供调用者判断活动单元当前是否占据主视口空间。
    /// Used by callers that need to know whether the active cell currently occupies viewport space.
    pub(crate) fn has_visible_running_run(&self) -> bool {
        self.runs.iter().any(|run| run.state.is_running_visible())
    }

    /// 推进揭示与移除计时器，并报告是否需要刷新渲染。
    /// Advances reveal/removal timers and reports whether rendering should be refreshed.
    pub(crate) fn advance_time(&mut self, now: Instant) -> bool {
        let old_len = self.runs.len();
        let mut changed = false;
        for run in &mut self.runs {
            changed |= run.state.reveal_if_due(now);
        }
        self.runs.retain(|run| !run.state.quiet_linger_expired(now));
        changed || self.runs.len() != old_len
    }

    /// 插入或刷新一个已开始的 Hook 运行。
    ///
    /// 重复开始事件会重置揭示计时器而非新增一行；按标识匹配是维持开始/结束配对的不变量。
    /// Inserts or refreshes a started hook run.
    ///
    /// A duplicate begin event resets the reveal timer rather than adding a second row, because
    /// matching by id is the invariant that keeps begin/end events paired.
    pub(crate) fn start_run(&mut self, run: HookRunSummary) {
        let now = Instant::now();
        if let Some(existing) = self.runs.iter_mut().find(|existing| existing.id == run.id) {
            existing.event_name = run.event_name;
            existing.status_message = run.status_message;
            existing.state = HookRunState::pending(now);
            return;
        }
        self.runs.push(HookRunCell {
            id: run.id,
            event_name: run.event_name,
            status_message: run.status_message,
            state: HookRunState::pending(now),
        });
    }

    /// 完成一次运行，并返回它此前是否已存在于本单元。
    ///
    /// 安静成功不产生持久输出：从未可见则立即消失，已经绘制则进入 `QuietLinger`。
    /// Completes a run and returns whether the run was already present in this cell.
    ///
    /// Quiet successes intentionally avoid persistent output. If they were never visible, they
    /// disappear immediately; if they had already drawn, they move into `QuietLinger`.
    pub(crate) fn complete_run(&mut self, run: HookRunSummary) -> bool {
        let Some(index) = self.runs.iter().position(|existing| existing.id == run.id) else {
            return false;
        };
        if hook_run_is_quiet_success(&run) {
            if !self.runs[index]
                .state
                .complete_quiet_success(Instant::now())
            {
                self.runs.remove(index);
            }
            return true;
        }
        let HookRunSummary {
            event_name,
            status_message,
            status,
            entries,
            ..
        } = run;
        let existing = &mut self.runs[index];
        existing.event_name = event_name;
        existing.status_message = status_message;
        existing.state = HookRunState::completed(status, entries);
        true
    }

    /// 添加未经过当前实时单元的已完成 Hook。
    ///
    /// 用于回放或恢复路径，此时最终运行摘要已经完整可用。
    /// Adds a completed hook that did not pass through this live cell.
    ///
    /// This is used for replay/restoration paths where the final run summary is already known.
    pub(crate) fn add_completed_run(&mut self, run: HookRunSummary) {
        if hook_run_is_quiet_success(&run) {
            return;
        }
        let HookRunSummary {
            id,
            event_name,
            status_message,
            status,
            entries,
            ..
        } = run;
        self.runs.push(HookRunCell {
            id,
            event_name,
            status_message,
            state: HookRunState::completed(status, entries),
        });
    }

    pub(crate) fn next_timer_deadline(&self) -> Option<Instant> {
        self.runs
            .iter()
            .filter_map(|run| run.state.next_timer_deadline())
            .min()
    }

    #[cfg(test)]
    pub(crate) fn expire_quiet_runs_now_for_test(&mut self) {
        for run in &mut self.runs {
            run.expire_quiet_linger_now_for_test();
        }
    }

    #[cfg(test)]
    pub(crate) fn reveal_running_runs_now_for_test(&mut self) {
        let now = Instant::now();
        for run in &mut self.runs {
            run.reveal_running_now_for_test(now);
        }
    }

    #[cfg(test)]
    pub(crate) fn reveal_running_runs_after_delayed_redraw_for_test(&mut self) {
        let now = Instant::now();
        for run in &mut self.runs {
            run.reveal_running_after_delayed_redraw_for_test(now);
        }
    }

    /// 为受限主视口或完整转录浮层构建 Hook 行。
    /// Builds hook lines for either the bounded main viewport or the full transcript overlay.
    fn output_lines(&self, width: u16, render_full_context: bool) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let mut running_group: Option<RunningHookGroup> = None;
        for run in &self.runs {
            if !run.state.should_render() {
                continue;
            }

            let Some(key) = run.running_group_key() else {
                // 已完成运行保留自己的输出行，因此绘制它之前必须先发出待处理的运行组。
                // Completed runs keep their own output lines, so any pending running group must be
                // emitted before drawing the completed run.
                if let Some(group) = running_group.take() {
                    push_running_hook_group(&mut lines, &group, self.animations_enabled);
                }
                push_hook_line_separator(&mut lines);
                run.push_display_lines(
                    &mut lines,
                    self.animations_enabled,
                    width,
                    render_full_context,
                );
                continue;
            };

            if let Some(group) = running_group.as_mut()
                && group.key == key
            {
                group.count += 1;
                // 保留最早开始时间，避免较晚 Hook 并入同组时重置合并指示器。
                // Preserve the earliest start time so grouped spinners do not reset when a later
                // adjacent hook is folded into the same line.
                group.start_time = earliest_instant(group.start_time, run.state.start_time());
                continue;
            }

            if let Some(group) =
                running_group.replace(RunningHookGroup::new(key, run.state.start_time()))
            {
                push_running_hook_group(&mut lines, &group, self.animations_enabled);
            }
        }
        if let Some(group) = running_group {
            push_running_hook_group(&mut lines, &group, self.animations_enabled);
        }
        lines
    }
}

impl HistoryCell for HookCell {
    /// 构建主视口行，并合并相邻且可见的运行中 Hook。
    /// Builds viewport lines while coalescing adjacent visible-running hooks.
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.output_lines(width, /*render_full_context*/ false)
    }

    /// 转录浮层保留主视口预览隐藏的完整 Hook 上下文。
    /// The transcript overlay preserves complete hook context hidden by the viewport preview.
    fn transcript_lines(&self, width: u16) -> Vec<Line<'static>> {
        self.output_lines(width, /*render_full_context*/ true)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        plain_lines(self.output_lines(u16::MAX, /*render_full_context*/ true))
    }

    /// Hook 动画活动期间，为转录浮层生成粗粒度缓存键。
    /// Produces a coarse cache key for transcript overlays while hook animations are active.
    fn transcript_animation_tick(&self) -> Option<u64> {
        if !self.animations_enabled {
            return None;
        }
        let elapsed = self
            .runs
            .iter()
            .filter(|run| run.state.is_running_visible())
            .find_map(|run| run.state.start_time())?
            .elapsed();
        Some(elapsed.as_millis() as u64 / 600)
    }
}

impl Renderable for HookCell {
    fn render(&self, area: Rect, buf: &mut Buffer) {
        let lines = self.display_lines(area.width);
        let paragraph = Paragraph::new(Text::from(lines)).wrap(Wrap { trim: false });
        paragraph.render(area, buf);
    }

    fn desired_height(&self, width: u16) -> u16 {
        HistoryCell::desired_height(self, width)
    }
}

impl HookRunCell {
    #[cfg(test)]
    fn expire_quiet_linger_now_for_test(&mut self) {
        if let HookRunState::QuietLinger {
            removal_deadline, ..
        } = &mut self.state
        {
            *removal_deadline = Instant::now();
        }
    }

    #[cfg(test)]
    fn reveal_running_now_for_test(&mut self, now: Instant) {
        if let HookRunState::PendingReveal {
            reveal_deadline, ..
        } = &mut self.state
        {
            *reveal_deadline = now;
        }
    }

    #[cfg(test)]
    fn reveal_running_after_delayed_redraw_for_test(&mut self, now: Instant) {
        if let HookRunState::PendingReveal {
            reveal_deadline, ..
        } = &mut self.state
        {
            let delayed_deadline = now
                .checked_sub(QUIET_HOOK_MIN_VISIBLE + Duration::from_millis(100))
                .unwrap_or(now);
            *reveal_deadline = delayed_deadline;
        }
    }

    /// 仅对渲染为运行中的状态返回分组键。
    /// Returns the grouping key only for states that render as running.
    fn running_group_key(&self) -> Option<RunningHookGroupKey> {
        self.state
            .is_running_visible()
            .then(|| RunningHookGroupKey {
                event_name: self.event_name,
                status_message: self.status_message.clone(),
            })
    }

    /// 追加单个未分组 Hook 运行的展示行。
    /// Appends the lines for a single, ungrouped hook run.
    fn push_display_lines(
        &self,
        lines: &mut Vec<Line<'static>>,
        animations_enabled: bool,
        width: u16,
        render_full_context: bool,
    ) {
        let label = hook_event_label(self.event_name);
        match &self.state {
            HookRunState::VisibleRunning { start_time, .. }
            | HookRunState::QuietLinger { start_time, .. } => {
                let hook_text = format!("Running {label} hook");
                push_running_hook_header(
                    lines,
                    &hook_text,
                    Some(*start_time),
                    self.status_message.as_deref(),
                    animations_enabled,
                );
            }
            HookRunState::Completed { status, entries } => {
                let system_message = entries
                    .iter()
                    .find(|entry| entry.kind == HookOutputEntryKind::Warning);
                let mut system_message_lines = system_message.map(|entry| entry.text.split('\n'));
                let status_text = format!("{status:?}").to_lowercase();
                let header_text = if let Some(first_line) =
                    system_message_lines.as_mut().and_then(Iterator::next)
                {
                    format!("{label} ({status_text}) says: {first_line}")
                } else {
                    format!("{label} hook ({status_text})")
                };
                lines.push(
                    vec![
                        hook_completed_bullet(*status, entries),
                        " ".into(),
                        header_text.into(),
                    ]
                    .into(),
                );
                if let Some(system_message_lines) = system_message_lines {
                    for line in system_message_lines {
                        if line.is_empty() {
                            lines.push("".into());
                        } else {
                            lines.push(format!("{HOOK_OUTPUT_BODY_INDENT}{line}").into());
                        }
                    }
                }
                for entry in entries {
                    if entry.kind == HookOutputEntryKind::Warning {
                        continue;
                    }
                    if !render_full_context && entry.kind == HookOutputEntryKind::Context {
                        lines.extend(hook_context_preview_lines(&entry.text, width));
                    } else {
                        push_full_hook_output_entry(lines, entry);
                    }
                }
            }
            HookRunState::PendingReveal { .. } => {}
        }
    }
}

fn push_full_hook_output_entry(lines: &mut Vec<Line<'static>>, entry: &HookOutputEntry) {
    let prefix = hook_output_prefix(entry.kind);
    let mut output_lines = entry.text.split('\n');
    if let Some(first_line) = output_lines.next() {
        lines.push(format!("{HOOK_OUTPUT_INDENT}{prefix}{first_line}").into());
    }
    for line in output_lines {
        if line.is_empty() {
            lines.push("".into());
        } else {
            lines.push(format!("{HOOK_OUTPUT_BODY_INDENT}{line}").into());
        }
    }
}

fn hook_context_preview_lines(text: &str, width: u16) -> Vec<Line<'static>> {
    let width = usize::from(width.max(1));
    let mut wrapped = Vec::new();
    let mut source_lines = text.split('\n');
    let first_line = source_lines.next().unwrap_or_default();
    push_wrapped_hook_context_line(
        &mut wrapped,
        first_line,
        width,
        Line::from(format!(
            "{HOOK_OUTPUT_INDENT}{}",
            hook_output_prefix(HookOutputEntryKind::Context)
        )),
    );
    for line in source_lines {
        if line.is_empty() {
            wrapped.push("".into());
        } else {
            push_wrapped_hook_context_line(
                &mut wrapped,
                line,
                width,
                Line::from(HOOK_OUTPUT_BODY_INDENT),
            );
        }
    }

    if wrapped.len() <= HOOK_CONTEXT_MAX_DISPLAY_ROWS {
        return wrapped;
    }

    let retained_rows = HOOK_CONTEXT_MAX_DISPLAY_ROWS - 1;
    let omitted_rows = wrapped.len() - retained_rows;
    wrapped.truncate(retained_rows);
    let hint = vec![
        HOOK_OUTPUT_BODY_INDENT.into(),
        format!("… +{omitted_rows} lines ({TRANSCRIPT_HINT})").dim(),
    ]
    .into();
    wrapped.push(truncate_line_with_ellipsis_if_overflow(hint, width));
    wrapped
}

fn push_wrapped_hook_context_line(
    output: &mut Vec<Line<'static>>,
    text: &str,
    width: usize,
    initial_indent: Line<'static>,
) {
    let line = Line::from(text.to_string());
    let wrapped = word_wrap_line(
        &line,
        RtOptions::new(width)
            .initial_indent(initial_indent)
            .subsequent_indent(Line::from(HOOK_OUTPUT_BODY_INDENT)),
    );
    push_owned_lines(&wrapped, output);
}

impl HookRunState {
    /// 为实时 Hook 运行创建隐藏的初始状态。
    /// Creates the hidden initial state for a live hook run.
    fn pending(start_time: Instant) -> Self {
        Self::PendingReveal {
            start_time,
            reveal_deadline: start_time + HOOK_RUN_REVEAL_DELAY,
        }
    }

    /// 为带可见输出或显著状态的 Hook 创建持久最终状态。
    /// Creates the persistent final state for a hook with visible output or a notable status.
    fn completed(status: HookRunStatus, entries: Vec<HookOutputEntry>) -> Self {
        Self::Completed { status, entries }
    }

    /// 运行仍等待完成事件或计时器清理时返回真。
    /// Returns true while the run is still waiting for a completion event or timer cleanup.
    fn is_active(&self) -> bool {
        match self {
            HookRunState::PendingReveal { .. }
            | HookRunState::VisibleRunning { .. }
            | HookRunState::QuietLinger { .. } => true,
            HookRunState::Completed { .. } => false,
        }
    }

    /// 本次运行至少为当前渲染贡献一行时返回真。
    /// Returns true when this run contributes at least one line to the current render.
    fn should_render(&self) -> bool {
        match self {
            HookRunState::VisibleRunning { .. }
            | HookRunState::QuietLinger { .. }
            | HookRunState::Completed { .. } => true,
            HookRunState::PendingReveal { .. } => false,
        }
    }

    /// 对应在活动单元之外继续保留的已完成运行返回真。
    /// Returns true for completed runs that should survive outside the active cell.
    fn has_persistent_output(&self) -> bool {
        match self {
            HookRunState::Completed { status, entries } => {
                *status != HookRunStatus::Completed || !entries.is_empty()
            }
            HookRunState::PendingReveal { .. }
            | HookRunState::VisibleRunning { .. }
            | HookRunState::QuietLinger { .. } => false,
        }
    }

    /// 返回活动状态的原始开始时间。
    ///
    /// 已完成运行不再动画，因此有意不再保存开始时间。
    /// Returns the original start time for active states.
    ///
    /// Completed runs no longer animate, so they intentionally have no start time.
    fn start_time(&self) -> Option<Instant> {
        match self {
            HookRunState::PendingReveal { start_time, .. }
            | HookRunState::VisibleRunning { start_time, .. }
            | HookRunState::QuietLinger { start_time, .. } => Some(*start_time),
            HookRunState::Completed { .. } => None,
        }
    }

    /// 运行应按进行中行处理时返回真。
    /// Returns true when the run should be treated as an in-progress row.
    fn is_running_visible(&self) -> bool {
        matches!(
            self,
            HookRunState::VisibleRunning { .. } | HookRunState::QuietLinger { .. }
        )
    }

    /// 待处理运行超过截止时间后将其揭示。
    ///
    /// 仅在状态实际改变时返回真，使计时回调避免不必要的重绘。
    /// Reveals a pending run once its deadline has passed.
    ///
    /// Returns true only when this call changes the state, allowing timer callbacks to avoid
    /// unnecessary redraws.
    fn reveal_if_due(&mut self, now: Instant) -> bool {
        let HookRunState::PendingReveal {
            start_time,
            reveal_deadline,
        } = self
        else {
            return false;
        };
        if now < *reveal_deadline {
            return false;
        }
        *self = HookRunState::VisibleRunning {
            start_time: *start_time,
            visible_since: now,
        };
        true
    }

    /// 返回本次运行状态机拥有的下一个截止时间。
    /// Returns the next state-machine deadline owned by this run.
    fn next_timer_deadline(&self) -> Option<Instant> {
        match self {
            HookRunState::PendingReveal {
                reveal_deadline, ..
            } => Some(*reveal_deadline),
            HookRunState::QuietLinger {
                removal_deadline, ..
            } => Some(*removal_deadline),
            HookRunState::VisibleRunning { .. } | HookRunState::Completed { .. } => None,
        }
    }

    /// 安静成功已经驻留足够长时间后返回真。
    /// Returns true once a quiet success has lingered for long enough.
    fn quiet_linger_expired(&self, now: Instant) -> bool {
        match self {
            HookRunState::QuietLinger {
                removal_deadline, ..
            } => now >= *removal_deadline,
            HookRunState::PendingReveal { .. }
            | HookRunState::VisibleRunning { .. }
            | HookRunState::Completed { .. } => false,
        }
    }

    /// 把可见的安静成功转换为临时驻留状态。
    ///
    /// 从未可见或已经满足最短可见时间时无需驻留，并返回假以便立即移除。
    /// Converts a visible quiet success into a temporary linger state.
    ///
    /// Returns false when the success should be removed immediately: either it was never visible or
    /// it has already stayed visible for the minimum duration.
    fn complete_quiet_success(&mut self, now: Instant) -> bool {
        let HookRunState::VisibleRunning {
            start_time,
            visible_since,
            ..
        } = self
        else {
            return false;
        };
        let start_time = *start_time;
        let minimum_deadline = *visible_since + QUIET_HOOK_MIN_VISIBLE;
        if now >= minimum_deadline {
            return false;
        }
        *self = HookRunState::QuietLinger {
            start_time,
            removal_deadline: minimum_deadline,
        };
        true
    }
}

impl RunningHookGroup {
    fn new(key: RunningHookGroupKey, start_time: Option<Instant>) -> Self {
        Self {
            key,
            start_time,
            count: 1,
        }
    }
}

/// 发出一条分组的运行中 Hook 状态行。
/// Emits one grouped running-hook status row.
fn push_running_hook_group(
    lines: &mut Vec<Line<'static>>,
    group: &RunningHookGroup,
    animations_enabled: bool,
) {
    push_hook_line_separator(lines);
    let label = hook_event_label(group.key.event_name);
    let hook_text = if group.count == 1 {
        format!("Running {label} hook")
    } else {
        format!("Running {} {label} hooks", group.count)
    };
    push_running_hook_header(
        lines,
        &hook_text,
        group.start_time,
        group.key.status_message.as_deref(),
        animations_enabled,
    );
}

/// 发出所有运行中 Hook 行共用的动画或静态标题。
/// Emits the animated or static header used by all running hook rows.
fn push_running_hook_header(
    lines: &mut Vec<Line<'static>>,
    hook_text: &str,
    start_time: Option<Instant>,
    status_message: Option<&str>,
    animations_enabled: bool,
) {
    let mut header = Vec::new();
    let motion_mode = MotionMode::from_animations_enabled(animations_enabled);
    if let Some(indicator) =
        activity_indicator(start_time, motion_mode, ReducedMotionIndicator::Hidden)
    {
        header.push(indicator);
        header.push(" ".into());
    }
    header.extend(shimmer_text(hook_text, motion_mode));
    if !animations_enabled && let Some(span) = header.last_mut() {
        span.style = span.style.patch(Style::default().bold());
    }
    if let Some(status_message) = status_message
        && !status_message.is_empty()
    {
        header.push(": ".into());
        header.push(status_message.to_string().dim());
    }
    lines.push(header.into());
}

/// 在 Hook 块之间添加空白分隔，但不留下开头空行。
/// Adds a blank separator between hook blocks without leaving a leading blank line.
fn push_hook_line_separator(lines: &mut Vec<Line<'static>>) {
    if !lines.is_empty() {
        lines.push("".into());
    }
}

/// 合并可选时刻，并保留已知的最早开始时间。
/// Combines optional instants while preserving the earliest known start time.
fn earliest_instant(left: Option<Instant>, right: Option<Instant>) -> Option<Instant> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

pub(crate) fn new_active_hook_cell(run: HookRunSummary, animations_enabled: bool) -> HookCell {
    HookCell::new_active(run, animations_enabled)
}

pub(crate) fn new_completed_hook_cell(run: HookRunSummary, animations_enabled: bool) -> HookCell {
    HookCell::new_completed(run, animations_enabled)
}

/// 对应不应出现在历史中的 Hook 完成结果返回真。
/// Returns true for hook completions that should be invisible in history.
fn hook_run_is_quiet_success(run: &HookRunSummary) -> bool {
    run.status == HookRunStatus::Completed && run.entries.is_empty()
}

fn hook_completed_bullet(status: HookRunStatus, entries: &[HookOutputEntry]) -> Span<'static> {
    match status {
        HookRunStatus::Completed => {
            if entries
                .iter()
                .any(|entry| entry.kind == HookOutputEntryKind::Warning)
            {
                "•".bold()
            } else {
                "•".green().bold()
            }
        }
        HookRunStatus::Blocked | HookRunStatus::Failed | HookRunStatus::Stopped => "•".red().bold(),
        HookRunStatus::Running => "•".into(),
    }
}

fn hook_output_prefix(kind: HookOutputEntryKind) -> &'static str {
    match kind {
        HookOutputEntryKind::Warning => "warning: ",
        HookOutputEntryKind::Stop => "stop: ",
        HookOutputEntryKind::Feedback => "feedback: ",
        HookOutputEntryKind::Context => "hook context: ",
        HookOutputEntryKind::Error => "error: ",
    }
}

fn hook_event_label(event_name: HookEventName) -> &'static str {
    match event_name {
        HookEventName::PreToolUse => "PreToolUse",
        HookEventName::PermissionRequest => "PermissionRequest",
        HookEventName::PostToolUse => "PostToolUse",
        HookEventName::PreCompact => "PreCompact",
        HookEventName::PostCompact => "PostCompact",
        HookEventName::SessionStart => "SessionStart",
        HookEventName::SessionEnd => "SessionEnd",
        HookEventName::UserPromptSubmit => "UserPromptSubmit",
        HookEventName::SubagentStart => "SubagentStart",
        HookEventName::SubagentStop => "SubagentStop",
        HookEventName::Stop => "Stop",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::PathBufExt;
    use crate::test_support::test_path_buf;
    use pretty_assertions::assert_eq;
    use ratatui::style::Modifier;

    #[test]
    // 警告完成项应使用默认粗体项目符号，而不是成功或失败的专用颜色。
    fn completed_hook_with_warning_uses_default_bold_bullet() {
        let entries = vec![HookOutputEntry {
            kind: HookOutputEntryKind::Warning,
            text: "Heads up from the hook".to_string(),
        }];

        let bullet = hook_completed_bullet(HookRunStatus::Completed, &entries);

        assert_eq!(bullet.content.as_ref(), "•");
        assert_eq!(bullet.style.fg, None);
        assert!(bullet.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    // 短多行上下文在视口、转录和原始文本路径中都应完整保留。
    fn completed_hook_short_multiline_context_preserves_display_transcript_and_raw_lines() {
        let cell = completed_hook_cell(
            HookEventName::SessionStart,
            HookRunStatus::Completed,
            vec![HookOutputEntry {
                kind: HookOutputEntryKind::Context,
                text: "## Working Memory Recall\n\nSource: Codex compaction".to_string(),
            }],
        );
        let expected = vec![
            "• SessionStart hook (completed)".to_string(),
            "  hook context: ## Working Memory Recall".to_string(),
            "".to_string(),
            "    Source: Codex compaction".to_string(),
        ];

        assert_eq!(line_texts(&cell.display_lines(/*width*/ 80)), expected);
        assert_eq!(line_texts(&cell.transcript_lines(/*width*/ 80)), expected);
        assert_eq!(line_texts(&cell.raw_lines()), expected);
    }

    #[test]
    // 长单行上下文只在受限视口缩略，完整转录仍保留原文。
    fn completed_hook_long_single_line_context_is_truncated_only_in_display() {
        let full_context = format!(
            "{}tail-marker",
            "context words that should wrap across the terminal width ".repeat(8)
        );
        let cell = completed_hook_cell(
            HookEventName::SessionStart,
            HookRunStatus::Completed,
            vec![HookOutputEntry {
                kind: HookOutputEntryKind::Context,
                text: full_context.clone(),
            }],
        );

        let display_lines = cell.display_lines(/*width*/ 80);
        let display = line_texts(&display_lines);
        assert_eq!(display.len(), 4);
        assert_eq!(
            Paragraph::new(Text::from(display_lines[1..].to_vec()))
                .wrap(Wrap { trim: false })
                .line_count(/*width*/ 80),
            HOOK_CONTEXT_MAX_DISPLAY_ROWS
        );
        assert!(
            display
                .iter()
                .any(|line| line.contains("ctrl + t to view transcript")),
            "expected truncated context to advertise the transcript: {display:?}"
        );
        assert!(display.iter().all(|line| !line.contains("tail-marker")));

        let expected_full = vec![
            "• SessionStart hook (completed)".to_string(),
            format!("  hook context: {full_context}"),
        ];
        assert_eq!(
            line_texts(&cell.transcript_lines(/*width*/ 80)),
            expected_full
        );
        assert_eq!(line_texts(&cell.raw_lines()), expected_full);
    }

    #[test]
    // 非上下文输出不应用上下文预览的截断策略。
    fn completed_hook_non_context_entries_are_not_truncated() {
        for kind in [
            HookOutputEntryKind::Warning,
            HookOutputEntryKind::Stop,
            HookOutputEntryKind::Feedback,
            HookOutputEntryKind::Error,
        ] {
            let cell = completed_hook_cell(
                HookEventName::UserPromptSubmit,
                HookRunStatus::Stopped,
                vec![HookOutputEntry {
                    kind,
                    text: "first\nsecond\nthird\nfourth\nfifth".to_string(),
                }],
            );

            let display = line_texts(&cell.display_lines(/*width*/ 20));
            assert!(
                display.iter().any(|line| line == "    fifth"),
                "expected {kind:?} output to remain complete: {display:?}"
            );
            assert!(
                display
                    .iter()
                    .all(|line| !line.contains("ctrl + t to view transcript")),
                "did not expect a transcript hint for {kind:?}: {display:?}"
            );
        }
    }

    #[test]
    // 停止 Hook 的多行系统消息只给首行添加语义前缀。
    fn completed_stop_hook_multiline_system_message_prefixes_first_line_only() {
        let cell = completed_hook_cell(
            HookEventName::Stop,
            HookRunStatus::Completed,
            vec![HookOutputEntry {
                kind: HookOutputEntryKind::Warning,
                text: "Heads up\nReview generated files".to_string(),
            }],
        );

        assert_eq!(
            line_texts(&cell.display_lines(/*width*/ 80)),
            vec![
                "• Stop (completed) says: Heads up".to_string(),
                "    Review generated files".to_string(),
            ]
        );
    }

    #[test]
    // 尚未揭示的 Hook 在转录路径中保持静态，避免隐藏状态产生动画噪声。
    fn pending_hook_does_not_animate_transcript() {
        let cell =
            HookCell::new_active(hook_run_summary("hook-1"), /*animations_enabled*/ true);

        assert_eq!(cell.transcript_animation_tick(), None);
    }

    #[test]
    // 已揭示 Hook 在启用动画时应随时间更新转录指示器。
    fn visible_hook_animates_transcript_when_animations_enabled() {
        let mut cell =
            HookCell::new_active(hook_run_summary("hook-1"), /*animations_enabled*/ true);
        cell.reveal_running_runs_now_for_test();
        cell.advance_time(Instant::now());

        assert_eq!(cell.transcript_animation_tick(), Some(0));
    }

    #[test]
    // 关闭动画后，同一可见 Hook 的转录输出必须稳定。
    fn visible_hook_does_not_animate_transcript_when_animations_disabled() {
        let mut cell = HookCell::new_active(
            hook_run_summary("hook-1"),
            /*animations_enabled*/ false,
        );
        cell.reveal_running_runs_now_for_test();
        cell.advance_time(Instant::now());

        assert_eq!(cell.transcript_animation_tick(), None);
    }

    #[test]
    // 禁用动画的可见 Hook 不应残留旋转指示器字符。
    fn visible_hook_without_animations_omits_spinner() {
        let mut cell = HookCell::new_active(
            hook_run_summary("hook-1"),
            /*animations_enabled*/ false,
        );
        cell.reveal_running_runs_now_for_test();
        cell.advance_time(Instant::now());

        let rendered: Vec<String> = cell
            .display_lines(/*width*/ 80)
            .iter()
            .map(line_text)
            .collect();

        assert_eq!(
            rendered,
            vec!["Running PostToolUse hook: checking output policy".to_string()]
        );
    }

    fn completed_hook_cell(
        event_name: HookEventName,
        status: HookRunStatus,
        entries: Vec<HookOutputEntry>,
    ) -> HookCell {
        let mut run = hook_run_summary("hook-1");
        run.event_name = event_name;
        run.status = status;
        run.status_message = None;
        run.completed_at = Some(2);
        run.duration_ms = Some(1);
        run.entries = entries;
        HookCell::new_completed(run, /*animations_enabled*/ false)
    }

    fn line_texts(lines: &[Line<'_>]) -> Vec<String> {
        lines.iter().map(line_text).collect()
    }

    fn line_text(line: &Line<'_>) -> String {
        line.spans
            .iter()
            .map(|span| span.content.as_ref())
            .collect::<String>()
    }

    fn hook_run_summary(id: &str) -> HookRunSummary {
        HookRunSummary {
            id: id.to_string(),
            event_name: HookEventName::PostToolUse,
            handler_type: codex_app_server_protocol::HookHandlerType::Command,
            execution_mode: codex_app_server_protocol::HookExecutionMode::Sync,
            scope: codex_app_server_protocol::HookScope::Turn,
            source_path: test_path_buf("/tmp/hooks.json").abs(),
            source: codex_app_server_protocol::HookSource::User,
            display_order: 0,
            status: HookRunStatus::Running,
            status_message: Some("checking output policy".to_string()),
            started_at: 1,
            completed_at: None,
            duration_ms: None,
            entries: Vec::new(),
        }
    }
}
