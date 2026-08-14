use super::{
    Clear, Line, Modifier, Paragraph, Path, PathBuf, PromptMode, PromptSubmissionStatus,
    QueuedPromptCommand, QueuedPromptCommandKind, QueuedPromptPriority, Rect, Span, Style, Text,
    TuiCursorPlacement, TuiPanelBorder, TuiPanelSurface, TuiPill, TuiRenderer, TuiState, TuiTheme,
    TuiTone, UnicodeWidthStr, prompt_suggestions, selected_suggestion_index, usize_to_u16,
};
use crate::cli::tui_v2::completion::{CompletionKind, completion_notices};
use crate::cli::tui_v2::editor::EditorSelection;
use crate::cli::tui_v2::input::{FILE_PICKER_VISIBLE_ROWS, is_file_picker_suggestions};
use unicode_segmentation::UnicodeSegmentation;

/// prompt frame primitive。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiPromptHost {
    pub(crate) value: String,
    pub(crate) placeholder: String,
    pub(crate) helper_text: String,
    pub(crate) suggestions: Vec<TuiPill>,
    pub(crate) suggestion_rows: Vec<TuiPromptSuggestionRow>,
    pub(crate) suggestion_detail: Option<String>,
    pub(crate) queued_commands: Vec<TuiPill>,
    pub(crate) footer_pills: Vec<TuiPill>,
    pub(crate) cursor_char_index: usize,
    pub(crate) selection: Option<EditorSelection>,
}

