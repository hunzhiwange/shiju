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
pub(crate) mod markdown;
#[cfg(test)]
#[path = "markdown_tests.rs"]
mod markdown_tests;
mod message_row;
#[cfg(test)]
#[path = "message_row_tests.rs"]
mod message_row_tests;

mod attachment;
#[cfg(test)]
#[path = "attachment_tests.rs"]
mod attachment_tests;
mod chrome;
mod diff;
#[cfg(test)]
#[path = "diff_tests.rs"]
mod diff_tests;
mod overlay;
mod prompt;
mod shell;
#[cfg(test)]
#[path = "shell_tests.rs"]
mod shell_tests;
mod theme;
#[cfg(test)]
#[path = "theme_tests.rs"]
mod theme_tests;
mod tool_row;
#[cfg(test)]
#[path = "tool_row_tests.rs"]
mod tool_row_tests;
mod transcript;
mod welcome;

use chrome::{build_activity_indicator, step_state_label, terminal_label};
use prompt::{compact_path_label, truncate_label};
use theme::TuiThemeName;
use theme::{TuiPanelBorder, TuiPanelSurface, TuiPill, TuiTheme, TuiTone};
use transcript::zh_unseen_pill_label;
use welcome::should_show_welcome;

#[cfg(test)]
use chrome::{clip_sidebar_body_lines, sidebar_lines, terminal_tone, tone_color};
#[cfg(test)]
use overlay::{overlay_kind_label, step_timing_line};
#[cfg(test)]
use prompt::{
    TuiPromptHost, last_submission_pill, prompt_cursor_line_column, prompt_line_prefix,
    prompt_mode_label, prompt_shortcut_hint, queue_command_pill, render_prompt_value_lines,
    truncate_start, visible_prompt_suggestion_window,
};
#[cfg(test)]
use transcript::{zh_sticky_notice, zh_unseen_divider_label, zh_window_sticky_label};

pub(crate) use chrome::build_status_footer;
#[cfg(test)]
pub(crate) use chrome::{
    build_modified_files_host, build_project_context_host, build_status_header,
};
pub(crate) use overlay::build_modal_host;
pub(crate) use prompt::build_prompt_host;

use std::path::{Path, PathBuf};

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span, Text};
use ratatui::widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap};
use unicode_width::UnicodeWidthStr;

use super::input::{prompt_suggestions, selected_suggestion_index};
use super::model::{
    OverlayFocus, PromptMode, PromptSubmissionStatus, QueuedPromptCommand, QueuedPromptCommandKind,
    QueuedPromptPriority, UiOverlay, UiOverlayKind, UiQuestionSurfaceKind, UiStepState,
    UiTurnTerminal,
};
use super::state::{
    TuiState, TuiStatusSummary, TuiTranscriptMeasurements, TuiVisibleTranscriptWindow,
    select_status_summary, select_visible_grouped_transcript_window,
};
use layout::FullscreenLayoutSlots;

const SPINNER_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];

#[cfg(test)]
mod parity_tests;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct TuiCursorPlacement {
    pub(crate) x: u16,
    pub(crate) y: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TuiScrollbarView {
    Main,
    Transcript,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct TuiScrollbarFeedback {
    pub(crate) view: TuiScrollbarView,
    pub(crate) track: Rect,
    pub(crate) thumb: Rect,
    pub(crate) content_length: usize,
    pub(crate) viewport_length: usize,
    pub(crate) position: usize,
}

impl TuiScrollbarFeedback {
    fn new(
        view: TuiScrollbarView,
        track: Rect,
        metrics: transcript::TuiScrollbarMetrics,
        content_length: usize,
        viewport_length: usize,
        position: usize,
    ) -> Self {
        let thumb_y =
            track.y.saturating_add(u16::try_from(metrics.thumb_start).unwrap_or(u16::MAX));
        let thumb_height = u16::try_from(metrics.thumb_height).unwrap_or(track.height);
        Self {
            view,
            track,
            thumb: Rect::new(track.x, thumb_y, track.width, thumb_height),
            content_length,
            viewport_length,
            position: position.min(content_length.saturating_sub(viewport_length)),
        }
    }
}

/// fullscreen skeleton 的 render 输出反馈。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub(crate) struct TuiRenderFeedback {
    pub(crate) layout: FullscreenLayoutSlots,
    pub(crate) cursor: Option<TuiCursorPlacement>,
    pub(crate) transcript_measurements: TuiTranscriptMeasurements,
    pub(crate) scrollbar: Option<TuiScrollbarFeedback>,
}

/// tui_v2 fullscreen skeleton renderer。
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct TuiRenderer {
    theme: TuiTheme,
    capabilities: crate::cli::tui_v2::terminal_capabilities::TerminalCapabilities,
}

impl TuiRenderer {
    pub(crate) fn for_capabilities(
        capabilities: crate::cli::tui_v2::terminal_capabilities::TerminalCapabilities,
        daltonized: bool,
    ) -> Self {
        Self { theme: TuiTheme::from_capabilities(capabilities, daltonized), capabilities }
    }

