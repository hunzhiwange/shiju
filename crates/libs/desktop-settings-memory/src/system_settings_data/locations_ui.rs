
#[derive(Clone)]
pub enum Event {
    Cleanup(String),
    Reveal(String),
}
fn message(event: Event) -> Event { event }
pub struct ViewState<'a> {
    pub busy: bool,
    pub pending: bool,
    pub principal: &'a str,
    pub cleanup_result: &'a Option<String>,
    pub locations: &'a [gateway_app_data::DataBackupLocation],
}
use desktop_ui_controls::system_settings_common::*;
use iced::{Element, widget::{button, column, row, text}};

pub fn view<'a>(state: ViewState<'a>, local: bool, host: &'a str) -> Element<'a, Event> {
    let enabled = !state.busy && !state.pending;
    let mut body = column![text("执行主机备份分类"), text(format!("目标主机：{}", host))].spacing(10);
    if !local { body = body.push(text("远程目录仅属于目标主机，不能在本机定位。")); }
    if let Some(result) = &state.cleanup_result { body = body.push(text(result.clone())); }
    for location in state.locations {
        body = body.push(settings_group_card(column![
            text(format!("{}：{} 项，{} 字节", location.id, location.item_count, location.size_bytes)),
            row![
                button(text("清理分类")).padding([8,14]).style(rounded_action_btn_style)
                    .on_press_maybe((enabled && super::locations::can_cleanup(location)).then(|| message(Event::Cleanup(location.id.clone())))),
                button(text("定位目录")).padding([8,14]).style(rounded_action_btn_style)
                    .on_press_maybe((enabled && local).then(|| message(Event::Reveal(location.id.clone())))),
            ].spacing(10),
        ].spacing(8)));
    }
    body.into()
}