impl TuiPromptHost {
    pub(super) fn extra_prompt_rows(&self) -> u16 {
        if self.queued_commands.is_empty() { 1 } else { 2 }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiPromptSuggestionRow {
    pub(crate) label: String,
    pub(crate) detail: Option<String>,
    pub(crate) kind: Option<CompletionKind>,
    pub(crate) selected: bool,
}

impl TuiRenderer {
    pub(super) fn render_prompt_host(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        prompt_host: &TuiPromptHost,
        prompt_has_focus: bool,
    ) -> Option<TuiCursorPlacement> {
        let input_height = area.height.saturating_sub(2).max(1).min(area.height);
        let input_area = Rect::new(area.x, area.y, area.width, input_height);
        let footer_x = area.x.saturating_add(1);
        let footer_width = area.width.saturating_sub(2);
        let status_area = Rect::new(
            footer_x,
            input_area.y.saturating_add(input_area.height),
            footer_width,
            u16::from(area.height > input_height),
        );
        let shortcut_area = Rect::new(
            footer_x,
            status_area.y.saturating_add(status_area.height),
            footer_width,
            u16::from(area.height > input_height.saturating_add(status_area.height)),
        );
        let block = self.panel_block("❯", TuiPanelSurface::Base, TuiPanelBorder::Standard);
        let inner = block.inner(input_area);
        frame.render_widget(block, input_area);

        if inner.width == 0 || inner.height == 0 {
            return None;
        }

        let suggestion_lines = self.render_prompt_suggestion_rows(&prompt_host.suggestion_rows);
        let suggestion_detail_line = prompt_host
            .suggestion_rows
            .is_empty()
            .then(|| prompt_host.suggestion_detail.as_deref())
            .flatten()
            .filter(|detail| !detail.trim().is_empty())
            .map(|detail| {
                Line::from(vec![
                    Span::styled("补全 ", self.theme.accent_style().add_modifier(Modifier::BOLD)),
                    Span::styled(detail.to_string(), self.theme.muted_style()),
                ])
            });
        let has_queued_commands = !prompt_host.queued_commands.is_empty();
        let reserved_rows = usize::from(has_queued_commands);
        let prompt_rows = (inner.height as usize).saturating_sub(reserved_rows).max(1);
        let prompt_viewport = render_prompt_value_lines(prompt_host, self.theme, prompt_rows);

        let queue_line = if has_queued_commands {
            Some(self.render_pills_line(&prompt_host.queued_commands, None))
        } else {
            None
        };

        let cursor = prompt_has_focus.then_some(TuiCursorPlacement {
            x: inner.x.saturating_add(prompt_viewport.cursor.x),
            y: inner.y.saturating_add(prompt_viewport.cursor.y),
        });

        frame.render_widget(
            Paragraph::new(Text::from(
                prompt_viewport.lines.into_iter().chain(queue_line).collect::<Vec<_>>(),
            ))
            .style(self.theme.panel_style(TuiPanelSurface::Base)),
            // prompt host 的光标与 reserved_rows 都按“逻辑行 = 屏幕行”计算，
            // 这里不能再启用软换行，否则 helper/suggestion/footer 会偷偷多占行。
            inner,
        );
        frame.render_widget(
            Paragraph::new(self.render_pills_line(&prompt_host.footer_pills, None)),
            status_area,
        );
        frame.render_widget(
            Paragraph::new(Line::styled(prompt_host.helper_text.clone(), self.theme.muted_style())),
            shortcut_area,
        );

        let mut completion_lines = suggestion_lines;
        completion_lines.extend(suggestion_detail_line);
        if !completion_lines.is_empty() {
            let popup_height =
                u16::try_from(completion_lines.len()).unwrap_or(u16::MAX).min(area.y);
            if popup_height > 0 {
                let popup = Rect::new(
                    area.x,
                    area.y.saturating_sub(popup_height),
                    area.width,
                    popup_height,
                );
                frame.render_widget(Clear, popup);
                frame.render_widget(
                    Paragraph::new(Text::from(completion_lines))
                        .style(self.theme.panel_style(TuiPanelSurface::Raised)),
                    popup,
                );
            }
        }

        cursor
    }

    pub(super) fn render_prompt_suggestion_rows(
        &self,
        rows: &[TuiPromptSuggestionRow],
    ) -> Vec<Line<'static>> {
        rows.iter()
            .map(|row| {
                if matches!(row.kind, Some(CompletionKind::Directory | CompletionKind::File)) {
                    let marker = if row.selected { ">" } else { " " };
                    let kind =
                        if row.kind == Some(CompletionKind::Directory) { "[D]" } else { "[F]" };
                    let style = if row.selected {
                        selected_completion_style(self.theme)
                    } else {
                        self.theme.body_style()
                    };
                    return Line::styled(
                        format!("{marker} {kind} {}", truncate_label(&row.label, 64)),
                        style,
                    );
                }
                let marker = if row.selected { ">" } else { " " };
                let marker_style = if row.selected {
                    self.theme.accent_style().add_modifier(Modifier::BOLD)
                } else {
                    self.theme.muted_style()
                };
                let label_style = if row.selected {
                    self.theme.body_style().add_modifier(Modifier::BOLD)
                } else {
                    self.theme.body_style()
                };
                let detail_style = if row.selected {
                    self.theme.muted_style().add_modifier(Modifier::ITALIC)
                } else {
                    self.theme.muted_style()
                };
                let detail = row
                    .detail
                    .as_ref()
                    .map(|detail| format!(" · {}", truncate_label(detail, 44)))
                    .unwrap_or_default();

                Line::from(vec![
                    Span::styled(format!("{marker} "), marker_style),
                    Span::styled(truncate_label(&row.label, 34), label_style),
                    Span::styled(detail, detail_style),
                ])
            })
            .collect()
    }
}

pub(crate) fn build_prompt_host(state: &TuiState) -> TuiPromptHost {
    const MAX_VISIBLE_SUGGESTION_ROWS: usize = 3;

    let suggestions = prompt_suggestions(state);
    let is_file_picker = is_file_picker_suggestions(&suggestions);
    let max_visible_suggestion_rows =
        if is_file_picker { FILE_PICKER_VISIBLE_ROWS } else { MAX_VISIBLE_SUGGESTION_ROWS };
    let suggestion_total = suggestions.len();
    let selected_index = selected_suggestion_index(state, &suggestions);
    let (visible_start, visible_end) = visible_prompt_suggestion_window(
        suggestion_total,
        selected_index.unwrap_or_default(),
        max_visible_suggestion_rows,
    );
    let visible_suggestions = &suggestions[visible_start..visible_end];
    let mut suggestion_pills = suggestions
        .iter()
        .enumerate()
        .skip(visible_start)
        .take(visible_suggestions.len())
        .map(|(index, suggestion)| {
            TuiPill::new(
                truncate_label(suggestion.label.as_str(), 20),
                if Some(index) == selected_index { TuiTone::Accent } else { TuiTone::Muted },
            )
        })
        .collect::<Vec<_>>();
    let mut suggestion_rows = visible_suggestions
        .iter()
        .enumerate()
        .map(|(offset, suggestion)| TuiPromptSuggestionRow {
            label: if matches!(suggestion.kind, CompletionKind::Directory | CompletionKind::File) {
                compact_file_suggestion_label(suggestion.label.as_str())
            } else {
                suggestion.label.clone()
            },
            detail: if matches!(suggestion.kind, CompletionKind::Directory | CompletionKind::File) {
                None
            } else {
                Some(match suggestion.detail.as_deref() {
                    Some(detail) => format!("{} · {detail}", suggestion.kind.label()),
                    None => suggestion.kind.label().to_string(),
                })
            },
            kind: Some(suggestion.kind),
            selected: Some(visible_start + offset) == selected_index,
        })
        .collect::<Vec<_>>();
    let mut suggestion_detail = (suggestion_total == 0)
        .then(|| {
            matches!(state.prompt.mode, PromptMode::SlashCommand)
                .then(|| "未找到匹配命令，继续输入或退格调整。".to_string())
        })
        .flatten();
    if state.history_search.active {
        suggestion_pills.clear();
        suggestion_rows = state
            .history_search
            .matches
            .iter()
            .rev()
            .take(MAX_VISIBLE_SUGGESTION_ROWS)
            .rev()
            .enumerate()
            .filter_map(|(offset, entry_index)| {
                state.prompt.history.entries.get(*entry_index).map(|entry| {
                    let match_index = state
                        .history_search
                        .matches
                        .len()
                        .saturating_sub(MAX_VISIBLE_SUGGESTION_ROWS)
                        .saturating_add(offset);
                    TuiPromptSuggestionRow {
                        label: entry.clone(),
                        detail: Some("历史".to_string()),
                        kind: None,
                        selected: Some(match_index) == state.history_search.selected_index,
                    }
                })
            })
            .collect();
        suggestion_detail =
            Some(format!("历史搜索 /{} · Enter 选择 · Esc 恢复草稿", state.history_search.query));
    }
    let notices = completion_notices(state);
    if !notices.is_empty() {
        suggestion_detail = Some(notices.join("；"));
    }
    let delegate_agent_label = state
        .session_capabilities
        .delegate_agents
        .iter()
        .find(|agent| agent.key == state.session_options.delegate_agent)
        .map(|agent| agent.label.as_str())
        .unwrap_or_else(|| state.session_options.delegate_agent.as_deref().unwrap_or("main"));
    let acp_backend_label = state.session_options.acp_backend.as_deref().unwrap_or("默认");

    let mut queued_commands =
        state.prompt.queued_commands.iter().take(3).map(queue_command_pill).collect::<Vec<_>>();
    if state.prompt.stashed_draft.is_some() {
        queued_commands.push(TuiPill::new("草稿已暂存", TuiTone::Muted));
    }
    let remaining = state.prompt.queued_commands.len().saturating_sub(queued_commands.len());
    if remaining > 0 {
        queued_commands.push(TuiPill::new(format!("+{remaining} more"), TuiTone::Muted));
    }

    TuiPromptHost {
        value: state.prompt.value.clone(),
        placeholder: if state.history_search.active {
            "搜索历史…".to_string()
        } else if matches!(state.prompt.mode, PromptMode::SlashCommand) {
            "输入 / 命令…".to_string()
        } else {
            "输入消息…".to_string()
        },
        helper_text: if state.history_search.active {
            "Ctrl+R 历史搜索：输入筛选，Enter 选择，Esc 取消。".to_string()
        } else if state.prompt.is_busy() {
            "当前轮次仍在输出，新输入会暂存到队列。".to_string()
        } else if matches!(state.prompt.mode, PromptMode::SlashCommand) {
            if suggestion_total > 0 {
                format!(
                    "斜杠命令模式：补全 {} 项，Up/Down 切换，Tab/Enter 接受当前项；接受后再按 Enter 执行。",
                    suggestion_total
                )
            } else {
                "斜杠命令模式：继续输入命令或参数，Enter 执行。".to_string()
            }
        } else if state.prompt.editor.selection.is_some() {
            "Enter 发送，Shift+Enter 换行，Ctrl+C 复制选区。".to_string()
        } else {
            "Enter 发送，Shift+Enter 换行，Ctrl+C/Ctrl+D 退出。".to_string()
        },
        suggestions: suggestion_pills,
        suggestion_rows,
        suggestion_detail,
        queued_commands,
        footer_pills: vec![
            TuiPill::new(
                format!(
                    "模式 {}",
                    if state.history_search.active {
                        "历史搜索"
                    } else if state.prompt.is_busy() {
                        "忙碌"
                    } else if matches!(state.prompt.mode, PromptMode::SlashCommand) {
                        "命令"
                    } else {
                        "输入"
                    }
                ),
                if state.prompt.is_busy() { TuiTone::Warning } else { TuiTone::Accent },
            ),
            TuiPill::new(
                format!(
                    "模型 {}",
                    truncate_label(state.status.model_name.as_deref().unwrap_or("-"), 28)
                ),
                TuiTone::Accent,
            ),
            TuiPill::new(
                format!("代理 {}", truncate_label(delegate_agent_label, 20)),
                TuiTone::Muted,
            ),
            TuiPill::new(format!("ACP {}", truncate_label(acp_backend_label, 20)), TuiTone::Muted),
            TuiPill::new(
                if state.session_options.full_access_enabled() {
                    "完全访问 已启用"
                } else {
                    "完全访问 关闭"
                },
                if state.session_options.full_access_enabled() {
                    TuiTone::Danger
                } else {
                    TuiTone::Muted
                },
            ),
            TuiPill::new(
                if state.session_options.subagent_mode_enabled() {
                    "子智能体 开"
                } else {
                    "子智能体 关"
                },
                if state.session_options.subagent_mode_enabled() {
                    TuiTone::Accent
                } else {
                    TuiTone::Muted
                },
            ),
            TuiPill::new(
                if state.session_options.workflow_mode_enabled() {
                    "工作流 开"
                } else {
                    "工作流 关"
                },
                if state.session_options.workflow_mode_enabled() {
                    TuiTone::Accent
                } else {
                    TuiTone::Muted
                },
            ),
        ],
        cursor_char_index: state.prompt.cursor.char_index,
        selection: state.prompt.editor.selection,
    }
}

fn compact_file_suggestion_label(label: &str) -> String {
    let label = label.strip_prefix('@').unwrap_or(label);
    let Some(quoted) = label.strip_prefix('"') else {
        return label.to_string();
    };
    let Some(quote_end) = quoted.rfind('"') else {
        return label.to_string();
    };
    format!("{}{}", &quoted[..quote_end], &quoted[quote_end + 1..])
}

fn selected_completion_style(theme: TuiTheme) -> Style {
    Style::default().fg(theme.accent).add_modifier(Modifier::BOLD)
}

pub(super) fn prompt_shortcut_hint(state: &TuiState) -> String {
    if state.prompt.is_busy() {
        return "当前轮次仍在输出，新输入会暂存到队列".to_string();
    }

    if state.history_search.active {
        return "Ctrl+R 历史搜索 · Enter 选择 · Esc 取消".to_string();
    }

    if matches!(state.prompt.mode, PromptMode::SlashCommand) {
        return "Up/Down 切换 · Tab/Enter 接受 · Enter 执行".to_string();
    }

    if state.prompt.editor.selection.is_some() {
        "Enter 发送 · Shift+Enter 换行 · Ctrl+C 复制选区".to_string()
    } else {
        "Enter 发送 · Shift+Enter 换行 · Ctrl+R 历史 · Ctrl+C/D 退出".to_string()
    }
}

pub(super) fn visible_prompt_suggestion_window(
    total: usize,
    selected_index: usize,
    max_visible: usize,
) -> (usize, usize) {
    if total <= max_visible {
        return (0, total);
    }

    let half = max_visible / 2;
    let mut start = selected_index.saturating_sub(half);
    let mut end = start.saturating_add(max_visible).min(total);
    if end.saturating_sub(start) < max_visible {
        start = end.saturating_sub(max_visible);
    }
    if end == total {
        start = total.saturating_sub(max_visible);
    }
    end = start.saturating_add(max_visible).min(total);
    (start, end)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct RenderedPromptViewport {
    pub(super) lines: Vec<Line<'static>>,
    pub(super) cursor: TuiCursorPlacement,
    pub(super) hidden_above: usize,
}

pub(super) fn render_prompt_value_lines(
    prompt_host: &TuiPromptHost,
    theme: TuiTheme,
    viewport_rows: usize,
) -> RenderedPromptViewport {
    let viewport_rows = viewport_rows.max(1);
    let (cursor_line, cursor_column) =
        prompt_cursor_line_column(prompt_host.value.as_str(), prompt_host.cursor_char_index);

    let raw_lines = if prompt_host.value.is_empty() {
        vec![""]
    } else {
        prompt_host.value.split('\n').collect::<Vec<_>>()
    };
    let total_lines = raw_lines.len().max(1);
    let visible_start = cursor_line.saturating_add(1).saturating_sub(viewport_rows);
    let visible_end = visible_start.saturating_add(viewport_rows).min(total_lines);

    let mut line_start_graphemes = Vec::with_capacity(total_lines);
    let mut next_line_start = 0usize;
    for line in &raw_lines {
        line_start_graphemes.push(next_line_start);
        next_line_start = next_line_start
            .saturating_add(UnicodeSegmentation::graphemes(*line, true).count())
            .saturating_add(1);
    }

    let mut lines = Vec::with_capacity(visible_end.saturating_sub(visible_start));
    for index in visible_start..visible_end {
        let prefix = prompt_line_prefix(index);
        let text = raw_lines.get(index).copied().unwrap_or_default();
        let text_spans = if prompt_host.value.is_empty() {
            vec![Span::styled(prompt_host.placeholder.clone(), Style::default().fg(theme.muted))]
        } else {
            prompt_line_value_spans(
                text,
                line_start_graphemes.get(index).copied().unwrap_or_default(),
                prompt_host.selection,
                theme,
            )
        };
        let mut spans = Vec::with_capacity(text_spans.len().saturating_add(1));
        spans.push(Span::styled(prefix, Style::default().fg(theme.accent)));
        spans.extend(text_spans);
        lines.push(Line::from(spans));
    }

    let current_line =
        raw_lines.get(cursor_line.min(total_lines.saturating_sub(1))).copied().unwrap_or_default();
    let current_prefix = prompt_line_prefix(cursor_line);
    let cursor_prefix_width = UnicodeWidthStr::width(current_prefix);
    let cursor_text =
        UnicodeSegmentation::graphemes(current_line, true).take(cursor_column).collect::<String>();
    let cursor_text =
        crate::cli::tui_v2::terminal_capabilities::sanitize_terminal_text(cursor_text.as_str());
    let cursor_x = cursor_prefix_width.saturating_add(UnicodeWidthStr::width(cursor_text.as_str()));
    let cursor_y = cursor_line.saturating_sub(visible_start);

    RenderedPromptViewport {
        lines,
        cursor: TuiCursorPlacement { x: usize_to_u16(cursor_x), y: usize_to_u16(cursor_y) },
        hidden_above: visible_start,
    }
}

fn prompt_line_value_spans(
    value: &str,
    line_start: usize,
    selection: Option<EditorSelection>,
    theme: TuiTheme,
) -> Vec<Span<'static>> {
    let graphemes = UnicodeSegmentation::graphemes(value, true).collect::<Vec<_>>();
    let line_end = line_start.saturating_add(graphemes.len());
    let Some(selection) = selection else {
        return vec![Span::styled(
            crate::cli::tui_v2::terminal_capabilities::sanitize_terminal_text(value),
            Style::default().fg(theme.text),
        )];
    };
    let (selection_start, selection_end) = selection.ordered();
    let selected_start = selection_start.clamp(line_start, line_end).saturating_sub(line_start);
    let selected_end = selection_end.clamp(line_start, line_end).saturating_sub(line_start);
    if selected_start >= selected_end {
        return vec![Span::styled(
            crate::cli::tui_v2::terminal_capabilities::sanitize_terminal_text(value),
            Style::default().fg(theme.text),
        )];
    }

    let normal_style = Style::default().fg(theme.text);
    let selection_style = if theme.accent == ratatui::style::Color::Reset {
        Style::default().add_modifier(Modifier::BOLD | Modifier::REVERSED)
    } else {
        Style::default().fg(theme.background).bg(theme.accent).add_modifier(Modifier::BOLD)
    };
    let mut spans = Vec::with_capacity(3);
    push_prompt_span(&mut spans, &graphemes[..selected_start], normal_style);
    push_prompt_span(&mut spans, &graphemes[selected_start..selected_end], selection_style);
    push_prompt_span(&mut spans, &graphemes[selected_end..], normal_style);
    spans
}

fn push_prompt_span(spans: &mut Vec<Span<'static>>, graphemes: &[&str], style: Style) {
    if graphemes.is_empty() {
        return;
    }
    let value = graphemes.concat();
    spans.push(Span::styled(
        crate::cli::tui_v2::terminal_capabilities::sanitize_terminal_text(value.as_str()),
        style,
    ));
}

pub(super) fn queue_command_pill(command: &QueuedPromptCommand) -> TuiPill {
    let kind = match command.kind {
        QueuedPromptCommandKind::Submit => "发送",
        QueuedPromptCommandKind::SlashCommand => "命令",
    };
    let timing = match command.priority {
        QueuedPromptPriority::Now => "立即",
        QueuedPromptPriority::NextTool => "工具后",
        QueuedPromptPriority::EndTurn => "回合后",
    };
    TuiPill::new(format!("{kind} · {timing}"), TuiTone::Warning)
}

pub(super) fn last_submission_pill(state: &TuiState) -> TuiPill {
    let Some(submission) = state.prompt.last_submission.as_ref() else {
        return TuiPill::new("上次空闲", TuiTone::Muted);
    };

    match &submission.status {
        PromptSubmissionStatus::Pending => TuiPill::new("上次等待", TuiTone::Muted),
        PromptSubmissionStatus::Streaming => TuiPill::new("上次输出中", TuiTone::Accent),
        PromptSubmissionStatus::Done { .. } => TuiPill::new("上次完成", TuiTone::Success),
        PromptSubmissionStatus::Cancelled { .. } => TuiPill::new("上次取消", TuiTone::Warning),
        PromptSubmissionStatus::TimedOut { .. } | PromptSubmissionStatus::Error { .. } => {
            TuiPill::new("上次失败", TuiTone::Danger)
        }
    }
}

pub(super) fn prompt_mode_label(mode: &PromptMode) -> &'static str {
    match mode {
        PromptMode::Compose => "对话",
        PromptMode::SlashCommand => "命令",
        PromptMode::Bash => "Bash",
        PromptMode::Search => "搜索",
        PromptMode::QuestionReply => "问答",
        PromptMode::TodoEdit => "待办",
        PromptMode::CommandPalette => "面板",
        PromptMode::Busy => "忙碌",
    }
}

