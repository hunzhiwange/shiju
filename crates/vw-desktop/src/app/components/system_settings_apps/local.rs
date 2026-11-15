pub use desktop_settings_apps::system_settings_apps::local::*;
pub(crate) fn available() -> bool {
    desktop_settings_apps::system_settings_apps::local::available(crate::app::config::gateway_client().ok().as_ref())
}
