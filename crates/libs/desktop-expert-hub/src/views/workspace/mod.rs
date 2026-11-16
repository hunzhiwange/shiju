//! 组合专家、连接器、技能与资料知识库的专家中心桌面视图。

pub use crate::message::skill_hub::SkillHubMessage;
use desktop_skill_hub::skill_hub::ExpertHubSection;
type WorkspaceInput<'a> = ViewInput<'a>;
use iced::Element;

mod connectors;
mod detail;
mod experts;
mod install_guide;
mod installed_skills;
mod knowledge;
mod list;
mod public_skill_detail;
mod public_skills;
pub mod shiju_apps;
mod styles;
mod upgrade_actions;

fn section_tabs(app: &WorkspaceInput) -> Element<'static, Message> {
    use crate::views::shell::{self, Navigation};
    let screen = match app.screen {
        Screen::ExpertHub => shell::Screen::ExpertHub,
        Screen::ShijuSkills => shell::Screen::ShijuSkills,
        Screen::ShijuApps => shell::Screen::ShijuApps,
        _ => shell::Screen::Other,
    };
    shell::section_tabs(app.desktop_locale, app.expert_hub.section, screen, |message| match message
    {
        Navigation::SelectSection(section) => {
            Message::SkillHub(SkillHubMessage::SelectSection(section))
        }
        Navigation::OpenShijuSkills => Message::SkillHub(SkillHubMessage::OpenShijuSkills),
        Navigation::OpenShijuApps => Message::ShijuApps(ShijuAppsMessage::Open),
    })
}

/// 构建专家中心一级分区，并为技能分区选择公共或识矩子页面。
pub fn view<'a>(app: &WorkspaceInput<'a>) -> Element<'a, Message> {
    let body: Element<'_, Message> = match app.expert_hub.section {
        ExpertHubSection::Experts => experts::view(app),
        ExpertHubSection::Connectors => connectors::view(app),
        ExpertHubSection::Knowledge => knowledge::view(app),
        ExpertHubSection::Skills if app.expert_hub.public_market.installed_open => {
            installed_skills::view(app)
        }
        ExpertHubSection::Skills if app.expert_hub.public_market.detail.is_some() => {
            public_skill_detail::view(app)
        }
        ExpertHubSection::Skills => public_skills::view(app),
    };

    let base = crate::views::shell::view(section_tabs(app), body);

    let base = installed_skills::with_delete_confirmation(app, base);
    connectors::with_binding_modal(app, base)
}

/// 构建兼容的识矩技能入口，复用专家中心技能列表与详情。
pub fn shiju_skills_view<'a>(app: &WorkspaceInput<'a>) -> Element<'a, Message> {
    let body = if app.expert_hub.detail.is_some() { detail::view(app) } else { list::view(app) };
    let base = crate::views::shell::view(section_tabs(app), body);
    install_guide::with_uninstall_confirmation(app, base)
}

#[derive(Debug, Clone, Copy)]
pub enum Screen {
    ExpertHub,
    ShijuSkills,
    ShijuApps,
    Other,
}
#[derive(Debug, Clone)]
pub enum AccountMessage {
    StartBrowserLogin,
}
#[derive(Debug, Clone)]
pub enum ViewMessage {
    OpenUrlExternal(String),
}
#[derive(Debug, Clone)]
pub enum Message {
    SkillHub(crate::message::skill_hub::SkillHubMessage),
    ShijuApps(ShijuAppsMessage),
    Connectors(crate::message::connectors::ConnectorMessage),
    Account(AccountMessage),
    View(ViewMessage),
    CopyCode(String),
    AppendToInput(String),
    Selection { session_id: String, message_idx: usize, text_idx: usize, text: Option<String> },
    None,
}
pub use desktop_apps_ui::shiju_apps::ShijuAppsMessage;
pub struct ViewInput<'a> {
    pub expert_hub: &'a desktop_skill_hub::skill_hub::ExpertHubState,
    pub shiju_apps: &'a desktop_app_launch::ShijuAppsState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a iced::Theme,
    pub project_path: &'a Option<String>,
    pub recent_projects: &'a [String],
    pub screen: Screen,
    pub has_project_chat: bool,
}

#[cfg(test)]
mod connectors_tests;
#[cfg(test)]
mod connectors_ui_refresh_39_tests;
#[cfg(test)]
mod detail_tests;
#[cfg(test)]
mod experts_tests;
#[cfg(test)]
mod fixture;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod mod_tests;
#[cfg(test)]
mod public_skill_detail_tests;
#[cfg(test)]
mod public_skills_tests;

pub use crate::message::skill_hub;

#[cfg(test)]
mod public_project_tests;

impl desktop_chat_view::message_view::markdown_selection::SelectionMessage for Message {
    fn none() -> Self {
        Self::None
    }
    fn copy(text: String) -> Self {
        Self::CopyCode(text)
    }
    fn context(effect: desktop_ui_controls::text_editor_context_menu::TextContextEffect) -> Self {
        use desktop_ui_controls::text_editor_context_menu::TextContextEffect;
        match effect {
            TextContextEffect::Copy(text) => Self::CopyCode(text),
            TextContextEffect::Append(text) => Self::AppendToInput(text),
            TextContextEffect::OpenUrl(url) => Self::View(ViewMessage::OpenUrlExternal(url)),
        }
    }
    fn selection(
        session_id: String,
        message_idx: usize,
        text_idx: usize,
        text: Option<String>,
    ) -> Self {
        Self::Selection { session_id, message_idx, text_idx, text }
    }
}