pub(super) fn prompt_line_prefix(line_index: usize) -> &'static str {
    if line_index == 0 { "" } else { "· " }
}

pub(super) fn prompt_cursor_line_column(value: &str, char_index: usize) -> (usize, usize) {
    let mut current_line = 0usize;
    let mut current_column = 0usize;

    for (index, grapheme) in UnicodeSegmentation::graphemes(value, true).enumerate() {
        if index >= char_index {
            break;
        }

        if grapheme == "\n" {
            current_line = current_line.saturating_add(1);
            current_column = 0;
        } else {
            current_column = current_column.saturating_add(1);
        }
    }

    (current_line, current_column)
}

pub(super) fn compact_path_label(path: Option<&Path>) -> String {
    let Some(path) = path else {
        return "-".to_string();
    };

    if let Some(home) = std::env::var_os("HOME") {
        let home_path = PathBuf::from(home);
        if let Ok(stripped) = path.strip_prefix(&home_path) {
            if stripped.as_os_str().is_empty() {
                return "~".to_string();
            }
            return truncate_start(format!("~/{}", stripped.display()), 28);
        }
    }

    truncate_start(path.display().to_string(), 28)
}

pub(super) fn truncate_label(value: impl AsRef<str>, max_chars: usize) -> String {
    let value = value.as_ref();
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }

    if max_chars <= 3 {
        return value.chars().take(max_chars).collect();
    }

    let prefix = value.chars().take(max_chars.saturating_sub(3)).collect::<String>();
    format!("{prefix}...")
}

pub(super) fn truncate_start(value: impl AsRef<str>, max_chars: usize) -> String {
    let value = value.as_ref();
    let count = value.chars().count();
    if count <= max_chars {
        return value.to_string();
    }

    if max_chars <= 3 {
        return value.chars().skip(count.saturating_sub(max_chars)).collect();
    }

    let suffix =
        value.chars().skip(count.saturating_sub(max_chars.saturating_sub(3))).collect::<String>();
    format!("...{suffix}")
}
