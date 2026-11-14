use crate::app::{App, Message};
use crate::app::components::system_settings_common::*;
use crate::app::message::settings::data_management::{backup_list_ui::{Event, message}, locations_ui};
use iced::{Element, widget::{button, column, row, text}};

pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let state = &app.apps_catalog.data_backups;
    let local = crate::app::config::gateway_client().ok().is_some_and(|client|
        locations_ui::can_reveal(client.endpoint().is_loopback(), &app.apps_catalog.host));
    let enabled = !state.busy && state.pending.is_none() && !state.principal.trim().is_empty();
    let mut body = column![text("执行主机备份分类"), text(format!("目标主机：{}", app.apps_catalog.host))].spacing(10);
    if !local { body = body.push(text("远程目录仅属于目标主机，不能在本机定位。")); }
    if let Some(result) = &state.cleanup_result { body = body.push(text(result.clone())); }
    for location in &state.locations {
        body = body.push(settings_group_card(column![
            text(format!("{}：{} 项，{} 字节", location.id, location.item_count, location.size_bytes)),
            row![
                button(text("清理分类")).padding([8,14]).style(rounded_action_btn_style)
                    .on_press_maybe((enabled && locations_ui::can_cleanup(location)).then(|| message(Event::Cleanup(location.id.clone())))),
                button(text("定位目录")).padding([8,14]).style(rounded_action_btn_style)
                    .on_press_maybe((enabled && local).then(|| message(Event::Reveal(location.id.clone())))),
            ].spacing(10),
        ].spacing(8)));
    }
    body.into()
}
