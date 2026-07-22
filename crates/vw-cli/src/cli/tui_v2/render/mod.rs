//! tui_v2 renderer 宿主。
//!
//! 当前 renderer 已从 S3-1 的纯 skeleton 文案升级为更稳定的宿主原语：
//! - 主题 token 统一管理 panel、边框、强调与状态色
//! - status header / footer 作为独立 primitive 供 fullscreen layout 组合
//! - prompt frame 承担输入、footer pills 与 queued commands 的稳定布局
//! - modal host 通过 overlay stack 组合器统一承接弹层标题、焦点和栈顺序

pub(crate) mod layout;
#[cfg(test)]
#[path = "layout_tests.rs"]
mod layout_tests;
mod message_row;
#[cfg(test)]
#[path = "message_row_tests.rs"]
mod message_row_tests;

use std::path::{Path, PathBuf};

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use super::input::{prompt_suggestions, selected_suggestion_index};
use super::model::{
    OverlayFocus, PromptMode, PromptSubmissionStatus, QueuedPromptCommand, QueuedPromptCommandKind,
    UiOverlay, UiOverlayKind, UiQuestionSurfaceKind, UiStepState, UiTurnTerminal,
};
use super::state::{
    TuiState, TuiStatusSummary, TuiVisibleTranscriptWindow, select_status_summary,
    select_visible_grouped_transcript_window,
};
use layout::FullscreenLayoutSlots;
use message_row::render_transcript_item_lines;

const SPINNER_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];

#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TuiCursorPlacement {
    pub(crate) x: u16,
    pub(crate) y: u16,
}

/// fullscreen skeleton 的 render 输出反馈。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TuiRenderFeedback {
    pub(crate) layout: FullscreenLayoutSlots,
    pub(crate) cursor: Option<TuiCursorPlacement>,
}

/// 统一的 pill 色调枚举，避免 header/footer/prompt 各自散落颜色判断。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiTone {
    Accent,
    Muted,
    Success,
    Warning,
    Danger,
}

/// 可复用的轻量状态 pill。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiPill {
    pub(crate) label: String,
    pub(crate) tone: TuiTone,
}

impl TuiPill {
    pub(crate) fn new(label: impl Into<String>, tone: TuiTone) -> Self {
        Self { label: label.into(), tone }
    }
}

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

/// prompt frame primitive。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiPromptHost {
    pub(crate) value: String,
    pub(crate) placeholder: String,
    pub(crate) helper_text: String,
    pub(crate) path_label: String,
    pub(crate) suggestions: Vec<TuiPill>,
    pub(crate) suggestion_rows: Vec<TuiPromptSuggestionRow>,
    pub(crate) suggestion_detail: Option<String>,
    pub(crate) queued_commands: Vec<TuiPill>,
    pub(crate) footer_pills: Vec<TuiPill>,
    pub(crate) cursor_char_index: usize,
}

impl TuiPromptHost {
    fn extra_prompt_rows(&self) -> u16 {
        u16::try_from(self.suggestion_rows.len().saturating_sub(1)).unwrap_or(u16::MAX)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiPromptSuggestionRow {
    pub(crate) label: String,
    pub(crate) detail: Option<String>,
    pub(crate) selected: bool,
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

/// modal compositor 输出的聚合结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TuiModalHost {
    pub(crate) title: String,
    pub(crate) chips: Vec<TuiPill>,
    pub(crate) body_lines: Vec<String>,
}

/// tui_v2 当前阶段使用的主题 token。
#[derive(Debug, Clone, Copy)]
pub(crate) struct TuiTheme {
    pub(crate) background: Color,
    pub(crate) surface: Color,
    pub(crate) surface_alt: Color,
    pub(crate) border: Color,
    pub(crate) accent: Color,
    pub(crate) text: Color,
    pub(crate) muted: Color,
    pub(crate) success: Color,
    pub(crate) warning: Color,
    pub(crate) danger: Color,
}

impl Default for TuiTheme {
    fn default() -> Self {
        Self {
            background: Color::Rgb(16, 15, 14),
            surface: Color::Rgb(24, 22, 20),
            surface_alt: Color::Rgb(31, 28, 25),
            border: Color::Rgb(119, 89, 67),
            accent: Color::Rgb(234, 136, 75),
            text: Color::Rgb(239, 230, 222),
            muted: Color::Rgb(172, 158, 147),
            success: Color::Rgb(121, 201, 138),
            warning: Color::Rgb(244, 191, 92),
            danger: Color::Rgb(231, 122, 104),
        }
    }
}

/// panel 使用的背景层级，避免 surface 判断散落在 renderer 各处。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiPanelSurface {
    Base,
    Raised,
}

/// panel 边框语义，仅区分普通信息面与当前强调面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TuiPanelBorder {
    Standard,
    Focused,
}

