//! SDK 图标库与改名入口。
mod rename_modal;
use super::system_settings_common::{
    rounded_action_btn_style, settings_divider, settings_error_banner, settings_group_card,
    settings_group_heading, settings_muted_text_style,
};
use crate::app::{
    Message,
    message::{SettingsMessage, settings::AgentAppsMessage},
    state::AgentAppsState,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use iced::widget::{button, column, container, image, row, text, text::Wrapping};
use iced::{Alignment, Element, Length};

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
    let refresh = button(text(if state.error.is_some() { "重试" } else { "刷新" }).size(13))
        .padding([8, 14])
        .style(rounded_action_btn_style)
        .on_press_maybe((!state.loading).then(refresh_message));
    let header = row![
        container(settings_group_heading("我的图标", "读取当前网关与公司的图标库"))
            .width(Length::Fill),
        refresh,
    ]
    .spacing(12)
    .align_y(Alignment::Center);

    let mut entries = column![].width(Length::Fill);
    if let Some(status) = status_text(state) {
        let status = if state.error.is_some() && !state.loading {
            settings_error_banner(status.to_owned())
        } else {
            text(status).size(13).style(settings_muted_text_style).into()
        };
        entries = entries.push(container(status).padding([14, 0]).width(Length::Fill));
    } else {
        for (index, icon) in state.icons.iter().enumerate() {
            if index > 0 {
                entries = entries.push(settings_divider());
            }
            let mut entry = row![].spacing(12).align_y(Alignment::Center);
            if let Some((header, encoded)) = icon.data.split_once(',')
                && header.starts_with("data:image/")
                && header.ends_with(";base64")
                && let Ok(bytes) = STANDARD.decode(encoded)
            {
                entry = entry.push(image(image::Handle::from_bytes(bytes)).width(32).height(32));
            }
            let rename = button(text("改名").size(13))
                .padding([8, 14])
                .style(rounded_action_btn_style)
                .on_press(Message::Settings(SettingsMessage::AgentApps(
                    AgentAppsMessage::RenameOpen(icon.id.clone()),
                )));
            entries = entries.push(
                container(
                    entry
                        .push(
                            text(&icon.name)
                                .size(14)
                                .wrapping(Wrapping::WordOrGlyph)
                                .width(Length::Fill),
                        )
                        .push(rename),
                )
                .padding([12, 0])
                .width(Length::Fill),
            );
        }
    }
    column![header, settings_group_card(entries)].spacing(12).width(Length::Fill).into()
}

#[cfg(test)]
#[path = "system_settings_agent_apps/rename_modal_tests.rs"]
mod rename_modal_tests;

pub(super) fn view_overlays<'a>(
    state: &'a AgentAppsState,
    base: Element<'a, Message>,
) -> Element<'a, Message> {
    match &state.rename {
        Some(form) => rename_modal::view(form, base),
        None => base,
    }
}

#[cfg(test)]
#[path = "system_settings_agent_apps_ui_refresh_24_tests.rs"]
mod ui_refresh_24_tests;
