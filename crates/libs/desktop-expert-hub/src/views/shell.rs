//! 专家中心共享导航与页面布局。
use desktop_ui_controls::system_settings_common::{settings_tab_button, settings_tab_row};
use desktop_i18n::I18n;
use desktop_skill_hub::skill_hub::ExpertHubSection;
use iced::{Element, Length};
use iced::widget::{column, container};
pub use crate::message::HubPage as Screen;
#[derive(Debug, Clone)]
pub enum Navigation { SelectSection(ExpertHubSection), OpenShijuSkills, OpenShijuApps }
/// 在专家中心各分区、识矩技能和识矩应用之间提供一致的顶部导航。
pub fn section_tabs<M: Clone + 'static>(locale: config_ui::DesktopLocale, selected_section: ExpertHubSection, screen: Screen, map: fn(Navigation) -> M) -> Element<'static, M> {
    let i18n = I18n::new(locale);
    let sections = [
        (i18n.__("专家"), ExpertHubSection::Experts),
        (i18n.__("连接器"), ExpertHubSection::Connectors),
        (i18n.__("技能"), ExpertHubSection::Skills),
        (i18n.__("识矩知识库"), ExpertHubSection::Knowledge),
    ];
    settings_tab_row(sections.into_iter().map(|(label, section)| {
        settings_tab_button(
            label,
            screen == Screen::ExpertHub && selected_section == section,
            Some(map(Navigation::SelectSection(section))),
        )
        .into()
    }))
    .push(settings_tab_button(
        i18n.__("识矩技能"),
        screen == Screen::ShijuSkills,
        Some(map(Navigation::OpenShijuSkills)),
    ))
    .push(settings_tab_button(
        i18n.__("识矩应用"),
        screen == Screen::ShijuApps,
        Some(map(Navigation::OpenShijuApps)),
    ))
    .wrap()
    .into()
}


pub fn view<'a, M: Clone + 'static>(tabs: Element<'a, M>, body: Element<'a, M>) -> Element<'a, M> {
    container(column![tabs, body].spacing(24))
        .width(Length::Fill).height(Length::Fill).padding([24, 28]).into()
}
