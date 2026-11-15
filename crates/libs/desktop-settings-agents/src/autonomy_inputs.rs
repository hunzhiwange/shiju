/// 仅借用此页面实际读取的数据与帮助渲染入口。
pub struct View<'a> {
    pub autonomy_settings: &'a desktop_settings_security::app_init::state::AutonomySettingsState,
    pub available_tools: &'a [String],
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a iced::Theme,
    pub last_copied_code_hash: Option<u64>,
    pub help_text: super::autonomy_help::Renderer,
}
