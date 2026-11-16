use crate::app::{App, Message};
use iced::Element;
use desktop_settings_memory::system_settings_device_sync::webdav_modal as domain;
fn message(event: domain::Event) -> Message {
    crate::app::message::settings::device_sync_webdav::message(match event {
        domain::Event::Enabled(v) => crate::app::message::settings::device_sync_webdav::Event::Enabled(v),
        domain::Event::Auto(v) => crate::app::message::settings::device_sync_webdav::Event::Auto(v),
        domain::Event::BaseUrl(v) => crate::app::message::settings::device_sync_webdav::Event::BaseUrl(v),
        domain::Event::Username(v) => crate::app::message::settings::device_sync_webdav::Event::Username(v),
        domain::Event::Password(v) => crate::app::message::settings::device_sync_webdav::Event::Password(v),
        domain::Event::Root(v) => crate::app::message::settings::device_sync_webdav::Event::Root(v),
        domain::Event::Profile(v) => crate::app::message::settings::device_sync_webdav::Event::Profile(v),
        domain::Event::Principal(v) => crate::app::message::settings::device_sync_webdav::Event::Principal(v),
        domain::Event::ConfirmAuto(v) => crate::app::message::settings::device_sync_webdav::Event::ConfirmAuto(v),
        domain::Event::ConfirmClear(v) => crate::app::message::settings::device_sync_webdav::Event::ConfirmClear(v),
        domain::Event::Close => crate::app::message::settings::device_sync_webdav::Event::Close,
        domain::Event::Load => crate::app::message::settings::device_sync_webdav::Event::Load,
        domain::Event::Test => crate::app::message::settings::device_sync_webdav::Event::Test,
        domain::Event::Clear => crate::app::message::settings::device_sync_webdav::Event::Clear,
        domain::Event::Save => crate::app::message::settings::device_sync_webdav::Event::Save,
        domain::Event::Selection(v) => crate::app::message::settings::device_sync_webdav::Event::Selection(v),
        domain::Event::Preview(v) => crate::app::message::settings::device_sync_webdav::Event::Preview(v),
        domain::Event::TaskPanel(v) => crate::app::message::settings::device_sync_webdav::Event::TaskPanel(v),
    })
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let s = &app.apps_catalog.device_sync_webdav;
    let state = domain::ViewState { busy: s.busy, draft: &s.draft, principal: &s.principal, secret_present: s.secret_present, auto_confirmation: s.auto_confirmation, task_busy: s.task_panel.is_busy(), clear_confirmation: s.clear_confirmation };
    let editable = state.editable();
    let selection = super::selection::inventory(app);
    let selection = desktop_settings_memory::system_settings_device_sync::selection::view(selection, &s.selection, editable).map(domain::Event::Selection);
    domain::view(state, selection, s.preview.view().map(domain::Event::Preview), s.task_panel.view().map(domain::Event::TaskPanel)).map(message)
}
