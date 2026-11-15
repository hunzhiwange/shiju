use crate::app::{App, Message};
use iced::Element;
use desktop_settings_memory::system_settings_device_sync::s3_modal as domain;
fn message(event: domain::Event) -> Message {
    crate::app::message::settings::device_sync_s3::message(match event {
        domain::Event::Preset(v) => crate::app::message::settings::device_sync_s3::Event::Preset(v),
        domain::Event::ClearSecret => crate::app::message::settings::device_sync_s3::Event::ClearSecret,
        domain::Event::Region(v) => crate::app::message::settings::device_sync_s3::Event::Region(v),
        domain::Event::Bucket(v) => crate::app::message::settings::device_sync_s3::Event::Bucket(v),
        domain::Event::Endpoint(v) => crate::app::message::settings::device_sync_s3::Event::Endpoint(v),
        domain::Event::AccessKey(v) => crate::app::message::settings::device_sync_s3::Event::AccessKey(v),
        domain::Event::Secret(v) => crate::app::message::settings::device_sync_s3::Event::Secret(v),
        domain::Event::Root(v) => crate::app::message::settings::device_sync_s3::Event::Root(v),
        domain::Event::Profile(v) => crate::app::message::settings::device_sync_s3::Event::Profile(v),
        domain::Event::Principal(v) => crate::app::message::settings::device_sync_s3::Event::Principal(v),
        domain::Event::Enabled(v) => crate::app::message::settings::device_sync_s3::Event::Enabled(v),
        domain::Event::AutoSync(v) => crate::app::message::settings::device_sync_s3::Event::AutoSync(v),
        domain::Event::ConfirmAutoSync(v) => crate::app::message::settings::device_sync_s3::Event::ConfirmAutoSync(v),
        domain::Event::Save => crate::app::message::settings::device_sync_s3::Event::Save,
        domain::Event::Test => crate::app::message::settings::device_sync_s3::Event::Test,
        domain::Event::Load => crate::app::message::settings::device_sync_s3::Event::Load,
        domain::Event::Close => crate::app::message::settings::device_sync_s3::Event::Close,
        domain::Event::Selection(v) => crate::app::message::settings::device_sync_s3::Event::Selection(v),
        domain::Event::Preview(v) => crate::app::message::settings::device_sync_s3::Event::Preview(v),
        domain::Event::TaskPanel(v) => crate::app::message::settings::device_sync_s3::Event::TaskPanel(v),
    })
}
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let s = &app.apps_catalog.device_sync_s3;
    let editable = !s.busy && !s.task_panel.is_busy() && !s.preview.busy;
    let state = domain::ViewState { busy: s.busy, draft: &s.draft, principal: &s.principal, secret_present: s.secret_present, auto_confirmation: s.auto_confirmation, task_busy: s.task_panel.is_busy(), preview_busy: s.preview.busy, preset: s.preset };
    let selection = super::selection::inventory(app);
    let selection = desktop_settings_memory::system_settings_device_sync::selection::view(selection, &s.selection, editable).map(domain::Event::Selection);
    domain::view(state, selection, s.preview.view().map(domain::Event::Preview), s.task_panel.view().map(domain::Event::TaskPanel)).map(message)
}
