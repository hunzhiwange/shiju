//! General settings editor state shared by the settings view and startup backfill.
use desktop_ui_controls::settings_navigation::GeneralSettingsTab;
use iced::widget::text_editor;
pub struct State {
    pub desktop_locale: config_ui::DesktopLocale,
    pub desktop_locale_selected: bool,
    pub desktop_locale_saving: bool,
    pub chat_timezone: String,
    pub task_completion_notification: bool,
    pub task_completion_notification_saving: bool,
    /// 常规配置页内标签
    pub general_settings_tab: GeneralSettingsTab,
    /// 设置侧边栏是否折叠
    pub settings_sidebar_collapsed: bool,
    /// 系统设置分类搜索词
    pub system_settings_query: String,
    /// 对话流权限编辑器内容
    pub dialogue_flow_permission_editor: text_editor::Content,
    pub dialogue_flow_show_reasoning_summary: bool,
    pub dialogue_flow_expand_shell_tool_section: bool,
    pub dialogue_flow_expand_edit_tool_section: bool,
    pub quick_messages: Vec<config_ui::QuickMessageConfig>,
    /// 对话流设置保存消息
    pub dialogue_flow_settings_save_message: Option<String>,
    pub dialogue_flow_settings_translation: Option<desktop_i18n::format::Message>,
}
impl State {
    pub fn new(system: &config_ui::AppSystemSettingsConfig) -> Self {
        let mut quick_messages = system.quick_messages.clone();
        config_ui::upgrade_legacy_quick_message_actions(&mut quick_messages);
        config_ui::append_missing_builtin_general_quick_messages(&mut quick_messages);
        config_ui::append_missing_builtin_git_quick_messages(&mut quick_messages);

        Self {
            desktop_locale: system.desktop_locale,
            desktop_locale_selected: false,
            desktop_locale_saving: false,
            chat_timezone: system.timezone.clone(),
            task_completion_notification: system.task_completion_notification,
            task_completion_notification_saving: false,
            general_settings_tab: GeneralSettingsTab::default(),
            settings_sidebar_collapsed: false,
            system_settings_query: String::new(),
            dialogue_flow_permission_editor: iced::widget::text_editor::Content::with_text("{}"),
            dialogue_flow_show_reasoning_summary: system.dialogue_flow_show_reasoning_summary,
            dialogue_flow_expand_shell_tool_section: system.dialogue_flow_expand_shell_tool_section,
            dialogue_flow_expand_edit_tool_section: system.dialogue_flow_expand_edit_tool_section,
            quick_messages,
            dialogue_flow_settings_save_message: None,
            dialogue_flow_settings_translation: None,
        }
    }
}

impl State {
    pub fn apply_quick_messages(&mut self, quick_messages: Vec<config_ui::QuickMessageConfig>) {
        self.quick_messages = quick_messages;
        config_ui::upgrade_legacy_quick_message_actions(&mut self.quick_messages);
        config_ui::append_missing_builtin_general_quick_messages(&mut self.quick_messages);
        config_ui::append_missing_builtin_git_quick_messages(&mut self.quick_messages);
    }
}
