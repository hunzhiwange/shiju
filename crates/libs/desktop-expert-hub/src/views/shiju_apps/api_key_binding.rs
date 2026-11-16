use std::fmt;

use iced::widget::{Space, button, column, container, row, text};
use iced::{Alignment, Element, Font, Length};
use gateway_shiju_app::ShijuIamApiKeyDto;

use super::styles;
use desktop_ui_controls::system_settings_common::{
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_control_with_hint, settings_error_banner, settings_field_label, settings_modal_header,
    settings_modal_layout, settings_modal_overlay, settings_muted_text_style, settings_select,
};
use desktop_apps_ui::shiju_apps::ShijuAppsMessage;
use desktop_app_launch::{ShijuAppLaunchState, ShijuAppsLoadState};
use super::{ViewInput, Message};

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

pub fn is_visible(state: &ShijuAppLaunchState) -> bool {
    matches!(
        state,
        ShijuAppLaunchState::LoadingApiKeys { .. }
            | ShijuAppLaunchState::SelectingApiKey { .. }
            | ShijuAppLaunchState::SavingApiKeyBinding { .. }
    )
}

pub fn selected_key(state: &ShijuAppLaunchState) -> Option<&ShijuIamApiKeyDto> {
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

pub fn selected_label(state: &ShijuAppLaunchState) -> Option<String> {
    selected_key(state).map(option_label)
}

pub fn save_enabled(state: &ShijuAppLaunchState) -> bool {
    matches!(state, ShijuAppLaunchState::SelectingApiKey { can_edit: true, .. })
        && selected_key(state).is_some_and(|key| key.status == "enabled")
}

pub fn select_action(api_key_id: u64) -> ShijuAppsMessage {
    ShijuAppsMessage::SelectApiKey(api_key_id)
}

pub fn refresh_action() -> ShijuAppsMessage {
    ShijuAppsMessage::RefreshApiKeys
}

pub fn save_action() -> ShijuAppsMessage {
    ShijuAppsMessage::SaveApiKeyBinding
}

pub fn cancel_action() -> ShijuAppsMessage {
    ShijuAppsMessage::CancelApiKeyBinding
}

fn option_label(key: &ShijuIamApiKeyDto) -> String {
    format!("{} ({})", key.key_name, key.api_key_mask)
}

fn application_name(app: &ViewInput<'_>, installation_id: &str) -> String {
    let ShijuAppsLoadState::Loaded(apps) = &app.shiju_apps.mine else {
        return desktop_i18n::I18n::new(app.desktop_locale).__("识矩应用").to_string();
    };
    apps.apps
        .iter()
        .find(|item| item.installation_id == installation_id)
        .map(|item| item.name.clone())
        .unwrap_or_else(|| {
            desktop_i18n::I18n::new(app.desktop_locale).__("识矩应用").to_string()
        })
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

fn detail_field(
    i18n: desktop_i18n::I18n,
    label: &'static str,
    value: String,
    monospace: bool,
) -> Element<'static, Message> {
    let value = if monospace && value.is_ascii() {
        text(value).size(12).font(Font::MONOSPACE)
    } else {
        text(value).size(13)
    };
    column![text(i18n.__(label)).size(11).style(settings_muted_text_style), value].spacing(4).into()
}

fn selected_key_details(
    i18n: desktop_i18n::I18n,
    key: &ShijuIamApiKeyDto,
) -> Element<'static, Message> {
    let enabled = key.status == "enabled";
    let status = if enabled { i18n.__("已启用") } else { i18n.__("未启用") };
    container(
        column![
            row![
                text(i18n.__("所选密钥详情")).size(14),
                Space::new().width(Length::Fill),
                container(text(status).size(11))
                    .padding([4, 9])
                    .style(move |theme| styles::status_style(theme, enabled)),
            ]
            .align_y(Alignment::Center),
            detail_field(i18n, i18n.__("名称"), key.key_name.clone(), false),
            detail_field(i18n, i18n.__("掩码"), key.api_key_mask.clone(), true),
            detail_field(i18n, i18n.__("密钥 ID"), key.api_key_id.to_string(), true),
            detail_field(
                i18n,
                i18n.__("有效期"),
                key.expires_at.clone().unwrap_or_else(|| i18n.__("长期有效").to_string()),
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

fn disabled_selection(label: String) -> Element<'static, Message> {
    container(text(label).size(13).width(Length::Fill))
        .padding([10, 12])
        .width(Length::Fill)
        .style(styles::binding_field_style)
        .into()
}

fn loading_feedback(
    i18n: desktop_i18n::I18n,
    label: &'static str,
) -> Element<'static, Message> {
    container(text(i18n.__(label)).size(13).style(settings_muted_text_style))
        .width(Length::Fill)
        .height(Length::Fixed(96.0))
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .style(styles::binding_field_style)
        .into()
}

fn selection_control(
    i18n: desktop_i18n::I18n,
    state: &ShijuAppLaunchState,
) -> Element<'static, Message> {
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
            settings_select(options, selected, |option| {
                Message::ShijuApps(select_action(option.api_key_id))
            })
            .placeholder(i18n.__("请选择 IAM API 密钥"))
            .width(Length::Fill)
            .into()
        }
        ShijuAppLaunchState::SavingApiKeyBinding { .. } => disabled_selection(
            selected_label(state).unwrap_or_else(|| i18n.__("正在保存所选密钥").to_string()),
        ),
        ShijuAppLaunchState::SelectingApiKey { can_edit: false, .. } => {
            disabled_selection(i18n.__("此应用不允许修改个人 API 密钥绑定").to_string())
        }
        ShijuAppLaunchState::SelectingApiKey { .. } => {
            disabled_selection(i18n.__("当前账户没有可选择的 IAM API 密钥").to_string())
        }
        ShijuAppLaunchState::CheckingBinding { .. } => {
            disabled_selection(i18n.__("正在检查应用 API 密钥绑定…").to_string())
        }
        ShijuAppLaunchState::LoadingApiKeys { .. } => {
            disabled_selection(i18n.__("正在加载 IAM API 密钥…").to_string())
        }
        ShijuAppLaunchState::Error { .. } => {
            disabled_selection(i18n.__("加载失败，请刷新后重试").to_string())
        }
        ShijuAppLaunchState::Idle | ShijuAppLaunchState::Preparing { .. } => {
            disabled_selection(i18n.__("暂无可选择的密钥").to_string())
        }
    }
}

