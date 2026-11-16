//! Skill directory inputs and Chinese text shaping in the profile modal.
use super::profile_update::{ProfileFixture, dispatch};
use crate::message::AgentsMessage;
use crate::state::SkillsCatalogItem;
use desktop_settings_skills::types::SkillsCatalogKind;

fn skill(id: &str, enabled: bool, source: &str) -> SkillsCatalogItem {
    SkillsCatalogItem {
        id: id.to_string(),
        title: id.to_string(),
        description: format!("{id} description"),
        kind: SkillsCatalogKind::System,
        resource_count: 0,
        installed: true,
        enabled,
        source: source.to_string(),
        source_path: None,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_skill_modes_render_chinese_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut app = ProfileFixture::default();
        app.agents_settings.config_ready = true;
        app.project_path = Some("/应用项目".into());
        app.recent_projects = vec!["/技能目录甲".into(), "/技能目录乙".into()];
        dispatch(&mut app, AgentsMessage::ProfileOpen("main".into()));
        dispatch(&mut app, AgentsMessage::SkillDirectorySelected(Some("/技能目录乙".into())));
        app.agents_settings.active_detail_tab = crate::state::AGENT_DETAIL_SKILLS_TAB.into();
        app.agents_settings.skill_catalog =
            vec![skill("中文技能", true, "global"), skill("disabled", false, "global")];
        let draft = app.agents_settings.profile_modal.as_mut().unwrap();
        draft.allowed_skills = vec!["中文技能".into()];
        if name == "dark" {
            draft.defaults.skills =
                config_agent_profile::AgentDefault::Fixed(vec!["中文技能".into()]);
        }
        let mut element = super::profile_render::overlay(
            &app,
            iced::widget::container(iced::widget::text("背景"))
                .width(iced::Length::Fill)
                .height(iced::Length::Fill)
                .into(),
        );
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .unwrap();
        let bounds = Size::new(1100.0, 900.0);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        assert_eq!(node.bounds().size(), bounds);
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style { text_color: theme.palette().text },
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &Rectangle::with_size(bounds),
        );
        let pixels = renderer.screenshot(Size::new(1100, 900), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        image::save_buffer(
            format!("/tmp/shiju-task4-skill-modes-{name}.png"),
            &pixels,
            1100,
            900,
            image::ColorType::Rgba8,
        )
        .unwrap();
    }
}
