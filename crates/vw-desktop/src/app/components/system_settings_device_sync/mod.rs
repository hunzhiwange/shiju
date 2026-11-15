pub(crate) mod s3_modal;
mod selection;
pub(crate) mod task_panel;
pub(crate) mod webdav_modal;

use crate::app::message::settings::{device_sync_s3, device_sync_webdav};
use crate::app::{App, Message};
use iced::widget::stack;
use iced::{Element, Length};

pub(crate) fn entry_points() -> Element<'static, Message> {
    use desktop_settings_memory::system_settings_device_sync::{self as domain, EntryPoint};
    domain::entry_points().map(|event| match event {
        EntryPoint::WebDav => device_sync_webdav::message(device_sync_webdav::Event::Open),
        EntryPoint::S3 => device_sync_s3::message(device_sync_s3::Event::Open),
    })
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
