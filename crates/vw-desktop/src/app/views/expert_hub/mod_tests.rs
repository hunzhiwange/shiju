//! 专家中心外壳测试，验证所有一级分区与技能标签都能由默认状态构建。
//! 该测试守护模块聚合层，避免新增分区后导航入口与内容区域脱节。

use crate::app::skill_hub::{ExpertHubSection, SkillTab};
use crate::app::{App, Message};

#[test]
fn expert_hub_shell_builds_all_sections_and_skill_tabs() {
    let (mut app, _) = App::new();

    for section in
        [ExpertHubSection::Experts, ExpertHubSection::Connectors, ExpertHubSection::Skills]
    {
        app.expert_hub.section = section;
        let element: iced::Element<'_, Message> = super::view(&app);
        std::hint::black_box(element);
    }

    for tab in [
        SkillTab::Marketplace,
        SkillTab::Installed,
        SkillTab::System,
        SkillTab::Mine,
        SkillTab::Install,
    ] {
        app.expert_hub.skill_tab = tab;
        let element: iced::Element<'_, Message> = super::shiju_skills_view(&app);
        std::hint::black_box(element);
    }
}
