//! 聊天输入栏的会话级推理强度选择器。

use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_settings_providers::state::ProviderModelsSummary;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Border, Element, Length, Theme};
use vw_api_chat_identity::ReasoningEffortDto;

use super::icons::icon_svg;
use super::reasoning_message::{Message, dark_tooltip_content};
use super::styles::{selector_label_font, square_icon_button_style};
use desktop_ui_controls::system_settings_common::settings_muted_text_style;

#[path = "reasoning_slider.rs"]
mod reasoning_slider;

const SLIDER_HEIGHT: f32 = 28.0;
const HANDLE_RADIUS: f32 = 13.0;

const CODEX_OPTIONS: [Option<ReasoningEffortDto>; 6] = [
    None,
    Some(ReasoningEffortDto::None),
    Some(ReasoningEffortDto::Low),
    Some(ReasoningEffortDto::Medium),
    Some(ReasoningEffortDto::High),
    Some(ReasoningEffortDto::Xhigh),
];
const CLAUDE_OPTIONS: [Option<ReasoningEffortDto>; 3] =
    [None, Some(ReasoningEffortDto::High), Some(ReasoningEffortDto::Max)];
const DEFAULT_OPTIONS: [Option<ReasoningEffortDto>; 1] = [None];

/// UI and request validation share provider metadata and legacy model inference.
pub fn model_reasoning_effort_options(
    providers: &[ProviderModelsSummary],
    model: &str,
) -> Vec<Option<ReasoningEffortDto>> {
    let (provider_id, model_id) =
        model.split_once('/').map_or((None, model), |(p, m)| (Some(p), m));
    let mut options = vec![None];
    let candidate = providers
        .iter()
        .filter(|p| provider_id.is_none_or(|id| p.id == id))
        .find_map(|p| p.models.iter().find(|m| m.id == model_id && m.enabled).map(|m| (p, m)));
    if let Some((provider, model)) = candidate {
        let detail = &model.detail;
        if detail.pointer("/capabilities/reasoning").and_then(serde_json::Value::as_bool)
            != Some(true)
        {
            return options;
        }
        let declared: Option<Vec<model_reasoning::ReasoningOption>> = detail
            .pointer("/capabilities/reasoning_options")
            .map(|value| serde_json::from_value(value.clone()).unwrap_or_default());
        let levels = model_reasoning::reasoning_levels(
            detail.pointer("/api/id").and_then(serde_json::Value::as_str).unwrap_or(&model.id),
            detail
                .pointer("/api/adapter")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("openai-compatible"),
            &provider.id,
            detail.pointer("/api/url").and_then(serde_json::Value::as_str).unwrap_or_default(),
            declared.as_deref(),
        );
        options.extend(
            levels
                .iter()
                .filter_map(|level| serde_json::from_value(serde_json::json!(level)).ok())
                .map(Some),
        );
    }
    options
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReasoningEffortItem {
    pub(crate) value: Option<ReasoningEffortDto>,
    pub(crate) selected: bool,
}

pub fn reasoning_effort_label(value: Option<ReasoningEffortDto>) -> &'static str {
    match value {
        None => "默认",
        Some(ReasoningEffortDto::None) => "None",
        Some(ReasoningEffortDto::Minimal) => "Minimal",
        Some(ReasoningEffortDto::Low) => "Low",
        Some(ReasoningEffortDto::Medium) => "Medium",
        Some(ReasoningEffortDto::High) => "High",
        Some(ReasoningEffortDto::Xhigh) => "Xhigh",
        Some(ReasoningEffortDto::Max) => "Max",
    }
}

pub fn localized_reasoning_effort_label(
    i18n: desktop_i18n::I18n,
    value: Option<ReasoningEffortDto>,
) -> &'static str {
    let source = match value {
        None => "默认",
        Some(ReasoningEffortDto::None) => "无推理",
        Some(ReasoningEffortDto::Minimal) => "最低",
        Some(ReasoningEffortDto::Low) => "低",
        Some(ReasoningEffortDto::Medium) => "中",
        Some(ReasoningEffortDto::High) => "高",
        Some(ReasoningEffortDto::Xhigh) => "极高",
        Some(ReasoningEffortDto::Max) => "最大",
    };
    i18n.__(source)
}

pub fn reasoning_effort_options(agent: &str) -> &'static [Option<ReasoningEffortDto>] {
    match agent.trim().to_ascii_lowercase().as_str() {
        "codex" | "codex cli" | "codex-cli" => &CODEX_OPTIONS,
        "claude" | "claude code" | "claude-code" | "claudecode" => &CLAUDE_OPTIONS,
        _ => &DEFAULT_OPTIONS,
    }
}

pub(crate) fn reasoning_effort_items(
    options: &[Option<ReasoningEffortDto>],
    current: Option<ReasoningEffortDto>,
) -> Vec<ReasoningEffortItem> {
    options
        .iter()
        .copied()
        .map(|value| ReasoningEffortItem { value, selected: value == current })
        .collect()
}

pub(crate) fn reasoning_effort_message(value: Option<ReasoningEffortDto>) -> Message {
    Message::Selected(value)
}

pub fn reasoning_effort_control(
    i18n: I18n,
    session_id: Option<&str>,
    model: &str,
    options: &[Option<ReasoningEffortDto>],
    current: Option<ReasoningEffortDto>,
) -> Element<'static, Message> {
    let items = reasoning_effort_items(options, current);
    let selected = items.iter().position(|item| item.selected).unwrap_or(0);
    let control = reasoning_slider::ReasoningSlider {
        selected,
        options: options.to_vec(),
        identity: format!("{session_id:?}:{model}:{options:?}"),
    };
    let reset = Tooltip::new(
        button(container(icon_svg(Icon::ArrowCounterClockwise, 14.0)).center(24.0))
            .padding(0)
            .style(move |theme, status| square_icon_button_style(theme, status, current.is_some()))
            .on_press_maybe(current.map(|_| reasoning_effort_message(None))),
        dark_tooltip_content(i18n.__("恢复默认")),
        TooltipPosition::Top,
    );
    let heading = row![
        text(i18n.__("推理强度")).size(12).style(settings_muted_text_style),
        Space::new().width(Length::Fill),
        text(localized_reasoning_effort_label(i18n, current)).size(13).font(selector_label_font()),
        reset,
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    container(column![heading, control].spacing(6))
        .padding([8, 10])
        .width(Length::Fill)
        .style(reasoning_control_surface)
        .into()
}

pub(super) fn reasoning_control_surface(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(theme.palette().text.scale_alpha(0.06).into()),
        border: Border::default().rounded(12.0),
        ..Default::default()
    }
}