impl TuiTheme {
    fn panel_background(self, surface: TuiPanelSurface) -> Color {
        match surface {
            TuiPanelSurface::Base => self.surface,
            TuiPanelSurface::Raised => self.surface_alt,
        }
    }

    fn panel_style(self, surface: TuiPanelSurface) -> Style {
        Style::default().bg(self.panel_background(surface))
    }

    fn panel_border_style(self, border: TuiPanelBorder) -> Style {
        let color = match border {
            TuiPanelBorder::Standard => self.border,
            TuiPanelBorder::Focused => self.accent,
        };
        Style::default().fg(color)
    }

    fn panel_title(self, title: impl Into<String>) -> Span<'static> {
        Span::styled(format!(" {} ", title.into()), self.accent_style())
    }

    fn fg_style(self, color: Color) -> Style {
        Style::default().fg(color)
    }

    fn accent_style(self) -> Style {
        self.fg_style(self.accent)
    }

    fn body_style(self) -> Style {
        self.fg_style(self.text)
    }

    fn muted_style(self) -> Style {
        self.fg_style(self.muted)
    }
}

/// tui_v2 fullscreen skeleton renderer。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TuiRenderer {
    theme: TuiTheme,
}

impl TuiRenderer {
    fn panel_block(
        &self,
        title: impl Into<String>,
        surface: TuiPanelSurface,
        border: TuiPanelBorder,
    ) -> Block<'static> {
        Block::default()
            .style(self.theme.panel_style(surface))
            .borders(Borders::ALL)
            .border_style(self.theme.panel_border_style(border))
            .title(self.theme.panel_title(title))
    }

    /// 将当前状态绘制到一帧 Ratatui frame 中。
    pub(crate) fn render_frame(
        &self,
        frame: &mut ratatui::Frame<'_>,
        state: &TuiState,
        badge_label: &str,
        endpoint_label: &str,
        spinner_frame: usize,
    ) -> TuiRenderFeedback {
        let area = frame.area();
        let status = select_status_summary(state);
        let visible_window = select_visible_grouped_transcript_window(state);
        let header =
            build_status_header(state, &status, badge_label, endpoint_label, spinner_frame);
        let footer = build_status_footer(state, &status, &visible_window);
        let prompt_host = build_prompt_host(state);
        let layout = layout::compute_fullscreen_layout(
            area,
            true,
            state.overlays.active().is_some(),
            prompt_host.extra_prompt_rows(),
        );
        let project_context = build_project_context_host(state, &status);
        let modified_files = build_modified_files_host(state);
        let modal_host = build_modal_host(state);
        let prompt_has_focus =
            matches!(state.overlays.focus, OverlayFocus::Prompt) && !state.prompt.is_busy();

        frame.render_widget(
            Block::default().style(Style::default().bg(self.theme.background)),
            area,
        );

        self.render_header(frame, layout.header, &header);
        self.render_scrollable(frame, layout.scrollable, &visible_window);

        if let Some(sidebar_area) = layout.project_context {
            self.render_project_context_panel(frame, sidebar_area, &project_context);
        }

        if let Some(sidebar_area) = layout.modified_files {
            self.render_modified_files_panel(frame, sidebar_area, &modified_files);
        }

        if let Some(bottom_float) = layout.bottom_float {
            self.render_status_footer(frame, bottom_float, &footer);
        }

        let cursor = self.render_prompt_host(frame, layout.bottom, &prompt_host, prompt_has_focus);
        if let Some(cursor) = cursor {
            frame.set_cursor_position((cursor.x, cursor.y));
        }

        if let (Some(modal_area), Some(modal_host)) = (layout.modal, modal_host.as_ref()) {
            self.render_modal_host(frame, modal_area, modal_host);
        }

        TuiRenderFeedback { layout, cursor }
    }

    fn render_header(&self, frame: &mut ratatui::Frame<'_>, area: Rect, header: &TuiStatusHeader) {
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

    fn render_scrollable(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        visible_window: &TuiVisibleTranscriptWindow<'_>,
    ) {
        let block = self.panel_block("对话流", TuiPanelSurface::Raised, TuiPanelBorder::Standard);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        let window_summary = visible_window.window_summary();
        let lines = if visible_window.is_empty() {
            vec![Line::from(vec![
                Span::styled("还没有消息。", self.theme.muted_style()),
                Span::raw(" 在下方输入内容并回车即可开始会话。"),
            ])]
        } else {
            let mut lines = Vec::new();
            if let Some(sticky_prompt) = visible_window.sticky_prompt() {
                lines.push(Line::from(vec![
                    Span::styled(
                        "上一条输入 ",
                        self.theme.accent_style().add_modifier(Modifier::BOLD),
                    ),
                    Span::styled(
                        sticky_prompt.label(),
                        self.theme.body_style().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(sticky_prompt.preview.clone(), self.theme.muted_style()),
                ]));
                lines.push(Line::raw(""));
            } else if let Some(sticky_notice) = window_summary.sticky_notice() {
                lines.push(Line::from(vec![
                    Span::styled("锚点 ", self.theme.accent_style().add_modifier(Modifier::BOLD)),
                    Span::styled(zh_sticky_notice(sticky_notice), self.theme.muted_style()),
                ]));
                lines.push(Line::raw(""));
            }

            let unseen_range = visible_window.unseen_range();
            for (offset, item) in visible_window.visible_items().iter().enumerate() {
                let item_index = visible_window.top_item_index.saturating_add(offset);
                if let Some(unseen_range) = unseen_range
                    && unseen_range.first_unseen_item_index == item_index
                {
                    lines.push(Line::from(vec![
                        Span::styled(
                            "未读 ",
                            Style::default().fg(self.theme.warning).add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(
                            zh_unseen_divider_label(unseen_range),
                            self.theme.muted_style(),
                        ),
                    ]));
                    lines.push(Line::raw(""));
                }

                lines.extend(render_transcript_item_lines(item, self.theme));
                lines.push(Line::raw(""));
            }
            lines
        };

        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .style(self.theme.panel_style(TuiPanelSurface::Raised))
                .wrap(Wrap { trim: false }),
            inner,
        );
    }

    fn render_status_footer(
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

    fn render_prompt_host(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        prompt_host: &TuiPromptHost,
        prompt_has_focus: bool,
    ) -> Option<TuiCursorPlacement> {
        let block = self.panel_block("输入区", TuiPanelSurface::Base, TuiPanelBorder::Standard);
        let inner = block.inner(area);
        frame.render_widget(block, area);

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
        let reserved_rows = 1usize
            .saturating_add(suggestion_lines.len())
            .saturating_add(usize::from(suggestion_detail_line.is_some()))
            .saturating_add(usize::from(has_queued_commands));
        let prompt_rows = (inner.height as usize).saturating_sub(reserved_rows).max(1);
        let prompt_viewport = render_prompt_value_lines(prompt_host, self.theme, prompt_rows);

        let queue_line = if has_queued_commands {
            let queue_detail = format!("目录 {}", prompt_host.path_label);
            Some(self.render_pills_line(&prompt_host.queued_commands, Some(queue_detail.as_str())))
        } else {
            None
        };

        let cursor = prompt_has_focus.then_some(TuiCursorPlacement {
            x: inner.x.saturating_add(prompt_viewport.cursor.x),
            y: inner.y.saturating_add(prompt_viewport.cursor.y),
        });

        frame.render_widget(
            Paragraph::new(Text::from(
                prompt_viewport
                    .lines
                    .into_iter()
                    .chain(suggestion_lines)
                    .chain(suggestion_detail_line)
                    .chain(queue_line)
                    .chain([self.render_pills_line(&prompt_host.footer_pills, None)])
                    .collect::<Vec<_>>(),
            ))
            .style(self.theme.panel_style(TuiPanelSurface::Base)),
            // prompt host 的光标与 reserved_rows 都按“逻辑行 = 屏幕行”计算，
            // 这里不能再启用软换行，否则 helper/suggestion/footer 会偷偷多占行。
            inner,
        );

        cursor
    }

    fn render_prompt_suggestion_rows(&self, rows: &[TuiPromptSuggestionRow]) -> Vec<Line<'static>> {
        rows.iter()
            .map(|row| {
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

    fn render_project_context_panel(
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

    fn render_modified_files_panel(
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

    fn render_modal_host(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        modal_host: &TuiModalHost,
    ) {
        let block = self.panel_block(
            modal_host.title.clone(),
            TuiPanelSurface::Raised,
            TuiPanelBorder::Focused,
        );
        let inner = block.inner(area);

        let mut lines = vec![self.render_pills_line(&modal_host.chips, None), Line::raw("")];
        lines.extend(modal_host.body_lines.iter().cloned().map(Line::from));

        frame.render_widget(Clear, area);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .style(self.theme.panel_style(TuiPanelSurface::Raised))
                .wrap(Wrap { trim: false }),
            inner,
        );
    }

    fn pill_span(&self, pill: &TuiPill) -> Span<'static> {
        Span::styled(format!(" {} ", pill.label), self.pill_style(pill.tone))
    }

    fn pill_style(&self, tone: TuiTone) -> Style {
        let background = tone_color(tone, self.theme);
        let foreground =
            if matches!(tone, TuiTone::Muted) { self.theme.text } else { self.theme.background };
        Style::default().fg(foreground).bg(background).add_modifier(Modifier::BOLD)
    }

    fn render_pills_line(&self, pills: &[TuiPill], detail: Option<&str>) -> Line<'static> {
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
    if status.step_count > 0 {
        pills.push(TuiPill::new(format!("步骤 {}", status.step_count), TuiTone::Accent));
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

    TuiStatusFooter { pills, detail: Some(prompt_shortcut_hint(state)) }
}

pub(crate) fn build_prompt_host(state: &TuiState) -> TuiPromptHost {
    const MAX_VISIBLE_SUGGESTION_ROWS: usize = 3;

    let suggestions = prompt_suggestions(state);
    let suggestion_total = suggestions.len();
    let selected_index = selected_suggestion_index(state, &suggestions);
    let (visible_start, visible_end) = visible_prompt_suggestion_window(
        suggestion_total,
        selected_index.unwrap_or_default(),
        MAX_VISIBLE_SUGGESTION_ROWS,
    );
    let visible_suggestions = &suggestions[visible_start..visible_end];
    let suggestion_pills = suggestions
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
    let suggestion_rows = visible_suggestions
        .iter()
        .enumerate()
        .map(|(offset, suggestion)| TuiPromptSuggestionRow {
            label: suggestion.label.clone(),
            detail: suggestion.detail.clone(),
            selected: Some(visible_start + offset) == selected_index,
        })
        .collect::<Vec<_>>();
    let suggestion_detail = (suggestion_total == 0)
        .then(|| {
            matches!(state.prompt.mode, PromptMode::SlashCommand)
                .then(|| "未找到匹配命令，继续输入或退格调整。".to_string())
        })
        .flatten();

    let mut queued_commands =
        state.prompt.queued_commands.iter().take(3).map(queue_command_pill).collect::<Vec<_>>();
    let remaining = state.prompt.queued_commands.len().saturating_sub(queued_commands.len());
    if remaining > 0 {
        queued_commands.push(TuiPill::new(format!("+{remaining} more"), TuiTone::Muted));
    }

    TuiPromptHost {
        value: state.prompt.value.clone(),
        placeholder: if matches!(state.prompt.mode, PromptMode::SlashCommand) {
            "输入 / 命令…".to_string()
        } else {
            "输入消息…".to_string()
        },
        helper_text: if state.prompt.is_busy() {
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
        } else {
            "Enter 发送，Shift+Enter 换行，Tab 接受建议。".to_string()
        },
        path_label: compact_path_label(state.project.workspace_root.as_deref()),
        suggestions: suggestion_pills,
        suggestion_rows,
        suggestion_detail,
        queued_commands,
        footer_pills: vec![TuiPill::new(
            format!(
                "模型 {}",
                truncate_label(state.status.model_name.as_deref().unwrap_or("-"), 28)
            ),
            TuiTone::Accent,
        )],
        cursor_char_index: state.prompt.cursor.char_index,
    }
}

fn prompt_shortcut_hint(state: &TuiState) -> String {
    if state.prompt.is_busy() {
        return "当前轮次仍在输出，新输入会暂存到队列".to_string();
    }

    if matches!(state.prompt.mode, PromptMode::SlashCommand) {
        return "Up/Down 切换 · Tab/Enter 接受 · Enter 执行".to_string();
    }

    "Enter 发送 · Shift+Enter 换行 · Tab 接受建议".to_string()
}

fn visible_prompt_suggestion_window(
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct RenderedPromptViewport {
    lines: Vec<Line<'static>>,
    cursor: TuiCursorPlacement,
    hidden_above: usize,
}

fn render_prompt_value_lines(
    prompt_host: &TuiPromptHost,
    theme: TuiTheme,
    viewport_rows: usize,
) -> RenderedPromptViewport {
    let viewport_rows = viewport_rows.max(1);
    let (cursor_line, cursor_column) =
        prompt_cursor_line_column(prompt_host.value.as_str(), prompt_host.cursor_char_index);

    let raw_lines = if prompt_host.value.is_empty() {
        vec![String::new()]
    } else {
        prompt_host.value.split('\n').map(ToOwned::to_owned).collect::<Vec<_>>()
    };
    let total_lines = raw_lines.len().max(1);
    let visible_start = cursor_line.saturating_add(1).saturating_sub(viewport_rows);
    let visible_end = visible_start.saturating_add(viewport_rows).min(total_lines);
    let top_padding = viewport_rows.saturating_sub(visible_end.saturating_sub(visible_start));

    let mut lines = vec![Line::raw(""); top_padding];
    for index in visible_start..visible_end {
        let prefix = prompt_line_prefix(index);
        let text = raw_lines.get(index).map(String::as_str).unwrap_or_default();
        let text_span = if prompt_host.value.is_empty() {
            Span::styled(prompt_host.placeholder.clone(), Style::default().fg(theme.muted))
        } else {
            Span::styled(text.to_string(), Style::default().fg(theme.text))
        };
        lines.push(Line::from(vec![
            Span::styled(prefix, Style::default().fg(theme.accent)),
            text_span,
        ]));
    }

    let current_line = raw_lines
        .get(cursor_line.min(total_lines.saturating_sub(1)))
        .map(String::as_str)
        .unwrap_or_default();
    let current_prefix = prompt_line_prefix(cursor_line);
    let cursor_prefix_width = UnicodeWidthStr::width(current_prefix);
    let cursor_text = current_line.chars().take(cursor_column).collect::<String>();
    let cursor_x = cursor_prefix_width.saturating_add(UnicodeWidthStr::width(cursor_text.as_str()));
    let cursor_y = top_padding.saturating_add(cursor_line.saturating_sub(visible_start));

    RenderedPromptViewport {
        lines,
        cursor: TuiCursorPlacement { x: usize_to_u16(cursor_x), y: usize_to_u16(cursor_y) },
        hidden_above: visible_start,
    }
}

pub(crate) fn build_modal_host(state: &TuiState) -> Option<TuiModalHost> {
    let overlay = state.overlays.active()?;
    let (title, body_lines) = overlay_body_lines(overlay);

    Some(TuiModalHost {
        title,
        chips: vec![TuiPill::new(
            format!("类型 {}", overlay_kind_label(overlay.kind())),
            TuiTone::Accent,
        )],
        body_lines,
    })
}

fn overlay_body_lines(overlay: &UiOverlay) -> (String, Vec<String>) {
    match overlay {
        UiOverlay::Confirm(overlay) => (
            if overlay.title.trim().is_empty() {
                "确认".to_string()
            } else {
                overlay.title.clone()
            },
            vec![
                overlay.body.clone(),
                String::new(),
                format!(
                    "Enter 确认 {}  Esc {}  危险操作={}",
                    overlay.confirm_label, overlay.cancel_label, overlay.destructive
                ),
            ],
        ),
        UiOverlay::Search(overlay) => ("搜索".to_string(), search_overlay_body_lines(overlay)),
        UiOverlay::Question(overlay) => {
            (overlay.modal_title().to_string(), question_overlay_body_lines(overlay))
        }
        UiOverlay::Todo(overlay) => ("待办".to_string(), todo_overlay_body_lines(overlay)),
        UiOverlay::Task(overlay) => ("任务面板".to_string(), task_overlay_body_lines(overlay)),
        UiOverlay::CommandPalette(overlay) => (
            "命令面板".to_string(),
            vec![
                format!("查询: {}", overlay.query),
                format!("条目: {}", overlay.items.len()),
                format!(
                    "选中: {}",
                    overlay
                        .selected_index
                        .map(|index| index.to_string())
                        .unwrap_or_else(|| "-".to_string())
                ),
            ],
        ),
        UiOverlay::Error(overlay) => (
            overlay.title.clone(),
            vec![
                overlay.message.clone(),
                String::new(),
                if overlay.recoverable {
                    "Esc 关闭弹层并返回输入区。".to_string()
                } else {
                    "此弹层需要显式恢复后才能继续。".to_string()
                },
            ],
        ),
        UiOverlay::Mcp(overlay) => ("MCP".to_string(), {
            let mut lines = vec![
                format!("配置来源: {}", overlay.config_source),
                format!("服务器数: {}", overlay.servers.len()),
            ];
            if overlay.servers.is_empty() {
                lines.push(String::new());
                lines.push("当前没有可展示的 MCP 服务器。".to_string());
            } else {
                lines.push(String::new());
                lines.extend(overlay.servers.iter().take(8).map(|server| {
                    format!("{} [{}] {}", server.name, server.transport.label(), server.address)
                }));
            }
            lines
        }),
        UiOverlay::Memory(overlay) => ("记忆".to_string(), {
            let mut lines = vec![format!("条目数: {}", overlay.entries.len())];
            if overlay.entries.is_empty() {
                lines.push(String::new());
                lines.push("当前没有可展示的记忆文件。".to_string());
            } else {
                lines.push(String::new());
                lines.extend(
                    overlay
                        .entries
                        .iter()
                        .take(8)
                        .map(|entry| format!("{} [{}]", entry.filename, entry.scope)),
                );
            }
            lines
        }),
    }
}

fn search_overlay_body_lines(overlay: &super::model::UiSearchOverlay) -> Vec<String> {
    let mut lines = vec![
        format!(
            "查询: {}",
            if overlay.query.trim().is_empty() {
                "<空>".to_string()
            } else {
                overlay.query.clone()
            }
        ),
        format!("大小写: {}", if overlay.case_sensitive { "区分" } else { "不区分" }),
        format!("匹配数: {}", overlay.matches.len()),
        String::new(),
    ];

    if overlay.query.trim().is_empty() {
        lines.push("开始输入后会在当前会话中搜索。".to_string());
        lines.push("Enter 跳到当前结果，End 跳到末尾，u 跳到第一条未读。".to_string());
        lines.push("Tab / Shift+Tab 切换结果，Ctrl+S 切换大小写。".to_string());
        return lines;
    }

    let Some(selected_index) = overlay.selected_index else {
        lines.push("当前查询没有匹配结果。".to_string());
        lines.push("继续输入以缩小范围，或用 End / u 快速跳转。".to_string());
        return lines;
    };

    if let Some(selected) = overlay.matches.get(selected_index) {
        lines.push(format!("选中: {}/{}", selected_index.saturating_add(1), overlay.matches.len()));
        lines.push(selected.preview.clone());
    }
    lines.push(String::new());
    lines.push("Enter 跳到当前结果并关闭搜索弹层。".to_string());
    lines.push("End 跳到会话末尾，u 跳到第一条未读锚点。".to_string());
    lines
}

fn question_overlay_body_lines(overlay: &super::model::UiQuestionOverlay) -> Vec<String> {
    let mut lines = vec![
        format!("请求: {}", overlay.request_id),
        format!("会话: {}", overlay.session_id),
        format!(
            "题目: {}/{}",
            overlay.selected_index.saturating_add(1),
            overlay.prompts.len().max(1)
        ),
    ];

    match overlay.surface_kind() {
        UiQuestionSurfaceKind::Question => {}
        UiQuestionSurfaceKind::ToolFallback => {
            if let Some(tool) = overlay.tool.as_ref() {
                lines.push(
                    "该提问来自工具回退通道，你可以直接在这里回答，无需离开 TUI。".to_string(),
                );
                lines.push(format!("工具调用: {}", tool.call_id));
                lines.push(format!("来源消息: {}", tool.message_id));
                lines.push(String::new());
            }
        }
        UiQuestionSurfaceKind::PermissionRequest => {
            if let Some(tool) = overlay.tool.as_ref() {
                lines.push(
                    "该工具调用正在等待你的授权。选择允许项后可继续，或按 Ctrl+R 明确拒绝。"
                        .to_string(),
                );
                lines.push(format!("工具调用: {}", tool.call_id));
                lines.push(format!("来源消息: {}", tool.message_id));
                lines.push(String::new());
            }
        }
    }

    let Some(prompt) = overlay.prompts.get(overlay.selected_index) else {
        lines.push("当前问题内容为空。".to_string());
        return lines;
    };

    if !prompt.header.trim().is_empty() {
        lines.push(format!("标题: {}", prompt.header));
    }
    lines.push(prompt.question.clone());

    let answers = overlay.answers.get(overlay.selected_index).cloned().unwrap_or_default();
    let selected_answers = answers
        .iter()
        .map(|answer| answer.strip_prefix("__custom__:").unwrap_or(answer.as_str()).to_string())
        .collect::<Vec<_>>();

    if !prompt.options.is_empty() {
        lines.push(String::new());
        lines.push("选项:".to_string());
        for (index, option) in prompt.options.iter().take(9).enumerate() {
            let selected = selected_answers.iter().any(|answer| answer == &option.label);
            let description = if option.description.trim().is_empty() {
                String::new()
            } else {
                format!(" - {}", option.description)
            };
            lines.push(format!(
                "  {}. [{}] {}{}",
                index + 1,
                if selected { "x" } else { " " },
                option.label,
                description
            ));
        }
    }

    lines.push(String::new());
    lines.push(format!(
        "当前回答: {}",
        if selected_answers.is_empty() { "-".to_string() } else { selected_answers.join(", ") }
    ));
    if !prompt.multiple
        && selected_answers.len() == 1
        && let Some(preview) = prompt
            .options
            .iter()
            .find(|option| option.label == selected_answers[0])
            .and_then(|option| option.preview.as_deref())
    {
        lines.push(String::new());
        lines.push("预览:".to_string());
        for line in preview.lines() {
            lines.push(format!("  {}", line));
        }
    }
    lines.push(match overlay.surface_kind() {
        UiQuestionSurfaceKind::PermissionRequest => {
            "按键: 1-9 选择授权  Tab/Shift+Tab 切题  Enter 提交  Ctrl+R 拒绝  Esc 关闭".to_string()
        }
        _ => "按键: 1-9 选择  Tab/Shift+Tab 切题  Enter 提交  Ctrl+R 拒绝  Esc 关闭".to_string(),
    });
    lines
}

fn todo_overlay_body_lines(overlay: &super::model::UiTodoOverlay) -> Vec<String> {
    let completed_count =
        overlay.items.iter().filter(|item| item.status.eq_ignore_ascii_case("completed")).count();
    let pending_count = overlay.items.len().saturating_sub(completed_count);
    let mut lines = vec![
        format!("会话: {}", overlay.session_id.as_deref().unwrap_or("-")),
        format!("条目: {}", overlay.items.len()),
        format!("脏标记: {}", overlay.dirty),
        format!("状态汇总: 待处理={} 已完成={}", pending_count, completed_count),
    ];

    if overlay.items.is_empty() {
        lines.push(String::new());
        lines.push("当前会话还没有待办项。".to_string());
        lines.push("按 r 刷新，或按 Esc 关闭面板。".to_string());
        return lines;
    }

    for (index, item) in overlay.items.iter().take(8).enumerate() {
        let marker = if index == overlay.selected_index { ">" } else { " " };
        lines.push(format!(
            "{} [{}] {} ({})",
            marker,
            if item.status.eq_ignore_ascii_case("completed") { "x" } else { " " },
            truncate_label(item.content.as_str(), 44),
            item.priority
        ));
    }

    if let Some(item) = overlay.items.get(overlay.selected_index) {
        lines.push(String::new());
        lines.push(format!(
            "当前待办: {}/{}",
            overlay.selected_index.saturating_add(1),
            overlay.items.len()
        ));
        lines.push(format!("ID: {}", item.id));
        lines.push(format!("状态: {}", item.status));
        lines.push(format!("优先级: {}", item.priority));
        lines.push("内容:".to_string());
        for line in item.content.lines() {
            lines.push(format!("  {}", line));
        }
        if item.content.trim().is_empty() {
            lines.push("  -".to_string());
        }
    }

    lines.push(String::new());
    lines.push("按键: Up/Down 移动  Space 切换完成  s 保存  r 刷新  Esc 关闭".to_string());
    lines
}

fn task_overlay_body_lines(overlay: &super::model::UiTaskOverlay) -> Vec<String> {
    let mut lines = vec![
        format!("会话: {}", overlay.session_id.as_deref().unwrap_or("-")),
        format!("终端: {}", terminal_label(&overlay.turn_terminal)),
        format!("待处理问题: {}", overlay.pending_questions),
        format!("待办: {}", overlay.todo_count),
    ];

    if let Some(sync_error) = overlay.sync_error.as_ref() {
        lines.push(format!("任务同步错误: {sync_error}"));
    }

    lines.push(String::new());

    if overlay.steps.is_empty() {
        lines.push("当前会话还没有步骤活动。".to_string());
        lines.push("该面板仍会持续显示问题、待办和任务同步状态。".to_string());
        lines.push("Esc 关闭面板。".to_string());
        return lines;
    }

    lines.push(format!("步骤数: {}", overlay.steps.len()));
    for (index, step) in overlay.steps.iter().enumerate() {
        let marker = if index == overlay.selected_index { ">" } else { " " };
        let finish_reason = step.finish_reason.as_deref().unwrap_or("-");
        lines.push(format!(
            "{marker} 步骤 {}  {}  结束={}",
            step.step_index,
            step_state_label(&step.state),
            finish_reason
        ));
    }

    if let Some(step) = overlay.steps.get(overlay.selected_index) {
        lines.push(String::new());
        lines.push(format!("当前步骤: {}", step.step_index));
        lines.push(format!("状态: {}", step_state_label(&step.state)));
        lines.push(step_timing_line(step));
        lines.push(format!(
            "令牌: 输入={} 输出={} 缓存={} 推理={}",
            step.usage.input_tokens,
            step.usage.output_tokens,
            step.usage.cached_tokens,
            step.usage.reasoning_tokens
        ));
        lines.push(format!("结束原因: {}", step.finish_reason.as_deref().unwrap_or("-")));
    }

    lines.push(String::new());
    lines.push("Up/Down 切换步骤，Enter 跳到选中步骤。".to_string());
    lines.push("Esc 关闭面板并返回输入区。".to_string());
    lines
}

fn sidebar_lines(
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

fn clip_sidebar_body_lines(lines: &[String], max_lines: usize) -> Vec<String> {
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

fn step_timing_line(step: &super::model::UiTaskStepItem) -> String {
    match step.finished_ms {
        Some(finished_ms) => format!(
            "耗时: 开始={} 结束={} 总计={}ms",
            step.started_ms,
            finished_ms,
            finished_ms.saturating_sub(step.started_ms)
        ),
        None => format!("耗时: 开始={} 结束=进行中", step.started_ms),
    }
}

fn terminal_label(terminal: &UiTurnTerminal) -> &'static str {
    match terminal {
        UiTurnTerminal::Pending => "待命",
        UiTurnTerminal::Streaming => "输出中",
        UiTurnTerminal::Done { .. } => "完成",
        UiTurnTerminal::Cancelled { .. } => "已取消",
        UiTurnTerminal::TimedOut { .. } => "超时",
        UiTurnTerminal::Error { .. } => "错误",
    }
}

fn terminal_tone(terminal: &UiTurnTerminal) -> TuiTone {
    match terminal {
        UiTurnTerminal::Pending => TuiTone::Muted,
        UiTurnTerminal::Streaming => TuiTone::Accent,
        UiTurnTerminal::Done { .. } => TuiTone::Success,
        UiTurnTerminal::Cancelled { .. } => TuiTone::Warning,
        UiTurnTerminal::TimedOut { .. } | UiTurnTerminal::Error { .. } => TuiTone::Danger,
    }
}

fn step_state_label(state: &UiStepState) -> &'static str {
    match state {
        UiStepState::Pending => "等待中",
        UiStepState::Running => "进行中",
        UiStepState::Complete => "完成",
        UiStepState::Cancelled => "已取消",
        UiStepState::Failed => "失败",
    }
}

fn tone_color(tone: TuiTone, theme: TuiTheme) -> Color {
    match tone {
        TuiTone::Accent => theme.accent,
        TuiTone::Muted => theme.border,
        TuiTone::Success => theme.success,
        TuiTone::Warning => theme.warning,
        TuiTone::Danger => theme.danger,
    }
}

fn queue_command_pill(command: &QueuedPromptCommand) -> TuiPill {
    let kind = match command.kind {
        QueuedPromptCommandKind::Submit => "发送",
        QueuedPromptCommandKind::SlashCommand => "命令",
    };
    TuiPill::new(truncate_label(format!("{kind} {}", command.raw), 24), TuiTone::Warning)
}

fn last_submission_pill(state: &TuiState) -> TuiPill {
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

fn prompt_mode_label(mode: &PromptMode) -> &'static str {
    match mode {
        PromptMode::Compose => "对话",
        PromptMode::SlashCommand => "命令",
        PromptMode::Search => "搜索",
        PromptMode::QuestionReply => "问答",
        PromptMode::TodoEdit => "待办",
        PromptMode::CommandPalette => "面板",
        PromptMode::Busy => "忙碌",
    }
}

fn overlay_kind_label(kind: UiOverlayKind) -> &'static str {
    match kind {
        UiOverlayKind::Confirm => "确认",
        UiOverlayKind::Search => "搜索",
        UiOverlayKind::Question => "提问",
        UiOverlayKind::Todo => "待办",
        UiOverlayKind::Task => "任务",
        UiOverlayKind::CommandPalette => "命令面板",
        UiOverlayKind::Error => "错误",
        UiOverlayKind::Mcp => "MCP",
        UiOverlayKind::Memory => "记忆",
    }
}

fn prompt_line_prefix(line_index: usize) -> &'static str {
    if line_index == 0 { "> " } else { "· " }
}

fn prompt_cursor_line_column(value: &str, char_index: usize) -> (usize, usize) {
    let mut current_line = 0usize;
    let mut current_column = 0usize;

    for (index, ch) in value.chars().enumerate() {
        if index >= char_index {
            break;
        }

        if ch == '\n' {
            current_line = current_line.saturating_add(1);
            current_column = 0;
        } else {
            current_column = current_column.saturating_add(1);
        }
    }

    (current_line, current_column)
}

fn zh_window_sticky_label(window_summary: super::state::TuiWindowSummary) -> String {
    match window_summary.sticky_message {
        Some(anchor) => format!("停在 m{anchor}"),
        None if window_summary.follow_tail => "末尾".to_string(),
        None => "浏览历史".to_string(),
    }
}

fn zh_sticky_notice(notice: String) -> String {
    notice
        .strip_prefix("message ")
        .and_then(|rest| rest.split_once(' '))
        .map(|(message, _)| format!("消息 {message} 固定在当前视口上方。"))
        .unwrap_or_else(|| "上方还有一段已固定的上下文。".to_string())
}

fn zh_unseen_pill_label(unseen_range: super::state::TuiUnseenRangeSummary) -> String {
    if unseen_range.unseen_message_count == 1 {
        "1 条新消息".to_string()
    } else {
        format!("{} 条新消息", unseen_range.unseen_message_count)
    }
}

fn zh_unseen_divider_label(unseen_range: super::state::TuiUnseenRangeSummary) -> String {
    format!("下方还有 {}", zh_unseen_pill_label(unseen_range))
}

fn usize_to_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn compact_path_label(path: Option<&Path>) -> String {
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

fn truncate_label(value: impl AsRef<str>, max_chars: usize) -> String {
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

fn truncate_start(value: impl AsRef<str>, max_chars: usize) -> String {
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
