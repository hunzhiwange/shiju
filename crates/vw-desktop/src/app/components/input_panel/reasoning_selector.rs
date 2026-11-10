//! 聊天输入栏的会话级推理强度选择器。

use iced::widget::svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length, Theme};
use vw_gateway_client::vw_api_types::ReasoningEffortDto;

use super::dark_tooltip_content;
use super::icons::icon_svg;
use super::styles::{
    BOTTOM_BAR_CHEVRON_ICON_SIZE, BOTTOM_BAR_ICON_SIZE, BOTTOM_BAR_LABEL_SIZE, popover_style,
    selector_chevron_color, selector_label_font, selector_pill_button_style, selector_text_color,
};
use crate::app::assets::Icon;
use crate::app::components::overlays::AboveOverlay;
use crate::app::components::system_settings_common::{
    SettingsSelectLabel, rounded_action_btn_style, settings_field_label,
};
use crate::app::{App, Message, message};

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
pub(crate) fn model_reasoning_effort_options(
    app: &App,
    model: &str,
) -> Vec<Option<ReasoningEffortDto>> {
    let (provider_id, model_id) =
        model.split_once('/').map_or((None, model), |(p, m)| (Some(p), m));
    let mut options = vec![None];
    let candidate = app
        .model_settings
        .providers
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
        let declared: Option<Vec<vw_shared::provider::reasoning::ReasoningOption>> = detail
            .pointer("/capabilities/reasoning_options")
            .map(|value| serde_json::from_value(value.clone()).unwrap_or_default());
        let levels = vw_shared::provider::reasoning::reasoning_levels(
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
pub(super) struct ReasoningEffortItem {
    pub(super) value: Option<ReasoningEffortDto>,
    pub(super) selected: bool,
}

pub(crate) fn reasoning_effort_label(value: Option<ReasoningEffortDto>) -> &'static str {
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

pub(crate) fn localized_reasoning_effort_label(
    i18n: crate::app::i18n::I18n,
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

pub(crate) fn reasoning_effort_options(agent: &str) -> &'static [Option<ReasoningEffortDto>] {
    match agent.trim().to_ascii_lowercase().as_str() {
        "codex" | "codex cli" | "codex-cli" => &CODEX_OPTIONS,
        "claude" | "claude code" | "claude-code" | "claudecode" => &CLAUDE_OPTIONS,
        _ => &DEFAULT_OPTIONS,
    }
}

pub(super) fn reasoning_effort_items(
    options: &[Option<ReasoningEffortDto>],
    current: Option<ReasoningEffortDto>,
) -> Vec<ReasoningEffortItem> {
    options
        .iter()
        .copied()
        .map(|value| ReasoningEffortItem { value, selected: value == current })
        .collect()
}

pub(super) fn reasoning_effort_message(value: Option<ReasoningEffortDto>) -> Message {
    Message::Chat(message::ChatMessage::ReasoningEffortSelected(value))
}

pub(super) fn reasoning_effort_selector<'a>(
    app: &'a App,
    agent: Option<&str>,
    current: Option<ReasoningEffortDto>,
) -> Option<Element<'a, Message>> {
    if agent.is_some() {
        return None;
    }
    let runtime = app.current_session_runtime_ref();
    let model = runtime.map(|runtime| runtime.model.as_str()).unwrap_or(&app.model);
    let options = match agent {
        Some(agent) => reasoning_effort_options(agent).to_vec(),
        None => model_reasoning_effort_options(app, model),
    };
    if agent.is_none() && options.len() <= 1 {
        return None;
    }
    let expanded = app.show_reasoning_effort_popover;
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
                crate::app::i18n::I18n::new(app.desktop_locale),
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
    .on_press(Message::View(message::ViewMessage::ToggleReasoningEffortPopover));
    let toggle: Element<'_, Message> = Tooltip::new(
        toggle,
        dark_tooltip_content(
            crate::app::i18n::I18n::new(app.desktop_locale).__("选择本会话的推理强度"),
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
            .on_close(Message::View(message::ViewMessage::CloseReasoningEffortPopover))
            .into(),
    )
}

/// 保留弹层以便连续拖动；点击外部或入口按钮时关闭。
pub(super) fn reasoning_effort_popover<'a>(
    app: &'a App,
    model: &str,
    options: &[Option<ReasoningEffortDto>],
    current: Option<ReasoningEffortDto>,
) -> Element<'a, Message> {
    let i18n = crate::app::i18n::I18n::new(app.desktop_locale);
    let items = reasoning_effort_items(options, current);
    let selected = items.iter().position(|item| item.selected).unwrap_or(0);
    let control = reasoning_slider::ReasoningSlider {
        selected,
        options: options.to_vec(),
        identity: format!("{:?}:{model}:{options:?}", app.active_session_id),
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
        super::model_popover::model_toggle_label(&app.model_settings.providers, false, model);
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