    #[cfg(test)]
    pub(crate) fn for_theme(
        name: TuiThemeName,
        capabilities: crate::cli::tui_v2::terminal_capabilities::TerminalCapabilities,
    ) -> Self {
        Self { theme: TuiTheme::for_name(name, capabilities), capabilities }
    }

    pub(crate) fn set_theme_name(&mut self, name: &str) -> Result<(), String> {
        let name = match name.trim().to_ascii_lowercase().as_str() {
            "dark" => TuiThemeName::Dark,
            "light" => TuiThemeName::Light,
            "darkdaltonized" | "dark-daltonized" => TuiThemeName::DarkDaltonized,
            "lightdaltonized" | "light-daltonized" => TuiThemeName::LightDaltonized,
            "darkansi" | "dark-ansi" => TuiThemeName::DarkAnsi,
            "lightansi" | "light-ansi" => TuiThemeName::LightAnsi,
            _ => return Err(format!("unsupported TUI theme: {name}")),
        };
        self.theme = TuiTheme::for_name(name, self.capabilities);
        Ok(())
    }

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
        let footer = build_status_footer(state, &status, &visible_window);
        let activity = build_activity_indicator(state, spinner_frame);
        let mut prompt_host = build_prompt_host(state);
        prompt_host.footer_pills.extend(footer.pills);
        if let Some(detail) = footer.detail {
            prompt_host.helper_text.push_str(" · ");
            prompt_host.helper_text.push_str(&detail);
        }
        let modal_host = build_modal_host(state);
        let layout = layout::compute_fullscreen_layout(
            area,
            true,
            modal_host.is_some(),
            prompt_host.extra_prompt_rows(),
        );
        let prompt_has_focus =
            matches!(state.overlays.focus, OverlayFocus::Prompt) && !state.prompt.is_busy();

        if state.transcript_mode.active {
            let scrollbar = self.render_transcript_mode(frame, area, state);
            return TuiRenderFeedback {
                layout,
                cursor: None,
                transcript_measurements: TuiTranscriptMeasurements::default(),
                scrollbar,
            };
        }

        let (transcript_measurements, mut scrollbar) = if should_show_welcome(state) {
            self.render_welcome(frame, layout.scrollable, state, badge_label, endpoint_label);
            (TuiTranscriptMeasurements::default(), None)
        } else {
            self.render_scrollable(
                frame,
                layout.scrollable_body,
                layout.scrollbar_track,
                &visible_window,
                state.status.verbose,
                activity.as_ref(),
                state.project.workspace_root.as_deref(),
                layout.scrollable_body.width,
                state.message_actions.selected_id.as_ref(),
            )
        };
        if modal_host.is_some() {
            scrollbar = None;
        }

        let cursor = self.render_prompt_host(frame, layout.bottom, &prompt_host, prompt_has_focus);
        if let Some(cursor) = cursor {
            frame.set_cursor_position((cursor.x, cursor.y));
        }

        if let (Some(modal_area), Some(modal_host)) = (layout.modal, modal_host.as_ref()) {
            self.render_modal_host(frame, modal_area, modal_host);
        }

        TuiRenderFeedback { layout, cursor, transcript_measurements, scrollbar }
    }
}

fn usize_to_u16(value: usize) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}
