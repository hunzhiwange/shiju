use super::{
    Color, Line, Modifier, Paragraph, Rect, SPINNER_FRAMES, Span, Style, Text, TuiPanelBorder,
    TuiPanelSurface, TuiPill, TuiRenderer, TuiScrollbarFeedback, TuiScrollbarView, TuiState,
    TuiStatusSummary, TuiTheme, TuiTone, TuiVisibleTranscriptWindow, UiStepState, UiTurnTerminal,
    Wrap, compact_path_label, truncate_label, zh_unseen_pill_label,
};
use crate::cli::tui_v2::model::{UiMessage, UiThinkingPhase, UiToolCallState};

/// status header 可直接消费的聚合结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiStatusHeader {
    pub(crate) badge: String,
    pub(crate) title: String,
    pub(crate) terminal: String,
    pub(crate) terminal_tone: TuiTone,
    pub(crate) scope: String,
    pub(crate) cwd: String,
    pub(crate) spinner: String,
}

/// status footer primitive。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiStatusFooter {
    pub(crate) pills: Vec<TuiPill>,
    pub(crate) detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiActivityIndicator {
    pub(crate) spinner: String,
    pub(crate) verb: String,
}

/// project context 侧栏 primitive。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiProjectContextHost {
    pub(crate) pills: Vec<TuiPill>,
    pub(crate) body_lines: Vec<String>,
}

/// modified files 侧栏 primitive。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiModifiedFilesHost {
    pub(crate) pills: Vec<TuiPill>,
    pub(crate) body_lines: Vec<String>,
}

