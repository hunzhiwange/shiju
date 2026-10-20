use std::fmt;

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Space, button, column, container, pick_list, row, scrollable, text};
use iced::{Alignment, Element, Font, Length};
use vw_gateway_client::ShijuIamApiKeyDto;

use super::styles;
use crate::app::components::system_settings_common::{
    rounded_action_btn_style, settings_close_button, settings_modal_card, settings_modal_overlay,
    settings_muted_text_style, settings_pick_list_menu_style, settings_pick_list_style,
};
use crate::app::message::ShijuAppsMessage;
use crate::app::shiju_apps::{ShijuAppLaunchState, ShijuAppsLoadState};
use crate::app::{App, Message};

#[derive(Debug, Clone, PartialEq, Eq)]
struct ApiKeyOption {
    api_key_id: u64,
    label: String,
}

impl fmt::Display for ApiKeyOption {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.label)
    }
}

pub(crate) fn is_visible(state: &ShijuAppLaunchState) -> bool {
    matches!(
        state,
        ShijuAppLaunchState::LoadingApiKeys { .. }
            | ShijuAppLaunchState::SelectingApiKey { .. }
            | ShijuAppLaunchState::SavingApiKeyBinding { .. }
    )
}

pub(crate) fn selected_key(state: &ShijuAppLaunchState) -> Option<&ShijuIamApiKeyDto> {
    let (api_keys, selected_api_key_id) = match state {
        ShijuAppLaunchState::SelectingApiKey {
            api_keys,
            selected_api_key_id: Some(selected_api_key_id),
            ..
        } => (api_keys, *selected_api_key_id),
        ShijuAppLaunchState::SavingApiKeyBinding { api_keys, selected_api_key_id, .. } => {
            (api_keys, *selected_api_key_id)
        }
        _ => return None,
    };
    api_keys.iter().find(|key| key.api_key_id == selected_api_key_id)
}

pub(crate) fn selected_label(state: &ShijuAppLaunchState) -> Option<String> {
    selected_key(state).map(option_label)
}

pub(crate) fn save_enabled(state: &ShijuAppLaunchState) -> bool {
    matches!(state, ShijuAppLaunchState::SelectingApiKey { can_edit: true, .. })
        && selected_key(state).is_some_and(|key| key.status == "enabled")
}

pub(crate) fn select_action(api_key_id: u64) -> ShijuAppsMessage {
    ShijuAppsMessage::SelectApiKey(api_key_id)
}

pub(crate) fn refresh_action() -> ShijuAppsMessage {
    ShijuAppsMessage::RefreshApiKeys
}

pub(crate) fn save_action() -> ShijuAppsMessage {
    ShijuAppsMessage::SaveApiKeyBinding
}

pub(crate) fn cancel_action() -> ShijuAppsMessage {
    ShijuAppsMessage::CancelApiKeyBinding
}

fn option_label(key: &ShijuIamApiKeyDto) -> String {
    format!("{} ({})", key.key_name, key.api_key_mask)
}

fn application_name(app: &App, installation_id: &str) -> String {
    let ShijuAppsLoadState::Loaded(apps) = &app.expert_hub.shiju_apps.mine else {
        return "识矩应用".to_string();
    };
    apps.apps
        .iter()
        .find(|item| item.installation_id == installation_id)
        .map(|item| item.name.clone())
        .unwrap_or_else(|| "识矩应用".to_string())
}

fn installation_id(state: &ShijuAppLaunchState) -> Option<&str> {
    match state {
        ShijuAppLaunchState::CheckingBinding { installation_id, .. }
        | ShijuAppLaunchState::LoadingApiKeys { installation_id, .. }
        | ShijuAppLaunchState::SelectingApiKey { installation_id, .. }
        | ShijuAppLaunchState::SavingApiKeyBinding { installation_id, .. }
        | ShijuAppLaunchState::Error { installation_id, .. } => Some(installation_id),
        ShijuAppLaunchState::Idle | ShijuAppLaunchState::Preparing { .. } => None,
    }
}

