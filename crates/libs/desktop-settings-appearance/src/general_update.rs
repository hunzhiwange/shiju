//! General appearance persistence, with gateway access supplied by the shell.
use config_ui::{DesktopLocale, QuickMessageConfig};
use desktop_config::ConfigAccess;

pub async fn save_locale(access: ConfigAccess, locale: DesktopLocale) -> Result<(), String> {
    access
        .update_system_settings_config_result_async(move |system| system.desktop_locale = locale)
        .await
}
pub async fn save_task_completion_notification(
    access: ConfigAccess,
    enabled: bool,
) -> Result<(), String> {
    access
        .update_system_settings_config_result_async(move |system| {
            system.task_completion_notification = enabled
        })
        .await
}
pub async fn save_quick_messages(
    access: ConfigAccess,
    items: Vec<QuickMessageConfig>,
) -> Result<(), String> {
    access
        .update_system_settings_config_result_async(move |system| system.quick_messages = items)
        .await
}

/// Local tab selection; the caller schedules the gallery refresh effect.
pub fn select_tab(
    current: &mut desktop_ui_controls::settings_navigation::GeneralSettingsTab,
    error: &mut Option<String>,
    tab: desktop_ui_controls::settings_navigation::GeneralSettingsTab,
) -> bool {
    *current = tab;
    if tab == desktop_ui_controls::settings_navigation::GeneralSettingsTab::Themes {
        *error = None;
        true
    } else {
        false
    }
}
/// Reject unknown timezones before changing the displayed value or persisting it.
pub fn select_timezone(current: &mut String, timezone: &str) -> bool {
    if !config_ui::timezone_options().contains(&timezone) {
        return false;
    }
    *current = timezone.to_owned();
    true
}
/// Serialize locale persistence while allowing immediate display updates.
pub fn select_locale(
    current: &mut DesktopLocale,
    selected: &mut bool,
    saving: &mut bool,
    locale: DesktopLocale,
) -> bool {
    *current = locale;
    *selected = true;
    if *saving {
        false
    } else {
        *saving = true;
        true
    }
}
pub fn locale_saved(current: DesktopLocale, saving: &mut bool, saved: DesktopLocale) -> bool {
    *saving = current != saved;
    *saving
}
/// Native startup registration stays a platform effect; its busy guard is local state.
pub fn begin_launch_at_login(busy: &mut bool, supported: bool) -> bool {
    if *busy || !supported {
        return false;
    }
    *busy = true;
    true
}
pub fn launch_at_login_saved(
    current: &mut bool,
    busy: &mut bool,
    result: Result<bool, String>,
    refresh_on_error: bool,
) -> Result<(), String> {
    *busy = false;
    match result {
        Ok(enabled) => {
            *current = enabled;
            Ok(())
        }
        Err(error) => {
            *busy = refresh_on_error;
            Err(error)
        }
    }
}

/// Immediate notification display update and serialized persistence request.
pub fn select_notification(current: &mut bool, saving: &mut bool, enabled: bool) -> bool {
    *current = enabled;
    if *saving {
        false
    } else {
        *saving = true;
        true
    }
}
pub fn notification_saved(current: bool, saving: &mut bool, saved: bool) -> bool {
    *saving = current != saved;
    *saving
}
