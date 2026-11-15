//! 组合专家、连接器、技能与资料知识库的专家中心桌面视图。

use crate::app::message::{ShijuAppsMessage, SkillHubMessage};
use crate::app::skill_hub::ExpertHubSection;
use crate::app::{App, Message, Screen};
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
pub(crate) mod shiju_apps;
mod styles;
mod upgrade_actions;

fn section_tabs(app: &App) -> Element<'static, Message> {
    use desktop_expert_hub::views::shell::{self, Navigation};
    let screen = match app.screen {
        Screen::ExpertHub => shell::Screen::ExpertHub,
        Screen::ShijuSkills => shell::Screen::ShijuSkills,
        Screen::ShijuApps => shell::Screen::ShijuApps,
        _ => shell::Screen::Other,
    };
    shell::section_tabs(app.desktop_locale, app.expert_hub.section, screen, |message| match message {
        Navigation::SelectSection(section) => Message::SkillHub(SkillHubMessage::SelectSection(section)),
        Navigation::OpenShijuSkills => Message::SkillHub(SkillHubMessage::OpenShijuSkills),
        Navigation::OpenShijuApps => Message::ShijuApps(ShijuAppsMessage::Open),
    })
}

/// 构建专家中心一级分区，并为技能分区选择公共或识矩子页面。
pub(crate) fn view(app: &App) -> Element<'_, Message> {
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

    let base = desktop_expert_hub::views::shell::view(section_tabs(app), body);

    let base = installed_skills::with_delete_confirmation(app, base);
    connectors::with_binding_modal(app, base)
}

/// 构建兼容的识矩技能入口，复用专家中心技能列表与详情。
pub(crate) fn shiju_skills_view(app: &App) -> Element<'_, Message> {
    let body = if app.expert_hub.detail.is_some() { detail::view(app) } else { list::view(app) };
    let base = desktop_expert_hub::views::shell::view(section_tabs(app), body);
    install_guide::with_uninstall_confirmation(app, base)
}

#[cfg(test)]
mod connectors_tests;
#[cfg(test)]
mod detail_tests;
#[cfg(test)]
mod experts_tests;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod mod_tests;
#[cfg(test)]
mod public_skill_detail_tests;
#[cfg(test)]
mod public_skills_tests;
#[cfg(test)]
mod shiju_knowledge_view_tests;

#[cfg(test)]
mod shiju_knowledge_reader_tests;
