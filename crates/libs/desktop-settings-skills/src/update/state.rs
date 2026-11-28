use crate::types::*;
#[derive(Debug, Clone)]
pub struct State {
    pub sync: crate::sync::State,
    /// 是否让委托代理默认加载项目及父级技能
    pub auto_load_project_skills: bool,
    /// 提示注入模式
    pub prompt_injection_mode: config_skills::SkillsPromptInjectionMode,
    /// 当前激活的技能设置页签
    pub active_tab: SkillsSettingsTab,
    /// 技能搜索关键词
    pub query: String,
    /// 当前选中的目录范围
    pub directory_scope: SkillsDirectoryScope,
    /// 技能目录是否正在通过 gateway 加载
    pub loading: bool,
    pub local_import_busy: bool,
    /// 当前展示的技能目录项
    pub catalog: Vec<SkillsCatalogItem>,
    pub expert_skill_ids: std::collections::BTreeSet<String>,
    /// 当前展示的插件目录项
    pub plugin_catalog: Vec<PluginCatalogItem>,
    pub plugin_install: Option<crate::plugin_state::PluginInstallForm>,
    pub plugin_operation_id: u64,
    /// 插件目录是否正在通过 gateway 加载
    pub plugin_loading: bool,
    /// 当前选中的插件 ID
    pub selected_plugin_id: Option<String>,
    /// 插件操作状态提示
    pub plugin_status_message: Option<String>,
    pub plugin_status_message_translation: Option<desktop_i18n::format::Message>,
    /// 当前插件状态提示是否为错误
    pub plugin_status_is_error: bool,
    /// 当前选中的技能 ID
    pub selected_skill_id: Option<String>,
    /// 当前选中的技能详情
    pub selected_skill_detail: Option<SkillsSelectedDetail>,
    /// 技能详情是否正在加载
    pub detail_loading: bool,
    /// 技能详情错误
    pub detail_error: Option<String>,
    /// 技能操作状态提示
    pub status_message: Option<String>,
    pub status_message_translation: Option<desktop_i18n::format::Message>,
    /// 当前状态提示是否为错误
    pub status_is_error: bool,
    /// 是否显示帮助对话框
    pub show_help_modal: bool,
    /// 保存错误信息
    pub save_error: Option<String>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            sync: Default::default(),
            auto_load_project_skills: true,
            prompt_injection_mode: config_skills::SkillsPromptInjectionMode::Compact,
            active_tab: SkillsSettingsTab::Skills,
            query: String::new(),
            directory_scope: SkillsDirectoryScope::Project,
            loading: false,
            local_import_busy: false,
            catalog: Vec::new(),
            expert_skill_ids: Default::default(),
            plugin_catalog: Vec::new(),
            plugin_install: None,
            plugin_operation_id: 0,
            plugin_loading: false,
            selected_plugin_id: None,
            plugin_status_message: None,
            plugin_status_message_translation: None,
            plugin_status_is_error: false,
            selected_skill_id: None,
            selected_skill_detail: None,
            detail_loading: false,
            detail_error: None,
            status_message: None,
            status_message_translation: None,
            status_is_error: false,
            show_help_modal: false,
            save_error: None,
        }
    }
}

pub fn build_skills_settings(
    config: &config_skills::SkillsConfig,
) -> crate::update::state::State {
    crate::update::state::State {
        auto_load_project_skills: config.auto_load_project_skills,
        prompt_injection_mode: config.prompt_injection_mode,
        ..Default::default()
    }
}