fn binding_card<'a>(app: &ViewInput<'_>) -> iced::widget::Container<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let state = &app.shiju_apps.launch;
    let current_installation_id = installation_id(state).unwrap_or_default();
    let app_name = application_name(app, current_installation_id);
    let status_error = matches!(state, ShijuAppLaunchState::Error { .. });
    let close_message = Message::ShijuApps(cancel_action());
    let mut refresh_button =
        button(text(desktop_i18n::I18n::new(app.desktop_locale).__("刷新")).size(12))
            .padding([7, 12])
            .style(rounded_action_btn_style);
    if matches!(
        state,
        ShijuAppLaunchState::SelectingApiKey { .. } | ShijuAppLaunchState::Error { .. }
    ) {
        refresh_button = refresh_button.on_press(Message::ShijuApps(refresh_action()));
    }

    let header = settings_modal_header(
        column![
            text(i18n.__("应用 API 密钥")).size(20),
            text(i18n.__("仅绑定当前账户启用的 IAM API 密钥；不会展示或保存原始密钥。"))
                .size(13)
                .style(settings_muted_text_style),
        ]
        .spacing(6),
        settings_close_button(close_message),
    );

    let summary = row![
        column![
            text(app_name).size(16),
            ascii_value(current_installation_id.to_string(), 11).style(settings_muted_text_style),
        ]
        .spacing(4)
        .width(Length::Fill),
        container(text(desktop_i18n::I18n::new(app.desktop_locale).__("个人使用")).size(11))
            .padding([4, 9])
            .style(styles::badge_style),
        container(
            text(desktop_i18n::I18n::new(app.desktop_locale).__(status_label(state))).size(11)
        )
        .padding([4, 9])
        .style(move |theme| styles::binding_state_style(theme, status_error)),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut content = column![
        summary,
        column![
            settings_field_label(i18n.__("IAM API 密钥")),
            settings_control_with_hint(
                selection_control(i18n, state),
                i18n.__("列表包含当前账户返回的启用与非启用项；只有启用项可以保存。"),
            ),
        ]
        .spacing(7),
    ]
    .spacing(14);

    match state {
        ShijuAppLaunchState::SelectingApiKey { error: Some(error), .. } => {
            content = content.push(settings_error_banner(
                app.shiju_apps
                    .render_launch_error(error, desktop_i18n::I18n::new(app.desktop_locale)),
            ));
        }
        ShijuAppLaunchState::Error { message, .. } => {
            content = content.push(settings_error_banner(
                app.shiju_apps
                    .render_launch_error(message, desktop_i18n::I18n::new(app.desktop_locale)),
            ));
        }
        _ => {}
    }
    if let Some(key) = selected_key(state) {
        content = content.push(selected_key_details(i18n, key));
    } else {
        match state {
            ShijuAppLaunchState::CheckingBinding { .. } => {
                content = content.push(loading_feedback(
                    i18n,
                    desktop_i18n::I18n::new(app.desktop_locale).__("正在检查当前绑定状态…"),
                ));
            }
            ShijuAppLaunchState::LoadingApiKeys { .. } => {
                content = content.push(loading_feedback(
                    i18n,
                    desktop_i18n::I18n::new(app.desktop_locale)
                        .__("正在安全加载 IAM API 密钥…"),
                ));
            }
            ShijuAppLaunchState::SelectingApiKey { error: None, .. } => {
                content = content.push(settings_error_banner(
                    desktop_i18n::I18n::new(app.desktop_locale)
                        .__("尚未绑定 IAM API 密钥，请选择一个已启用项。")
                        .to_string(),
                ));
            }
            _ => {}
        }
    }

    let mut save_button = button(text(i18n.__(
        if matches!(state, ShijuAppLaunchState::SavingApiKeyBinding { .. }) {
            desktop_i18n::I18n::new(app.desktop_locale).__("正在保存 API 密钥…")
        } else {
            desktop_i18n::I18n::new(app.desktop_locale).__("保存 API 密钥")
        },
    )))
    .padding([9, 16])
    .style(primary_action_btn_style);
    if save_enabled(state) {
        save_button = save_button.on_press(Message::ShijuApps(save_action()));
    }
    let footer = row![Space::new().width(Length::Fill), refresh_button, save_button]
        .spacing(10)
        .align_y(Alignment::Center);
    settings_modal_layout(header, content, Some(footer.into()))
        .width(Length::Fixed(640.0))
        .height(Length::Fixed(560.0))
}

pub fn with_overlay<'a, M: Clone + 'static>(app: &ViewInput<'_>, base: Element<'a, M>, map: impl Fn(Message) -> M + 'a) -> Element<'a, M> {
    if !is_visible(&app.shiju_apps.launch) {
        return base;
    }
    let close_message = Message::ShijuApps(cancel_action());
    let card: Element<'a, Message> = binding_card(app).into();
    settings_modal_overlay(Some(base), map(close_message), container(card.map(map)).padding(12))
}
