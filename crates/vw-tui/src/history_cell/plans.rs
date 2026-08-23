//! 拟议计划与计划更新历史单元。
//!
//! 流式尾部是仅用于预览的已渲染片段，完成后必须合并为持有原始 Markdown 的单元。
//! 最终单元同时保存会话工作目录，使终端缩放后仍能稳定重排本地文件链接。
//! Proposed-plan and plan-update history cells.

use super::markdown_render_cache::MarkdownRenderCache;
use super::*;

/// 拟议计划流可变尾部的短暂活动单元表示。
///
/// 控制器准备完整带样式计划行，使尾部与已提交单元共享标题、内边距和背景，
/// 同时在流式期间保持仅预览性质。
/// Transient active-cell representation of the mutable tail of a proposed-plan stream.
///
/// The controller prepares the full styled plan lines because plan tails need the same header,
/// padding, and background treatment as committed `ProposedPlanStreamCell`s while remaining
/// preview-only during streaming.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StreamingPlanTailCell {
    lines: Vec<HyperlinkLine>,
    is_stream_continuation: bool,
}

impl StreamingPlanTailCell {
    pub(crate) fn new(lines: Vec<HyperlinkLine>, is_stream_continuation: bool) -> Self {
        Self {
            lines,
            is_stream_continuation,
        }
    }
}

impl HistoryCell for StreamingPlanTailCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        visible_lines(self.lines.clone())
    }

    fn display_hyperlink_lines(&self, _width: u16) -> Vec<HyperlinkLine> {
        self.lines.clone()
    }

    fn transcript_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        self.display_hyperlink_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        plain_lines(visible_lines(self.lines.clone()))
    }

    fn is_stream_continuation(&self) -> bool {
        self.is_stream_continuation
    }
}
/// 把计划更新渲染为用户友好的复选框待办列表。
/// Render a user‑friendly plan update styled like a checkbox todo list.
pub(crate) fn new_plan_update(update: UpdatePlanArgs) -> PlanUpdateCell {
    let UpdatePlanArgs { explanation, plan } = update;
    PlanUpdateCell { explanation, plan }
}

/// 创建拟议计划单元，并快照会话工作目录供后续 Markdown 渲染使用。
///
/// 计划正文保存为原始 Markdown，终端缩放时可按当前宽度重新排版。
/// 流式构造器只用于短暂实时单元，计划完成后应合并为该源码驱动单元。
/// Create a proposed-plan cell that snapshots the session cwd for later markdown rendering.
///
/// The plan body is stored as raw markdown so terminal resize reflow can render it again at the
/// current width. Callers should use `new_proposed_plan_stream` only for transient live streaming
/// cells, then consolidate to this source-backed cell when the plan is complete.
pub(crate) fn new_proposed_plan(plan_markdown: String, cwd: &Path) -> ProposedPlanCell {
    ProposedPlanCell {
        plan_markdown,
        cwd: cwd.to_path_buf(),
        rendered_lines: MarkdownRenderCache::default(),
    }
}

/// 根据已渲染行创建短暂的拟议计划流单元。
///
/// 流单元是展示片段而非源码历史；定稿前必须替换为 `ProposedPlanCell`，
/// 后者才能在终端缩放时重新排版。
/// Create a transient proposed-plan stream cell from already rendered lines.
///
/// Stream cells are display fragments, not source-backed history. They should be replaced by
/// `ProposedPlanCell` during consolidation before relying on resize reflow for finalized history.
pub(crate) fn new_proposed_plan_stream(
    lines: Vec<impl Into<HyperlinkLine>>,
    is_stream_continuation: bool,
) -> ProposedPlanStreamCell {
    ProposedPlanStreamCell {
        lines: lines.into_iter().map(Into::into).collect(),
        is_stream_continuation,
    }
}

/// 可针对新宽度重新渲染的已定稿拟议计划历史。
///
/// 它是流式单元的源码驱动对应物，拥有原始 Markdown 和稳定本地链接所需的会话目录。
/// Finalized proposed-plan history that can render itself again for a new width.
///
/// This is the source-backed counterpart to `ProposedPlanStreamCell`. It owns raw markdown and the
/// session cwd needed for stable local-link rendering during later transcript reflow.
#[derive(Debug)]
pub(crate) struct ProposedPlanCell {
    plan_markdown: String,
    /// 用于让本地文件链接展示与实时流式计划渲染保持一致的会话目录。
    /// Session cwd used to keep local file-link display aligned with live streamed plan rendering.
    cwd: PathBuf,
    rendered_lines: MarkdownRenderCache,
}

/// 计划仍在流式传输时发出的短暂拟议计划历史。
///
/// 这些行已按当前流宽度渲染；定稿转录不应在合并后继续保留它们，
/// 因为它们无法在后续终端缩放时从源码重新渲染。
/// Transient proposed-plan history emitted while a plan is still streaming.
///
/// The lines are already rendered for the stream's current width. A finalized transcript should not
/// keep these cells after consolidation, because they cannot re-render their source on a later
/// terminal resize.
#[derive(Debug)]
pub(crate) struct ProposedPlanStreamCell {
    lines: Vec<HyperlinkLine>,
    is_stream_continuation: bool,
}

