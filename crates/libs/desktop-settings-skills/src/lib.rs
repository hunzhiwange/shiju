pub mod browser;
pub mod catalog;
pub mod prompts;
pub mod types;
#[derive(Clone, Debug)]
pub enum Event {
    PluginsSetEnabledRequested { plugin_id: String, enabled: bool },
    PluginsDeleteRequested(String), PluginsSelected(String), PluginsUpdateOpen(String), PluginsRefresh, PluginsInstallOpen,
    None,
    CopyText(String),
    AppendText(String),
    OpenUrl(String),
    Selection(String, usize, usize, Option<String>),
    SkillsDirectoryScopeChanged(types::SkillsDirectoryScope),
    SkillsRefresh,
    SkillsInstallBuiltInRequested(String),
    SkillsSetEnabledRequested { skill_id: String, enabled: bool },
    SkillsDeleteRequested(String),
    SkillsDetailClosed,
    SkillsQueryChanged(String),
    SkillsProjectSelected(String),
    SkillsDetailRequested(String),
}
#[derive(Clone, Copy)]
pub struct BrowserState<'a> {
    pub directory_provider: config_skills::SkillsDirectoryProvider,
    pub query: &'a String,
    pub directory_scope: types::SkillsDirectoryScope,
    pub loading: bool,
    pub catalog: &'a Vec<types::SkillsCatalogItem>,
    pub selected_skill_id: &'a Option<String>,
    pub selected_skill_detail: &'a Option<types::SkillsSelectedDetail>,
    pub detail_loading: bool,
    pub detail_error: &'a Option<String>,
    pub status_message: &'a Option<String>,
    pub status_message_translation: &'a Option<desktop_i18n::format::Message>,
    pub status_is_error: bool,
}
#[derive(Clone, Copy)]
pub struct BrowserContext<'a> {
    pub help: HelpRenderer<Event>,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a iced::Theme,
    pub project_path: &'a Option<String>,
    pub recent_projects: &'a Vec<String>,
    pub skills_settings: BrowserState<'a>,
}
pub fn modal_card_style(theme: &iced::Theme) -> iced::widget::container::Style {
    let mut style = desktop_ui_controls::system_settings_common::settings_modal_card_style(theme);
    if let Some(iced::Background::Color(color)) = &mut style.background {
        color.a = 1.0;
    }
    style
}
pub mod backup_modal;
pub mod bindings_view;
pub mod claude_view;
pub mod deeplink;

pub mod page;
pub mod third_party;
pub fn plugin_dependency_summary(
    checks: &[gateway_desktop_settings::DesktopPluginDependencyDto],
) -> &'static str {
    if checks.iter().any(|c| {
        !matches!(
            c.status.as_str(),
            "present"
                | "missing_host"
                | "missing_package"
                | "unverified"
                | "scope_excluded"
                | "invalid"
        )
    }) {
        "未知检查结果，需重新审计"
    } else if checks.iter().any(|c| matches!(c.status.as_str(), "missing_host" | "missing_package"))
    {
        "缺少运行依赖"
    } else if checks.iter().any(|c| c.status == "invalid") {
        "依赖声明无效"
    } else if checks.iter().any(|c| c.status == "unverified") {
        "运行依赖待验证"
    } else if checks.is_empty() {
        "未声明依赖，运行待验证"
    } else if checks.iter().any(|c| c.status == "scope_excluded") {
        "含范围外托管能力"
    } else {
        "依赖文件已检测，运行待验证"
    }
}

#[cfg(test)]
mod catalog_tests;

#[cfg(test)]
mod browser_tests;

/// Host renderer preserves selectable help and clipboard actions without coupling to chat views.
pub type HelpRenderer<E> = for<'a, 'theme> fn(
    desktop_i18n::I18n,
    Option<&'theme iced::Theme>,
    &'a str,
    iced::widget::Text<'a>,
    std::borrow::Cow<'a, str>,
) -> iced::widget::Row<'a, E>;

mod browser_detail;
mod browser_discovery;
mod browser_widgets;

pub mod import_modal;
pub mod install_view;
pub mod jobs_view;
pub mod market_view;
pub mod repo_modal;
pub mod storage_modal;
pub mod updates_view;
pub mod plugin_state;
pub mod plugin_install;
pub mod plugin_install_modal;

pub mod plugins;
pub mod help;
#[cfg(test)] mod help_tests;

pub mod update;

pub mod save_feedback;

#[cfg(test)]
mod plugin_compat_t55_tests;

#[cfg(test)]
mod plugin_compat_t56_tests;

#[cfg(test)]
mod desktop_split_02_tests;

#[cfg(test)]
mod desktop_split_14_tests;

#[cfg(test)]
mod host_module_23_tests;

#[cfg(test)]
mod host_module_36_tests;

impl desktop_ui_controls::selectable_text::markdown_selection::SelectionMessage for Event {
    fn none() -> Self { Self::None }
    fn copy(text: String) -> Self { Self::CopyText(text) }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
        match effect { TextContextEffect::Copy(text) => Self::CopyText(text), TextContextEffect::Append(text) => Self::AppendText(text), TextContextEffect::OpenUrl(url) => Self::OpenUrl(url) }
    }
    fn selection(session: String, index: usize, text_index: usize, text: Option<String>) -> Self { Self::Selection(session, index, text_index, text) }
}
