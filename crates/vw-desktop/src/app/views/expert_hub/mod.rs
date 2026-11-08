//! 组合专家、连接器、技能与资料知识库的专家中心桌面视图。

use crate::app::components::system_settings_common::{settings_tab_button, settings_tab_row};
use crate::app::i18n::I18n;
use crate::app::message::{ShijuAppsMessage, SkillHubMessage};
use crate::app::skill_hub::ExpertHubSection;
use crate::app::{App, Message, Screen};
use iced::widget::{column, container};
use iced::{Element, Length};

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

/// 在专家中心各分区、识矩技能和识矩应用之间提供一致的顶部导航。
fn section_tabs(app: &App) -> Element<'static, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let sections = [
        (i18n.__("专家"), ExpertHubSection::Experts),
        (i18n.__("连接器"), ExpertHubSection::Connectors),
        (i18n.__("技能"), ExpertHubSection::Skills),
        (i18n.__("识矩知识库"), ExpertHubSection::Knowledge),
    ];
    settings_tab_row(sections.into_iter().map(|(label, section)| {
        settings_tab_button(
            label,
            app.screen == Screen::ExpertHub && app.expert_hub.section == section,
            Some(Message::SkillHub(SkillHubMessage::SelectSection(section))),
        )
        .into()
    }))
    .push(settings_tab_button(
        i18n.__("识矩技能"),
        app.screen == Screen::ShijuSkills,
        Some(Message::SkillHub(SkillHubMessage::OpenShijuSkills)),
    ))
    .push(settings_tab_button(
        i18n.__("识矩应用"),
        app.screen == Screen::ShijuApps,
        Some(Message::ShijuApps(ShijuAppsMessage::Open)),
    ))
    .wrap()
    .into()
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

    let base = container(column![section_tabs(app), body].spacing(24))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([24, 28])
        .into();

    let base = installed_skills::with_delete_confirmation(app, base);
    connectors::with_binding_modal(app, base)
}

/// 构建兼容的识矩技能入口，复用专家中心技能列表与详情。
pub(crate) fn shiju_skills_view(app: &App) -> Element<'_, Message> {
    let body = if app.expert_hub.detail.is_some() { detail::view(app) } else { list::view(app) };
    let base = container(column![section_tabs(app), body].spacing(24))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([24, 28])
        .into();
    install_guide::with_uninstall_confirmation(app, base)
}

#[cfg(test)]
mod connectors_tests;
#[cfg(test)]
mod detail_tests;
#[cfg(test)]
mod experts_tests;
#[cfg(test)]
mod install_guide_tests;
#[cfg(test)]
mod installed_skills_tests;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod mod_tests;
#[cfg(test)]
mod public_skill_detail_tests;
#[cfg(test)]
mod public_skills_tests;
#[cfg(test)]
mod shiju_apps_tests;
#[cfg(test)]
mod shiju_knowledge_view_tests;

#[cfg(test)]
mod shiju_knowledge_reader_tests;
