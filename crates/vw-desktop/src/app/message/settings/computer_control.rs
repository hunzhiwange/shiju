//! Desktop effects for computer control.
use crate::app::{App, Message};
use crate::app::message::SettingsMessage;
use crate::app::state::ToastKind;
use iced::Task;
pub(crate) use desktop_settings_security::computer_control_update::Event;
#[cfg(target_os = "macos")]
#[path = "computer_control_permission_guide.rs"]
mod permission_guide;

pub(crate) fn message(event: Event) -> Message { Message::Settings(SettingsMessage::ComputerControl(event)) }
pub(crate) fn update(app: &mut App, event: Event) -> Task<Message> {
    let endpoint = crate::app::config::gateway_client_endpoint();
    #[cfg(target_os = "macos")]
    if let Event::OpenNativePermissionGuide { screen_recording } = &event {
        if !endpoint.is_loopback() {
            return app.show_toast(ToastKind::Error, "请在运行网关的 Mac 上打开系统设置");
        }
        if let Err(error) = permission_guide::show(*screen_recording) {
            return app.show_toast(ToastKind::Error, error);
        }
        return Task::none();
    }
    #[cfg(target_os = "macos")]
    if matches!(&event, Event::Refresh | Event::ManageClose) {
        permission_guide::close();
    }
    let (task, notice) = desktop_settings_security::computer_control_update::update(
        &mut app.computer_control_settings, &endpoint,
        app.session_state.active_session_id.as_deref(), app.navigation_tabs.project_path.as_deref(), event);
    let toast = match notice {
        Some(desktop_settings_security::computer_control_update::Notice::Error(value)) => app.show_toast(ToastKind::Error, value),
        Some(desktop_settings_security::computer_control_update::Notice::Success(value)) => app.show_toast(ToastKind::Success, value),
        None => Task::none(),
    };
    Task::batch([task.map(message), toast])
}
