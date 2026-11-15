//! Map native events and inject the desktop configuration directory.
use crate::app::message::GeneralSettingsMessage;
use desktop_platform::native_lifecycle as native;
pub(crate) use native::{ensure_tray, remove_tray, refresh_tray_locale, refresh_tray_state,
    launch_at_login_enabled, read_launch_at_login, set_launch_at_login};
fn preference_path() -> Result<std::path::PathBuf, String> {
    crate::app::project_dirs()
        .map(|dirs| dirs.config_dir().join("desktop-lifecycle.json"))
        .ok_or_else(|| "无法定位本机配置目录".into())
}
pub(crate) fn close_to_tray_enabled() -> bool {
    native::close_to_tray_enabled(preference_path())
}
pub(crate) fn save_close_to_tray(enabled: bool) -> Result<(), String> {
    native::save_close_to_tray(preference_path(), enabled)
}
pub(crate) fn poll_tray() -> Vec<GeneralSettingsMessage> {
    native::poll_tray().into_iter().map(|event| match event {
        native::TrayEvent::Show => GeneralSettingsMessage::TrayShow,
        native::TrayEvent::Quit => GeneralSettingsMessage::TrayQuit,
        native::TrayEvent::OpenApp(app) => GeneralSettingsMessage::TrayOpenApp(app),
        native::TrayEvent::ProviderSwitch(id) => GeneralSettingsMessage::TrayProviderSwitch(id),
    }).collect()
}
pub(crate) fn refresh_tray_providers(app: &crate::app::App) {
    let state = &app.app_providers;
    let (signature, enabled, rows) = desktop_provider_apps::tray_switch::menu(
        state.scope.as_ref(), state.generation, state.snapshot.as_ref(),
        app.shutdown_requested, state.loading, state.tray_switch.is_some(),
    );
    native::refresh_tray_providers(signature, enabled, state.generation, rows);
}