fn status_label(state: &ShijuAppLaunchState) -> &'static str {
    match state {
        ShijuAppLaunchState::CheckingBinding { .. } => "检查绑定",
        ShijuAppLaunchState::LoadingApiKeys { .. } => "加载密钥",
        ShijuAppLaunchState::SelectingApiKey { can_edit: false, .. } => "不可编辑",
        ShijuAppLaunchState::SelectingApiKey { .. } => "待配置",
        ShijuAppLaunchState::SavingApiKeyBinding { .. } => "保存中",
        ShijuAppLaunchState::Error { .. } => "需要处理",
        ShijuAppLaunchState::Idle | ShijuAppLaunchState::Preparing { .. } => "运行态",
    }
}

fn ascii_value(value: String, size: u32) -> iced::widget::Text<'static> {
    let is_ascii = value.is_ascii();
    let value = text(value).size(size);
    if is_ascii { value.font(Font::MONOSPACE) } else { value }
}

fn detail_field(label: &'static str, value: String, monospace: bool) -> Element<'static, Message> {
    let value = if monospace && value.is_ascii() {
        text(value).size(12).font(Font::MONOSPACE)
    } else {
        text(value).size(13)
    };
    column![text(label).size(11).style(settings_muted_text_style), value].spacing(4).into()
}

fn selected_key_details(key: &ShijuIamApiKeyDto) -> Element<'static, Message> {
    let enabled = key.status == "enabled";
    let status = if enabled { "已启用" } else { "未启用" };
    container(
        column![
            row![
                text("所选密钥详情").size(14),
                Space::new().width(Length::Fill),
                container(text(status).size(11))
                    .padding([4, 9])
                    .style(move |theme| styles::status_style(theme, enabled)),
            ]
            .align_y(Alignment::Center),
            detail_field("名称", key.key_name.clone(), false),
            detail_field("掩码", key.api_key_mask.clone(), true),
            detail_field("密钥 ID", key.api_key_id.to_string(), true),
            detail_field(
                "有效期",
                key.expires_at.clone().unwrap_or_else(|| "长期有效".to_string()),
                true,
            ),
        ]
        .spacing(10),
    )
    .padding(14)
    .width(Length::Fill)
    .style(styles::binding_field_style)
    .into()
}

fn error_banner(message: String) -> Element<'static, Message> {
    container(text(message).size(12).width(Length::Fill))
        .padding([10, 12])
        .width(Length::Fill)
        .style(styles::binding_error_style)
        .into()
}

fn disabled_selection(label: String) -> Element<'static, Message> {
    container(text(label).size(13).width(Length::Fill))
        .padding([10, 12])
        .width(Length::Fill)
        .style(styles::binding_field_style)
        .into()
}