impl TuiRenderer {
    pub(super) fn render_transcript_mode(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        state: &TuiState,
    ) -> Option<TuiScrollbarFeedback> {
        let mode = &state.transcript_mode;
        let block =
            self.panel_block("Transcript 阅读模式", TuiPanelSurface::Raised, TuiPanelBorder::Focused);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let (content_area, scrollbar_track) =
            super::layout::split_scrollbar_area(inner);

        let mut lines = mode
            .visible_lines()
            .iter()
            .enumerate()
            .map(|(offset, line)| {
                let line_number = mode.top_line.saturating_add(offset).saturating_add(1);
                Line::from(vec![
                    Span::styled(format!("{line_number:>5} "), self.theme.muted_style()),
                    Span::styled(line.clone(), self.theme.body_style()),
                ])
            })
            .collect::<Vec<_>>();
        lines.push(Line::raw(""));
        if mode.search_bar_open {
            lines.push(Line::from(vec![
                Span::styled("/", self.theme.accent_style().add_modifier(Modifier::BOLD)),
                Span::styled(mode.search_draft.clone(), self.theme.body_style()),
                Span::styled("  Enter 确认 · Esc 取消", self.theme.muted_style()),
            ]));
        } else {
            let match_label = mode.selected_match.map_or_else(
                String::new,
                |selected| format!(" · 匹配 {}/{}", selected + 1, mode.search_matches.len()),
            );
            lines.push(Line::styled(
                format!(
                    "j/k 滚动 · g/G 首尾 · / 搜索 · n/N 匹配 · [ 导出 · v 编辑 · q 退出{match_label}"
                ),
                self.theme.muted_style(),
            ));
        }
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .style(self.theme.panel_style(TuiPanelSurface::Raised))
                .wrap(Wrap { trim: false }),
            content_area,
        );
        scrollbar_track.and_then(|track| {
            super::transcript::scrollbar_metrics(
                mode.lines.len(),
                mode.viewport_rows,
                mode.top_line,
                track.height,
            )
            .map(|metrics| {
                self.render_scrollbar(frame, track, metrics);
                TuiScrollbarFeedback::new(
                    TuiScrollbarView::Transcript,
                    track,
                    metrics,
                    mode.lines.len(),
                    mode.viewport_rows,
                    mode.top_line,
                )
            })
        })
    }

    pub(super) fn render_header(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        header: &TuiStatusHeader,
    ) {
        let block = self.panel_block("会话", TuiPanelSurface::Base, TuiPanelBorder::Standard);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let line = vec![
            self.pill_span(&TuiPill::new(header.badge.clone(), TuiTone::Accent)),
            Span::raw(" "),
            Span::styled(
                header.spinner.clone(),
                self.theme.accent_style().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
            Span::styled(
                header.title.clone(),
                self.theme.body_style().add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            self.pill_span(&TuiPill::new(
                format!("状态 {}", header.terminal),
                header.terminal_tone,
            )),
            Span::raw("  "),
            Span::styled("目录 ", self.theme.muted_style()),
            Span::styled(header.cwd.clone(), self.theme.body_style()),
        ];

        frame.render_widget(
            Paragraph::new(Text::from(vec![Line::from(line)]))
                .style(self.theme.panel_style(TuiPanelSurface::Base))
                .wrap(Wrap { trim: false }),
            inner,
        );
    }

    pub(super) fn render_status_footer(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        footer: &TuiStatusFooter,
    ) {
        let block = self.panel_block("状态", TuiPanelSurface::Base, TuiPanelBorder::Standard);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(self.render_pills_line(&footer.pills, footer.detail.as_deref()))
                .style(self.theme.panel_style(TuiPanelSurface::Base))
                .wrap(Wrap { trim: false }),
            inner,
        );
    }

    pub(super) fn render_project_context_panel(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        project_context: &TuiProjectContextHost,
    ) {
        let block = self.panel_block("项目上下文", TuiPanelSurface::Base, TuiPanelBorder::Standard);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = sidebar_lines(
            self.render_pills_line(&project_context.pills, None),
            &project_context.body_lines,
            inner.height as usize,
        );
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .style(self.theme.panel_style(TuiPanelSurface::Base))
                .wrap(Wrap { trim: false }),
            inner,
        );
    }

    pub(super) fn render_modified_files_panel(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        modified_files: &TuiModifiedFilesHost,
    ) {
        let block =
            self.panel_block("工作区变更", TuiPanelSurface::Raised, TuiPanelBorder::Standard);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let lines = sidebar_lines(
            self.render_pills_line(&modified_files.pills, None),
            &modified_files.body_lines,
            inner.height as usize,
        );
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .style(self.theme.panel_style(TuiPanelSurface::Raised))
                .wrap(Wrap { trim: false }),
            inner,
        );
    }

    pub(super) fn pill_span(&self, pill: &TuiPill) -> Span<'static> {
        Span::styled(format!(" {} ", pill.label), self.pill_style(pill.tone))
    }

    pub(super) fn pill_style(&self, tone: TuiTone) -> Style {
        Style::default()
            .fg(tone_color(tone, self.theme))
            .add_modifier(Modifier::BOLD)
    }

    pub(super) fn render_pills_line(
        &self,
        pills: &[TuiPill],
        detail: Option<&str>,
    ) -> Line<'static> {
        let mut spans = Vec::new();
        for (index, pill) in pills.iter().enumerate() {
            if index > 0 {
                spans.push(Span::raw(" "));
            }
            spans.push(self.pill_span(pill));
        }

        if let Some(detail) = detail.filter(|value| !value.trim().is_empty()) {
            if !spans.is_empty() {
                spans.push(Span::raw("  "));
            }
            spans.push(Span::styled(detail.to_string(), self.theme.muted_style()));
        }

        Line::from(spans)
    }
}

pub(crate) fn build_status_header(
    state: &TuiState,
    status: &TuiStatusSummary,
    badge_label: &str,
    _endpoint_label: &str,
    spinner_frame: usize,
) -> TuiStatusHeader {
    TuiStatusHeader {
        badge: badge_label.to_string(),
        title: if status.title.trim().is_empty() {
            "新会话".to_string()
        } else {
            status.title.clone()
        },
        terminal: terminal_label(&status.turn_terminal).to_string(),
        terminal_tone: terminal_tone(&status.turn_terminal),
        scope: state.session.scope.clone().unwrap_or_else(|| "-".to_string()),
        cwd: compact_path_label(state.project.workspace_root.as_deref()),
        spinner: SPINNER_FRAMES[spinner_frame % SPINNER_FRAMES.len()].to_string(),
    }
}