impl HistoryCell for ProposedPlanCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        visible_lines(self.display_hyperlink_lines(width))
    }

    fn display_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        self.rendered_lines.render(width, || {
            let mut lines = vec![
                HyperlinkLine::new(vec!["• ".dim(), "Proposed Plan".bold()].into()),
                HyperlinkLine::new(Line::from(" ")),
            ];

            let mut plan_lines = vec![HyperlinkLine::new(Line::from(" "))];
            let plan_style = proposed_plan_style();
            let wrap_width = width.saturating_sub(4).max(1) as usize;
            let mut body = crate::markdown::render_markdown_agent_with_links_and_cwd(
                &self.plan_markdown,
                Some(wrap_width),
                Some(self.cwd.as_path()),
            );
            if body.is_empty() {
                body.push(HyperlinkLine::new(Line::from("(empty)".dim().italic())));
            }
            plan_lines.extend(prefix_hyperlink_lines(body, "  ".into(), "  ".into()));
            plan_lines.push(HyperlinkLine::new(Line::from(" ")));

            lines.extend(plan_lines.into_iter().map(|line| line.style(plan_style)));
            lines
        })
    }

    fn transcript_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        self.display_hyperlink_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        raw_lines_from_source(&self.plan_markdown)
    }
}

#[cfg(test)]
#[path = "plans_tests.rs"]
mod tests;

impl HistoryCell for ProposedPlanStreamCell {
    fn display_lines(&self, _width: u16) -> Vec<Line<'static>> {
        visible_lines(self.lines.clone())
    }

    fn display_hyperlink_lines(&self, _width: u16) -> Vec<HyperlinkLine> {
        self.lines.clone()
    }

    fn transcript_hyperlink_lines(&self, width: u16) -> Vec<HyperlinkLine> {
        self.display_hyperlink_lines(width)
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        plain_lines(visible_lines(self.lines.clone()))
    }

    fn is_stream_continuation(&self) -> bool {
        self.is_stream_continuation
    }
}

#[derive(Debug)]
pub(crate) struct PlanUpdateCell {
    explanation: Option<String>,
    plan: Vec<PlanItemArg>,
}

impl HistoryCell for PlanUpdateCell {
    fn display_lines(&self, width: u16) -> Vec<Line<'static>> {
        let render_note = |text: &str| -> Vec<Line<'static>> {
            let wrap_width = width.saturating_sub(4).max(1) as usize;
            let note = Line::from(text.to_string().dim().italic());
            let wrapped = adaptive_wrap_line(&note, RtOptions::new(wrap_width));
            let mut out = Vec::new();
            push_owned_lines(&wrapped, &mut out);
            out
        };

        let render_step = |status: &StepStatus, text: &str| -> Vec<Line<'static>> {
            let (box_str, step_style) = match status {
                StepStatus::Completed => ("✔ ", Style::default().crossed_out().dim()),
                StepStatus::InProgress => ("□ ", Style::default().cyan().bold()),
                StepStatus::Pending => ("□ ", Style::default().dim()),
            };

            let opts = RtOptions::new(width.saturating_sub(4).max(1) as usize)
                .initial_indent(box_str.into())
                .subsequent_indent("  ".into());
            let step = Line::from(text.to_string().set_style(step_style));
            let wrapped = adaptive_wrap_line(&step, opts);
            let mut out = Vec::new();
            push_owned_lines(&wrapped, &mut out);
            out
        };

        let mut lines: Vec<Line<'static>> = vec![];
        lines.push(vec!["• ".dim(), "Updated Plan".bold()].into());

        let mut indented_lines = vec![];
        let note = self
            .explanation
            .as_ref()
            .map(|s| s.trim())
            .filter(|t| !t.is_empty());
        if let Some(expl) = note {
            indented_lines.extend(render_note(expl));
        };

        if self.plan.is_empty() {
            indented_lines.push(Line::from("(no steps provided)".dim().italic()));
        } else {
            for PlanItemArg { step, status } in self.plan.iter() {
                indented_lines.extend(render_step(status, step));
            }
        }
        lines.extend(prefix_lines(indented_lines, "  └ ".dim(), "    ".into()));

        lines
    }

    fn raw_lines(&self) -> Vec<Line<'static>> {
        let mut lines = vec![Line::from("Updated Plan")];
        if let Some(explanation) = self
            .explanation
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            lines.extend(raw_lines_from_source(explanation));
        }
        if self.plan.is_empty() {
            lines.push(Line::from("(no steps provided)"));
        } else {
            for PlanItemArg { step, status } in &self.plan {
                lines.push(Line::from(format!("{status:?}: {step}")));
            }
        }
        lines
    }
}
