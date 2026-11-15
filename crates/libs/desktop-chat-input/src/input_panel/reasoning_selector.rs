//! 聊天输入栏的会话级推理强度选择器。

use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};
use vw_api_chat_identity::ReasoningEffortDto;


use super::icons::icon_svg;
use super::styles::{
    popover_style,
    selector_chevron_color, selector_label_font, selector_pill_button_style, selector_text_color,
};
use desktop_assets::assets::Icon;
use desktop_ui_controls::overlays::AboveOverlay;
use desktop_ui_controls::system_settings_common::{
    SettingsSelectLabel, rounded_action_btn_style, settings_field_label,
};
use desktop_settings_providers::state::ProviderModelsSummary;
use super::reasoning_message::{Message, dark_tooltip_content};
const BOTTOM_BAR_CHEVRON_ICON_SIZE: f32 = 11.0;
const BOTTOM_BAR_ICON_SIZE: f32 = 12.0;
const BOTTOM_BAR_LABEL_SIZE: f32 = 12.0;
#[derive(Clone, Copy)]
pub struct ReasoningInput<'a> {
    pub providers: &'a [ProviderModelsSummary],
    pub model: &'a str,
    pub expanded: bool,
    pub i18n: desktop_i18n::I18n,
    pub session_id: Option<&'a str>,
}

#[path = "reasoning_slider.rs"]
mod reasoning_slider;

const SELECTOR_WIDTH: f32 = 260.0;
const SLIDER_WIDTH: f32 = SELECTOR_WIDTH - 24.0;
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

pub fn reasoning_effort_selector<'a>(
    app: ReasoningInput<'a>,
    agent: Option<&str>,
    current: Option<ReasoningEffortDto>,
) -> Option<Element<'a, Message>> {
    if agent.is_some() {
        return None;
    }
    let model = app.model;
    let options = match agent {
        Some(agent) => reasoning_effort_options(agent).to_vec(),
        None => model_reasoning_effort_options(app.providers, model),
    };
    if agent.is_none() && options.len() <= 1 {
        return None;
    }
    let expanded = app.expanded;
    let icon = icon_svg(Icon::BarChartFill, BOTTOM_BAR_ICON_SIZE).style(move |theme: &Theme, _| {
        svg::Style { color: Some(selector_text_color(theme, expanded || current.is_some())) }
    });
    let chevron = icon_svg(
        if expanded { Icon::ChevronUp } else { Icon::ChevronDown },
        BOTTOM_BAR_CHEVRON_ICON_SIZE,
    )
    .style(move |theme: &Theme, _| svg::Style {
        color: Some(selector_chevron_color(theme, expanded)),
    });
    let toggle = button(
        row![
            icon,
            text(localized_reasoning_effort_label(
                app.i18n,
                current
            ))
            .size(BOTTOM_BAR_LABEL_SIZE)
            .font(selector_label_font()),
            chevron,
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([4, 10])
    .style(move |theme: &Theme, status| {
        selector_pill_button_style(theme, status, expanded || current.is_some())
    })
    .on_press(Message::Toggle);
    let toggle: Element<'_, Message> = Tooltip::new(
        toggle,
        dark_tooltip_content(
            app.i18n.__("选择本会话的推理强度"),
        ),
        TooltipPosition::Top,
    )
    .gap(6.0)
    .into();

    let popover = reasoning_effort_popover(app, model, &options, current);

    Some(
        AboveOverlay::new(toggle, popover)
            .show(expanded)
            .block_background_events(true)
            .gap(6.0)
            .on_close(Message::Close)
            .into(),
    )
}

/// 保留弹层以便连续拖动；点击外部或入口按钮时关闭。
pub fn reasoning_effort_popover<'a>(
    app: ReasoningInput<'a>,
    model: &str,
    options: &[Option<ReasoningEffortDto>],
    current: Option<ReasoningEffortDto>,
) -> Element<'a, Message> {
    let i18n = app.i18n;
    let items = reasoning_effort_items(options, current);
    let selected = items.iter().position(|item| item.selected).unwrap_or(0);
    let control = reasoning_slider::ReasoningSlider {
        selected,
        options: options.to_vec(),
        identity: format!("{:?}:{model}:{options:?}", app.session_id),
    };
    let reset = Tooltip::new(
        button(icon_svg(Icon::ArrowCounterClockwise, 14.0))
            .padding(5)
            .style(rounded_action_btn_style)
            .on_press(reasoning_effort_message(None)),
        dark_tooltip_content(i18n.__("默认")),
        TooltipPosition::Top,
    );
    let model_name =
        super::model_popover::model_toggle_label(app.providers, false, model);
    let heading = row![
        icon_svg(Icon::BarChartFill, 16.0),
        column![
            settings_field_label(localized_reasoning_effort_label(i18n, current))
                .font(selector_label_font()),
            SettingsSelectLabel::new(model_name),
        ]
        .spacing(2)
        .width(Length::Fill),
        reset,
    ]
    .spacing(6)
    .align_y(Alignment::Center);
    container(column![heading, control].spacing(12))
        .padding(12)
        .width(Length::Fixed(SELECTOR_WIDTH))
        .style(popover_style)
        .into()
}
