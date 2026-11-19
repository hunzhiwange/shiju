pub(crate) mod s3_modal;
mod selection;
pub(crate) mod task_panel;
pub(crate) mod webdav_modal;

use crate::app::message::settings::{device_sync_s3, device_sync_webdav};
use crate::app::{
    App, Message,
    message::{SettingsMessage, settings::AppsCatalogMessage},
};
use desktop_apps_ui::catalog_state::DeviceSyncMethod;
use desktop_ui_controls::system_settings_common::*;
use iced::widget::{column, text};
use iced::{Element, Length, Task};

fn select_message(method: DeviceSyncMethod, inline: bool) -> Message {
    Message::Settings(SettingsMessage::AppsCatalog(AppsCatalogMessage::SelectDeviceSync(
        method, inline,
    )))
}

pub(crate) fn select(app: &mut App, method: DeviceSyncMethod, inline: bool) -> Task<Message> {
    if inline {
        app.apps_catalog.device_sync_inline = Some(method);
        app.apps_catalog.device_sync_dialog = None;
    } else {
        app.apps_catalog.device_sync_dialog = Some(method);
    }

    match method {
        DeviceSyncMethod::WebDav if !device_sync_webdav::visible(app) => {
            device_sync_webdav::update(app, device_sync_webdav::Event::Open)
        }
        DeviceSyncMethod::S3 if !device_sync_s3::visible(app) => {
            device_sync_s3::update(app, device_sync_s3::Event::Open)
        }
        _ => Task::none(),
    }
}

pub(crate) fn entry_points() -> Element<'static, Message> {
    use desktop_settings_memory::system_settings_device_sync::{self as domain, EntryPoint};
    domain::entry_points().map(|event| match event {
        EntryPoint::WebDav => select_message(DeviceSyncMethod::WebDav, false),
        EntryPoint::S3 => select_message(DeviceSyncMethod::S3, false),
    })
}

pub(crate) fn inline(app: &App) -> Element<'_, Message> {
    let active = match app.apps_catalog.device_sync_inline {
        Some(DeviceSyncMethod::WebDav) if device_sync_webdav::visible(app) => {
            Some(DeviceSyncMethod::WebDav)
        }
        Some(DeviceSyncMethod::S3) if device_sync_s3::visible(app) => Some(DeviceSyncMethod::S3),
        _ => None,
    };
    let selector = settings_responsive_row("同步方式", "", move || {
        settings_select([DeviceSyncMethod::WebDav, DeviceSyncMethod::S3], active, |method| {
            select_message(method, true)
        })
        .width(Length::Fill)
        .placeholder("选择同步方式")
    });
    let mut content = column![text("云同步").size(18), selector,].spacing(12);
    match active {
        Some(DeviceSyncMethod::WebDav) => content = content.push(webdav_modal::view(app, true)),
        Some(DeviceSyncMethod::S3) => content = content.push(s3_modal::view(app, true)),
        None => {}
    }
    settings_panel(content).into()
}

pub(crate) fn overlay<'a>(app: &'a App, base: Element<'a, Message>) -> Element<'a, Message> {
    let (close, card) = match app.apps_catalog.device_sync_dialog {
        Some(DeviceSyncMethod::WebDav) if device_sync_webdav::visible(app) => (
            device_sync_webdav::message(device_sync_webdav::Event::Close),
            webdav_modal::view(app, false),
        ),
        Some(DeviceSyncMethod::S3) if device_sync_s3::visible(app) => {
            (device_sync_s3::message(device_sync_s3::Event::Close), s3_modal::view(app, false))
        }
        _ => return base,
    };
    settings_modal_overlay(Some(base), close, card)
}
