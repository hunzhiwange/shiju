//! 本机启动项、关闭偏好与原生托盘。系统级行为不随网关配置同步到其它设备。

#[cfg(not(target_arch = "wasm32"))]
mod native;

#[cfg(not(target_arch = "wasm32"))]
pub(crate) use native::*;

#[cfg(target_arch = "wasm32")]
pub(crate) fn launch_at_login_enabled() -> bool {
    false
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn read_launch_at_login() -> Result<bool, String> {
    Err("浏览器不支持开机启动".into())
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn close_to_tray_enabled() -> bool {
    false
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn set_launch_at_login(_: bool) -> Result<bool, String> {
    Err("浏览器不支持开机启动".into())
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn save_close_to_tray(_: bool) -> Result<(), String> {
    Err("浏览器不支持系统托盘".into())
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn ensure_tray(_: vw_config_types::ui::DesktopLocale) -> Result<(), String> {
    Err("浏览器不支持系统托盘".into())
}
#[cfg(target_arch = "wasm32")]
pub(crate) fn remove_tray() {}
#[cfg(target_arch = "wasm32")]
pub(crate) fn poll_tray() -> Vec<crate::app::message::GeneralSettingsMessage> {
    Vec::new()
}

mod window;
#[cfg(not(target_os = "macos"))]
pub(crate) use window::hide_main_window;

pub(crate) mod tray_switch;
