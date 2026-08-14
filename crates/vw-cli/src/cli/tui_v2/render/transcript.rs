use super::chrome::TuiActivityIndicator;
use super::{
    Line, Modifier, Paragraph, Rect, Span, Style, Text, TuiPanelSurface, TuiRenderer,
    TuiScrollbarFeedback, TuiScrollbarView, TuiVisibleTranscriptWindow, Wrap,
};
use crate::cli::tui_v2::model::UiMessage;
use crate::cli::tui_v2::model::UiMessageId;
use crate::cli::tui_v2::state::{TuiTranscriptMeasurement, TuiTranscriptMeasurements};
use std::path::Path;

impl TuiRenderer {
    pub(super) fn render_scrollable(
        &self,
        frame: &mut ratatui::Frame<'_>,
        area: Rect,
        scrollbar_track: Option<Rect>,
        visible_window: &TuiVisibleTranscriptWindow<'_>,
        verbose: bool,
        activity: Option<&TuiActivityIndicator>,
        workspace_root: Option<&Path>,
        measurement_width: u16,
        selected_message_id: Option<&UiMessageId>,
    ) -> (TuiTranscriptMeasurements, Option<TuiScrollbarFeedback>) {
        let window_summary = visible_window.window_summary();
        let mut measurements = Vec::new();
        let lines = if visible_window.is_empty() {
            vec![Line::from(vec![
                Span::styled("还没有消息。", self.theme.muted_style()),
                Span::raw(" 在下方输入内容并回车即可开始会话。"),
            ])]
        } else {
            let mut lines = Vec::new();
            if visible_window.top_spacer_rows > 0 {
                lines.push(Line::from(Span::styled(
                    format!("⋮ 上方 {} 行", visible_window.top_spacer_rows),
                    self.theme.muted_style(),
                )));
            }
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
                if selected_message_id.is_some_and(|id| transcript_item_contains_id(item, id)) {
                    lines.push(Line::from(Span::styled(
                        "▶ 已选择消息",
                        self.theme.accent_style().add_modifier(Modifier::BOLD),
                    )));
                }
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

                let workspace_root = workspace_root.map(|path| path.to_string_lossy().into_owned());
                let item_lines = super::message_row::render_transcript_item_lines_with_context(
                    item,
                    self.theme,
                    area.width,
                    verbose,
                    self.capabilities,
                    workspace_root.as_deref(),
                );
                if item_lines.is_empty() {
                    continue;
                }
                let next_is_user = visible_window
                    .visible_items()
                    .get(offset.saturating_add(1))
                    .is_some_and(is_user_item);
                let has_separator = !(is_user_item(item) && next_is_user);
                if let Some(key) = visible_window.visible_keys().get(offset) {
                    measurements.push(TuiTranscriptMeasurement {
                        key: key.clone(),
                        height: rendered_line_height(&item_lines, measurement_width)
                            .saturating_add(usize::from(has_separator)),
                    });
                }
                lines.extend(item_lines);
                if has_separator {
                    lines.push(Line::raw(""));
                }
            }
            if visible_window.bottom_spacer_rows > 0 {
                lines.push(Line::from(Span::styled(
                    format!("⋮ 下方 {} 行", visible_window.bottom_spacer_rows),
                    self.theme.muted_style(),
                )));
            }
            if let Some(activity) = activity {
                lines.push(Line::from(vec![
                    Span::styled(
                        activity.spinner.clone(),
                        self.theme.accent_style().add_modifier(Modifier::BOLD),
                    ),
                    Span::raw(" "),
                    Span::styled(activity.verb.clone(), self.theme.muted_style()),
                ]));
            }
            lines
        };

