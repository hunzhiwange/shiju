
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
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::*;
use iced::{Element, widget::{button, column, row, text}};

pub fn view<'a>(i18n: I18n, state: ViewState<'a>, local: bool, _host: &'a str) -> Element<'a, Event> {
    let enabled = !state.busy && !state.pending;
    let mut body = column![text(i18n.__("备份分类"))].spacing(10);
    if !local { body = body.push(text(i18n.__("此目录无法在本机定位。"))); }
    if let Some(result) = &state.cleanup_result { body = body.push(text(desktop_i18n::settings::message(i18n, result))); }
    for location in state.locations {
        body = body.push(settings_group_card(column![
            text(desktop_i18n::__!(i18n, "%s：%u 项，%u 字节", (location.id).to_string(), location.item_count, location.size_bytes).expect("valid settings message")),
            row![
                button(text(i18n.__("清理分类"))).padding([8,14]).style(rounded_action_btn_style)
                    .on_press_maybe((enabled && super::locations::can_cleanup(location)).then(|| message(Event::Cleanup(location.id.clone())))),
                button(text(i18n.__("定位目录"))).padding([8,14]).style(rounded_action_btn_style)
                    .on_press_maybe((enabled && local).then(|| message(Event::Reveal(location.id.clone())))),
            ].spacing(10),
        ].spacing(8)));
    }
    body.into()
}