pub(crate) fn build_status_footer(
    state: &TuiState,
    status: &TuiStatusSummary,
    visible_window: &TuiVisibleTranscriptWindow<'_>,
) -> TuiStatusFooter {
    let mut pills = Vec::new();

    if let Some(error) = state.status.last_error.as_ref() {
        pills.push(TuiPill::new(truncate_label(format!("错误 {error}"), 24), TuiTone::Danger));
    }

    pills.push(TuiPill::new(
        format!("终端 {}", terminal_label(&status.turn_terminal)),
        terminal_tone(&status.turn_terminal),
    ));
    if status.token_usage.input_tokens > 0 || status.token_usage.output_tokens > 0 {
        pills.push(TuiPill::new(
            format!(
                "令牌 {}/{}",
                status.token_usage.input_tokens, status.token_usage.output_tokens
            ),
            TuiTone::Accent,
        ));
    }
    if let Some(unseen_range) = visible_window.unseen_range() {
        pills.push(TuiPill::new(zh_unseen_pill_label(unseen_range), TuiTone::Warning));
    }

    if status.pending_questions > 0 {
        pills.push(TuiPill::new(format!("问题 {}", status.pending_questions), TuiTone::Warning));
    }

    if status.todo_count > 0 {
        pills.push(TuiPill::new(format!("待办 {}", status.todo_count), TuiTone::Warning));
    }

    TuiStatusFooter {
        pills,
        detail: Some("/model 模型 · /acp ACP · $ 技能".to_string()),
    }
}

pub(crate) fn build_activity_indicator(
    state: &TuiState,
    spinner_frame: usize,
) -> Option<TuiActivityIndicator> {
    let spinner = SPINNER_FRAMES[spinner_frame % SPINNER_FRAMES.len()].to_string();
    if state.messages.iter().rev().any(
        |message| matches!(message, UiMessage::ToolCall(call) if call.state == UiToolCallState::Running),
    ) {
        return Some(TuiActivityIndicator { spinner, verb: "Working…".to_string() });
    }

    let thinking = state.messages.iter().rev().find_map(|message| match message {
        UiMessage::Thinking(block) => Some(block),
        _ => None,
    })?;
    match thinking.phase {
        UiThinkingPhase::Streaming => {
            Some(TuiActivityIndicator { spinner, verb: "Thinking…".to_string() })
        }
        UiThinkingPhase::Closed => {
            let timing = thinking.timing.last()?;
            let end_ms = timing.end_ms?;
            let now_ms = state.runtime.last_event_time_ms.unwrap_or(end_ms);
            if now_ms > end_ms.saturating_add(2_000) {
                return None;
            }
            let seconds = end_ms
                .saturating_sub(timing.start_ms)
                .saturating_add(500)
                .checked_div(1_000)
                .unwrap_or_default()
                .max(1);
            Some(TuiActivityIndicator {
                spinner,
                verb: format!("Thought for {seconds}s"),
            })
        }
        UiThinkingPhase::Redacted | UiThinkingPhase::HiddenHistory => None,
    }
}

pub(crate) fn build_project_context_host(
    state: &TuiState,
    status: &TuiStatusSummary,
) -> TuiProjectContextHost {
    let modified_files = state.project.git_status.modified_files();
    let scope = state.session.scope.as_deref().unwrap_or("-");
    let workspace_root = compact_path_label(state.project.workspace_root.as_deref());
    let project_info = if state.project.info.trim().is_empty() {
        "-".to_string()
    } else {
        truncate_label(state.project.info.as_str(), 48)
    };
    let session_title = if state.session.title.trim().is_empty() {
        "-".to_string()
    } else {
        truncate_label(state.session.title.as_str(), 36)
    };

    TuiProjectContextHost {
        pills: vec![
            TuiPill::new(
                format!("范围 {}", truncate_label(scope, 16)),
                if scope == "-" { TuiTone::Muted } else { TuiTone::Accent },
            ),
            TuiPill::new(
                if modified_files.is_empty() {
                    "git 干净".to_string()
                } else {
                    format!("git 脏区 {}", modified_files.len())
                },
                if modified_files.is_empty() { TuiTone::Muted } else { TuiTone::Warning },
            ),
        ],
        body_lines: vec![
            format!("项目: {project_info}"),
            format!("工作区: {workspace_root}"),
            format!("会话标题: {session_title}"),
            format!("消息/步骤: {}/{}", status.message_count, status.step_count),
        ],
    }
}