        frame.render_widget(
            Paragraph::new(Text::from(lines))
                .style(self.theme.panel_style(TuiPanelSurface::Raised))
                .wrap(Wrap { trim: false }),
            area,
        );
        let scrollbar = scrollbar_track.and_then(|track| {
            scrollbar_metrics(
                visible_window.total_items,
                visible_window.viewport_message_capacity,
                visible_window.top_item_index,
                track.height,
            )
            .map(|metrics| {
                self.render_scrollbar(frame, track, metrics);
                TuiScrollbarFeedback::new(
                    TuiScrollbarView::Main,
                    track,
                    metrics,
                    visible_window.total_items,
                    visible_window.viewport_message_capacity,
                    visible_window.top_item_index,
                )
            })
        });
        (
            TuiTranscriptMeasurements {
                content_width: measurement_width,
                verbose,
                items: measurements,
            },
            scrollbar,
        )
    }

    pub(super) fn render_scrollbar(
        &self,
        frame: &mut ratatui::Frame<'_>,
        track: Rect,
        metrics: TuiScrollbarMetrics,
    ) {
        if track.width == 0 || track.height == 0 {
            return;
        }
        let thumb_end = metrics.thumb_start.saturating_add(metrics.thumb_height);
        let lines = (0..usize::from(track.height))
            .map(|row| {
                if row >= metrics.thumb_start && row < thumb_end {
                    Line::from(Span::styled(
                        "█",
                        self.theme.accent_style().add_modifier(Modifier::BOLD),
                    ))
                } else {
                    Line::from(Span::styled("│", self.theme.muted_style()))
                }
            })
            .collect::<Vec<_>>();
        frame.render_widget(Paragraph::new(Text::from(lines)), track);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TuiScrollbarMetrics {
    pub(crate) thumb_start: usize,
    pub(crate) thumb_height: usize,
}

pub(crate) fn scrollbar_metrics(
    content_length: usize,
    viewport_length: usize,
    position: usize,
    track_height: u16,
) -> Option<TuiScrollbarMetrics> {
    let track_height = usize::from(track_height);
    if track_height < 2 || content_length <= viewport_length || viewport_length == 0 {
        return None;
    }

    let thumb_height = viewport_length
        .saturating_mul(track_height)
        .checked_div(content_length)
        .unwrap_or_default()
        .max(2)
        .min(track_height);
    let max_position = content_length.saturating_sub(viewport_length);
    let max_thumb_start = track_height.saturating_sub(thumb_height);
    let thumb_start = position
        .min(max_position)
        .saturating_mul(max_thumb_start)
        .checked_div(max_position)
        .unwrap_or_default();
    Some(TuiScrollbarMetrics { thumb_start, thumb_height })
}

fn transcript_item_contains_id(
    item: &super::super::state::TuiTranscriptItem<'_>,
    id: &UiMessageId,
) -> bool {
    match item {
        super::super::state::TuiTranscriptItem::Standalone(message) => &message.base().id == id,
        super::super::state::TuiTranscriptItem::AssistantTurn(turn) => {
            &turn.assistant.base.id == id
                || turn.preface.iter().chain(&turn.children).any(|entry| {
                    use super::super::state::TuiAssistantTurnEntry;
                    match entry {
                        TuiAssistantTurnEntry::Thinking(message) => &message.base.id == id,
                        TuiAssistantTurnEntry::Step(message) => &message.base.id == id,
                        TuiAssistantTurnEntry::Tool(group) => {
                            &group.call.base.id == id
                                || group.results.iter().any(|message| &message.base.id == id)
                        }
                        TuiAssistantTurnEntry::ToolResult(message) => &message.base.id == id,
                        TuiAssistantTurnEntry::CollapsedTools(batch) => {
                            batch.calls.iter().any(|group| {
                                &group.call.base.id == id
                                    || group.results.iter().any(|message| &message.base.id == id)
                            })
                        }
                    }
                })
        }
    }
}

fn rendered_line_height(lines: &[Line<'_>], content_width: u16) -> usize {
    let width = usize::from(content_width.max(1));
    lines.iter().map(|line| line.width().max(1).div_ceil(width)).sum::<usize>().max(1)
}

fn is_user_item(item: &super::super::state::TuiTranscriptItem<'_>) -> bool {
    matches!(item, super::super::state::TuiTranscriptItem::Standalone(UiMessage::User(_)))
}

pub(super) fn zh_window_sticky_label(
    window_summary: super::super::state::TuiWindowSummary,
) -> String {
    match window_summary.sticky_message {
        Some(anchor) => format!("停在 m{anchor}"),
        None if window_summary.follow_tail => "末尾".to_string(),
        None => "浏览历史".to_string(),
    }
}

pub(super) fn zh_sticky_notice(notice: String) -> String {
    notice
        .strip_prefix("message ")
        .and_then(|rest| rest.split_once(' '))
        .map(|(message, _)| format!("消息 {message} 固定在当前视口上方。"))
        .unwrap_or_else(|| "上方还有一段已固定的上下文。".to_string())
}

pub(super) fn zh_unseen_pill_label(
    unseen_range: super::super::state::TuiUnseenRangeSummary,
) -> String {
    if unseen_range.unseen_message_count == 1 {
        "1 条新消息".to_string()
    } else {
        format!("{} 条新消息", unseen_range.unseen_message_count)
    }
}

pub(super) fn zh_unseen_divider_label(
    unseen_range: super::super::state::TuiUnseenRangeSummary,
) -> String {
    format!("下方还有 {}", zh_unseen_pill_label(unseen_range))
}
