//! SDK 图标库的最小只读设置入口。
use super::system_settings_common::{
    rounded_action_btn_style, settings_error_banner, settings_muted_text_style,
    settings_section_card, settings_section_content,
};
use crate::app::{
    Message,
    message::{SettingsMessage, settings::AgentAppsMessage},
    state::AgentAppsState,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use iced::widget::{button, column, container, image, row, text};
use iced::{Element, Length};

pub(super) fn refresh_message() -> Message {
    Message::Settings(SettingsMessage::AgentApps(AgentAppsMessage::Refresh))
}

pub(super) fn status_text(state: &AgentAppsState) -> Option<&str> {
    if state.loading {
        Some("正在加载图标…")
    } else if let Some(error) = state.error.as_deref() {
        Some(error)
    } else if state.icons.is_empty() {
        Some("暂无图标，请点击刷新读取图标库。")
    } else {
        None
    }
}

pub(super) fn view(state: &AgentAppsState) -> Element<'_, Message> {
    let mut refresh = button(text(if state.error.is_some() { "重试" } else { "刷新" }))
        .padding([8, 16])
        .style(rounded_action_btn_style);
    if !state.loading {
        refresh = refresh.on_press(refresh_message());
    }
    let mut content = column![
        settings_section_card("我的图标", "读取当前网关与公司的图标库"),
        row![container(text("图标库")).width(Length::Fill), refresh].spacing(12)
    ]
    .spacing(12)
    .width(Length::Fill);
    if let Some(status) = status_text(state) {
        if state.error.is_some() && !state.loading {
            content = content.push(settings_error_banner(status.to_owned()));
        } else {
            content = content.push(text(status).style(settings_muted_text_style));
        }
    }
    if !state.loading && state.error.is_none() {
        for icon in &state.icons {
            let mut entry = row![].spacing(12).align_y(iced::Alignment::Center);
            if let Some((header, encoded)) = icon.data.split_once(',')
                && header.starts_with("data:image/")
                && header.ends_with(";base64")
                && let Ok(bytes) = STANDARD.decode(encoded)
            {
                entry = entry.push(image(image::Handle::from_bytes(bytes)).width(32).height(32));
            }
            content = content.push(settings_section_content(entry.push(text(&icon.name).size(14))));
        }
    }
    container(content).padding([0, 7]).width(Length::Fill).into()
}
