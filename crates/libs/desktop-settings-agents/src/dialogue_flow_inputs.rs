/// 仅借用此页面实际读取的数据与帮助渲染入口。
pub struct View<'a> {
    pub dialogue_flow_show_reasoning_summary: bool,
    pub dialogue_flow_expand_shell_tool_section: bool,
    pub dialogue_flow_expand_edit_tool_section: bool,
    pub dialogue_flow_settings_save_message: &'a Option<String>,
    pub dialogue_flow_settings_translation: &'a Option<desktop_i18n::format::Message>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a iced::Theme,
    pub help_text: super::dialogue_flow_help::Renderer,
}
