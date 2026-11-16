#[derive(Debug, Clone)]
pub enum GeneralSettingsMessage {
    LocaleSelected(config_ui::DesktopLocale),
    LocaleSaved(config_ui::DesktopLocale, Result<(), String>),
    TabSelected(desktop_ui_controls::settings_navigation::GeneralSettingsTab),
    TimezoneSelected(String),
    TaskCompletionNotificationToggled(bool),
    TaskCompletionNotificationSaved(bool, Result<(), String>),
    TestTaskCompletionNotification,
    TaskCompletionNotificationSent(Result<(), String>),
    TaskCompletionNotificationPermissionChecked(Result<(), String>),
    LaunchAtLoginToggled(bool),
    LaunchAtLoginSaved(Result<bool, String>),
    LaunchAtLoginRefreshed(Result<bool, String>),
    CloseToTrayToggled(bool),
    #[cfg(target_os = "macos")]
    ProviderLinkRejected(u64),
    #[cfg(target_os = "macos")]
    McpLinkPreview(u64),
    #[cfg(target_os = "macos")]
    McpLinkRejected(u64),
    #[cfg(target_os = "macos")]
    SkillLinkPreview(u64),
    #[cfg(target_os = "macos")]
    SkillLinkRejected(u64),
    #[cfg(target_os = "macos")]
    PromptLinkPreview(u64),
    #[cfg(target_os = "macos")]
    PromptLinkRejected(u64),
    #[cfg(target_os = "macos")]
    InstanceTick,
    TrayTick,
    TrayProviderSwitch(String),
    TrayProviderSwitched {
        scope: desktop_provider_apps::Scope,
        generation: u64,
        id: String,
        result: Result<(), String>,
    },
    TrayShow,
    TrayOpenApp(vw_api_app_identity::AppId),
    TrayQuit,
}