fn loading_feedback(label: &'static str) -> Element<'static, Message> {
    container(text(label).size(13).style(settings_muted_text_style))
        .width(Length::Fill)
        .height(Length::Fixed(96.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(styles::binding_field_style)
        .into()
}

fn selection_control(state: &ShijuAppLaunchState) -> Element<'static, Message> {
    match state {
        ShijuAppLaunchState::SelectingApiKey { can_edit: true, api_keys, .. }
            if !api_keys.is_empty() =>
        {
            let options = api_keys
                .iter()
                .map(|key| ApiKeyOption { api_key_id: key.api_key_id, label: option_label(key) })
                .collect::<Vec<_>>();
            let selected_id = selected_key(state).map(|key| key.api_key_id);
            let selected =
                options.iter().find(|option| Some(option.api_key_id) == selected_id).cloned();
            pick_list(options, selected, |option| {
                Message::ShijuApps(select_action(option.api_key_id))
            })
            .placeholder("请选择 IAM API 密钥")
            .padding([10, 12])
            .text_size(13)
            .width(Length::Fill)
            .style(settings_pick_list_style)
            .menu_style(settings_pick_list_menu_style)
            .into()
        }
        ShijuAppLaunchState::SavingApiKeyBinding { .. } => disabled_selection(
            selected_label(state).unwrap_or_else(|| "正在保存所选密钥".to_string()),
        ),
        ShijuAppLaunchState::SelectingApiKey { can_edit: false, .. } => {
            disabled_selection("此应用不允许修改个人 API 密钥绑定".to_string())
        }
        ShijuAppLaunchState::SelectingApiKey { .. } => {
            disabled_selection("当前账户没有可选择的 IAM API 密钥".to_string())
        }
        ShijuAppLaunchState::CheckingBinding { .. } => {
            disabled_selection("正在检查应用 API 密钥绑定…".to_string())
        }
        ShijuAppLaunchState::LoadingApiKeys { .. } => {
            disabled_selection("正在加载 IAM API 密钥…".to_string())
        }
        ShijuAppLaunchState::Error { .. } => {
            disabled_selection("加载失败，请刷新后重试".to_string())
        }
        ShijuAppLaunchState::Idle | ShijuAppLaunchState::Preparing { .. } => {
            disabled_selection("暂无可选择的密钥".to_string())
        }
    }
}

fn body(app: &App) -> Element<'static, Message> {
    let state = &app.expert_hub.shiju_apps.launch;
    let current_installation_id = installation_id(state).unwrap_or_default();
    let app_name = application_name(app, current_installation_id);
    let status_error = matches!(state, ShijuAppLaunchState::Error { .. });
    let close_message = Message::ShijuApps(cancel_action());
    let mut refresh_button =
        button(text("刷新").size(12)).padding([7, 12]).style(rounded_action_btn_style);
    if matches!(
        state,
        ShijuAppLaunchState::SelectingApiKey { .. } | ShijuAppLaunchState::Error { .. }
    ) {
        refresh_button = refresh_button.on_press(Message::ShijuApps(refresh_action()));
    }

    let header = row![
        column![
            text("应用 API 密钥").size(22),
            text("仅绑定当前账户启用的 IAM API 密钥；不会展示或保存原始密钥。")
                .size(12)
                .style(settings_muted_text_style),
        ]
        .spacing(5)
        .width(Length::Fill),
        refresh_button,
        settings_close_button(close_message),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let summary = row![
        column![
            text(app_name).size(16),
            ascii_value(current_installation_id.to_string(), 11).style(settings_muted_text_style),
        ]
        .spacing(4)
        .width(Length::Fill),
        container(text("个人使用").size(11)).padding([4, 9]).style(styles::badge_style),
        container(text(status_label(state)).size(11))
            .padding([4, 9])
            .style(move |theme| styles::binding_state_style(theme, status_error)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut content = column![
        summary,
        column![
            text("IAM API 密钥").size(12),
            selection_control(state),
            text("列表包含当前账户返回的启用与非启用项；只有启用项可以保存。")
                .size(11)
                .style(settings_muted_text_style),
        ]
        .spacing(7),
    ]
    .spacing(14);

    match state {
        ShijuAppLaunchState::SelectingApiKey { error: Some(error), .. } => {
            content = content.push(error_banner(error.clone()));
        }
        ShijuAppLaunchState::Error { message, .. } => {
            content = content.push(error_banner(message.clone()));
        }
        _ => {}
    }
    if let Some(key) = selected_key(state) {
        content = content.push(selected_key_details(key));
    } else {
        match state {
            ShijuAppLaunchState::CheckingBinding { .. } => {
                content = content.push(loading_feedback("正在检查当前绑定状态…"));
            }
            ShijuAppLaunchState::LoadingApiKeys { .. } => {
                content = content.push(loading_feedback("正在安全加载 IAM API 密钥…"));
            }
            ShijuAppLaunchState::SelectingApiKey { error: None, .. } => {
                content = content
                    .push(error_banner("尚未绑定 IAM API 密钥，请选择一个已启用项。".to_string()));
            }
            _ => {}
        }
    }

    let mut save_button =
        button(text(if matches!(state, ShijuAppLaunchState::SavingApiKeyBinding { .. }) {
            "正在保存 API 密钥…"
        } else {
            "保存 API 密钥"
        }))
        .padding([11, 16])
        .width(Length::Fill)
        .style(styles::binding_primary_action_style);
    if save_enabled(state) {
        save_button = save_button.on_press(Message::ShijuApps(save_action()));
    }
    content = content.push(save_button);

    let scrollable_body = scrollable(
        container(content).padding([0, styles::SCROLLABLE_SIDE_PADDING]).width(Length::Fill),
    )
    .direction(Direction::Vertical(
        Scrollbar::new().width(styles::SCROLLBAR_WIDTH).scroller_width(styles::SCROLLBAR_WIDTH),
    ))
    .height(Length::Shrink);
    column![header, scrollable_body].spacing(16).into()
}

pub(super) fn with_overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    if !is_visible(&app.expert_hub.shiju_apps.launch) {
        return base;
    }
    let close_message = Message::ShijuApps(cancel_action());
    let card = settings_modal_card(body(app)).width(Length::Fixed(640.0)).max_height(680.0);
    settings_modal_overlay(Some(base), close_message, card)
}
