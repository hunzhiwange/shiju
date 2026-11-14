pub(crate) mod s3_modal;
pub(crate) mod task_panel;
pub(crate) mod webdav_modal;

use crate::app::components::system_settings_common::rounded_action_btn_style;
use crate::app::message::settings::{device_sync_s3, device_sync_webdav};
use crate::app::{App, Message};
use iced::widget::{button, row, stack, text};
use iced::{Element, Length};

pub(crate) fn entry_points() -> Element<'static, Message> {
    row![
        button(text("WebDAV 同步"))
            .on_press(device_sync_webdav::message(device_sync_webdav::Event::Open))
            .padding([8, 14])
            .style(rounded_action_btn_style),
        button(text("S3 同步"))
            .on_press(device_sync_s3::message(device_sync_s3::Event::Open))
            .padding([8, 14])
            .style(rounded_action_btn_style),
    ]
    .spacing(8)
    .into()
}
pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    if device_sync_webdav::visible(app) {
        stack![base, webdav_modal::view(app)].width(Length::Fill).height(Length::Fill).into()
    } else if device_sync_s3::visible(app) {
        stack![base, s3_modal::view(app)].width(Length::Fill).height(Length::Fill).into()
    } else {
        base
    }
}
