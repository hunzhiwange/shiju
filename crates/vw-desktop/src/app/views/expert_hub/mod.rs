//! 组合专家、连接器与技能三个专家中心分区的桌面视图。

use crate::app::message::SkillHubMessage;
use crate::app::skill_hub::ExpertHubSection;
use crate::app::{App, Message};
use iced::widget::{button, column, container, row, text};
use iced::{Element, Length};

mod detail;
mod experts;
mod install_guide;
mod installed_skills;
mod list;
mod public_skill_detail;
mod public_skills;
pub(crate) mod shiju_apps;
mod styles;

fn section_button(
    label: &'static str,
    section: ExpertHubSection,
    selected: bool,
) -> Element<'static, Message> {
    button(text(label).size(15))
        .on_press(Message::SkillHub(SkillHubMessage::SelectSection(section)))
        .padding([8, 14])
        .style(move |theme, status| styles::selected_tab_style(theme, status, selected))
        .into()
}

/// 构建专家中心一级分区，并为技能分区选择公共或识矩子页面。
pub(crate) fn view(app: &App) -> Element<'_, Message> {
    let section_tabs = row![
        section_button(
            "专家",
            ExpertHubSection::Experts,
            app.expert_hub.section == ExpertHubSection::Experts,
        ),
        section_button(
            "连接器",
            ExpertHubSection::Connectors,
            app.expert_hub.section == ExpertHubSection::Connectors,
        ),
        section_button(
            "技能",
            ExpertHubSection::Skills,
            app.expert_hub.section == ExpertHubSection::Skills,
        ),
    ]
    .spacing(8);

    let body: Element<'_, Message> = match app.expert_hub.section {
        ExpertHubSection::Experts => experts::view(app),
        ExpertHubSection::Connectors => container(text("连接器能力即将接入").size(16))
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .into(),
        ExpertHubSection::Skills if app.expert_hub.public_market.installed_open => {
            installed_skills::view(app)
        }
        ExpertHubSection::Skills if app.expert_hub.public_market.detail.is_some() => {
            public_skill_detail::view(app)
        }
        ExpertHubSection::Skills => public_skills::view(app),
    };

    let base = container(column![section_tabs, body].spacing(24))
        .width(Length::Fill)
        .height(Length::Fill)
        .padding([24, 28])
        .into();

    installed_skills::with_delete_confirmation(app, base)
}

/// 构建兼容的识矩技能入口，复用专家中心技能列表与详情。
pub(crate) fn shiju_skills_view(app: &App) -> Element<'_, Message> {
    let body = if app.expert_hub.detail.is_some() { detail::view(app) } else { list::view(app) };
    let base = container(body).width(Length::Fill).height(Length::Fill).padding([24, 28]).into();
    install_guide::with_uninstall_confirmation(app, base)
}

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