pub(crate) fn build_modified_files_host(state: &TuiState) -> TuiModifiedFilesHost {
    let modified_files = state.project.git_status.modified_files();
    let body_lines = if modified_files.is_empty() {
        vec!["当前工作区没有文件变更。".to_string()]
    } else {
        modified_files.iter().map(|path| format!("• {path}")).collect()
    };

    TuiModifiedFilesHost {
        pills: vec![TuiPill::new(
            format!("数量 {}", modified_files.len()),
            if modified_files.is_empty() { TuiTone::Muted } else { TuiTone::Accent },
        )],
        body_lines,
    }
}

pub(super) fn sidebar_lines(
    headline: Line<'static>,
    body_lines: &[String],
    max_lines: usize,
) -> Vec<Line<'static>> {
    if max_lines == 0 {
        return Vec::new();
    }

    if max_lines == 1 {
        return vec![headline];
    }

    let available_body_lines = max_lines.saturating_sub(2);
    let clipped_body_lines = clip_sidebar_body_lines(body_lines, available_body_lines);
    std::iter::once(headline)
        .chain(std::iter::once(Line::raw("")))
        .chain(clipped_body_lines.into_iter().map(Line::from))
        .collect()
}

pub(super) fn clip_sidebar_body_lines(lines: &[String], max_lines: usize) -> Vec<String> {
    if max_lines == 0 {
        return Vec::new();
    }

    if lines.len() <= max_lines {
        return lines.to_vec();
    }

    if max_lines == 1 {
        return vec![format!("... +{} more", lines.len())];
    }

    let visible_lines = max_lines.saturating_sub(1);
    let mut clipped = lines.iter().take(visible_lines).cloned().collect::<Vec<_>>();
    clipped.push(format!("... +{} more", lines.len().saturating_sub(visible_lines)));
    clipped
}

pub(super) fn terminal_label(terminal: &UiTurnTerminal) -> &'static str {
    match terminal {
        UiTurnTerminal::Pending => "待命",
        UiTurnTerminal::Streaming => "输出中",
        UiTurnTerminal::Done { .. } => "完成",
        UiTurnTerminal::Cancelled { .. } => "已取消",
        UiTurnTerminal::TimedOut { .. } => "超时",
        UiTurnTerminal::Error { .. } => "错误",
    }
}

pub(super) fn terminal_tone(terminal: &UiTurnTerminal) -> TuiTone {
    match terminal {
        UiTurnTerminal::Pending => TuiTone::Muted,
        UiTurnTerminal::Streaming => TuiTone::Accent,
        UiTurnTerminal::Done { .. } => TuiTone::Success,
        UiTurnTerminal::Cancelled { .. } => TuiTone::Warning,
        UiTurnTerminal::TimedOut { .. } | UiTurnTerminal::Error { .. } => TuiTone::Danger,
    }
}

pub(super) fn step_state_label(state: &UiStepState) -> &'static str {
    match state {
        UiStepState::Pending => "等待中",
        UiStepState::Running => "进行中",
        UiStepState::Complete => "完成",
        UiStepState::Cancelled => "已取消",
        UiStepState::Failed => "失败",
    }
}

pub(super) fn tone_color(tone: TuiTone, theme: TuiTheme) -> Color {
    match tone {
        TuiTone::Accent => theme.accent,
        TuiTone::Muted => theme.border,
        TuiTone::Success => theme.success,
        TuiTone::Warning => theme.warning,
        TuiTone::Danger => theme.danger,
    }
}
