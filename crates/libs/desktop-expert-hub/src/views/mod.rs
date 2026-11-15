//! 专家中心的局部视图与消息。
pub mod connectors;
pub mod detail;
pub mod experts;
pub mod styles;
use desktop_skill_hub::skill_hub::ExpertHubState;
#[derive(Debug, Clone)]
pub enum Message {
    SkillHub(crate::message::skill_hub::SkillHubMessage),
    Connectors(crate::message::connectors::ConnectorMessage),
    StartBrowserLogin,
    None,
}
/// 视图只借用专家中心状态、语言和主题。
pub struct ViewInput<'a> {
    pub expert_hub: &'a ExpertHubState,
    pub desktop_locale: config_ui::DesktopLocale,
    pub app_theme: &'a iced::Theme,
}

#[cfg(test)] mod detail_tests;
#[cfg(test)] mod connectors_tests;

pub mod install_guide;
pub mod installed_skills;
pub mod knowledge;
#[cfg(test)] mod install_guide_tests;
#[cfg(test)] mod installed_skills_tests;

pub mod list;
pub mod public_skill_detail;
#[cfg(test)] mod list_tests;
#[cfg(test)] mod public_skill_detail_tests;
