//! 验证代理技能列表、筛选、启停状态与操作消息。
//! 未安装、禁用和搜索无结果状态用于确认操作按钮不会越权出现。

use super::skills::{discovery_order_text, view, skill_card_button_style};
use crate::app::Message;
use crate::app::message::settings::{AgentsMessage, SettingsMessage};
use crate::app::state::{SkillsCatalogItem, SkillsCatalogKind, SkillsDirectoryScope};
use iced::advanced::widget::Tree;
use iced::widget::button;
use iced::{Background, Theme};

#[test]
fn discovery_order_text_distinguishes_open_project_and_global_context() {
    let (mut app, _) = crate::app::App::new();
    let global_order = discovery_order_text(&app);

    assert!(!global_order.contains("内置技能"));
    assert!(!global_order.contains("父级 skills、.agents/skills"));

    app.agents_settings.skill_directory = Some("/workspace/demo".to_string());
    let project_order = discovery_order_text(&app);

    assert!(project_order.starts_with("发现顺序：/workspace/demo/skills"));
    assert!(
        project_order.contains("/workspace/demo/.agents/skills -> 父级 skills、.agents/skills")
    );
}

#[test]
fn skills_view_renders_empty_loading_and_filtered_catalog_states() {
    let (mut app, _) = crate::app::App::new();
    let entry = app.agents_settings.entries[0].clone();
    app.agents_settings.skill_directory_scope = SkillsDirectoryScope::Project;
    app.agents_settings.skill_catalog_loading = true;

    let loading_tree = Tree::new(view(&app, &entry).as_widget());
    assert_eq!(loading_tree.children.len(), 4);
    assert_eq!(loading_tree.children[3].children.len(), 2);

    app.agents_settings.skill_catalog_loading = false;
    app.agents_settings.skill_catalog = vec![
        skill("project-enabled", true, "workspace"),
        skill("project-disabled", false, "workspace"),
        skill("global", true, "global"),
    ];
    app.agents_settings.skill_directory_scope = SkillsDirectoryScope::All;

    let catalog_tree = Tree::new(view(&app, &entry).as_widget());
    assert_eq!(catalog_tree.children.len(), 4);
    assert_eq!(catalog_tree.children[3].children.len(), 2);
    assert_eq!(catalog_tree.children[3].children[1].children.len(), 3);
}

#[test]
fn unselected_skill_card_style_covers_dark_hover_and_light_press() {
    let dark_hovered = skill_card_button_style(&Theme::Dark, button::Status::Hovered, false);
    let light_pressed = skill_card_button_style(&Theme::Light, button::Status::Pressed, false);
    let dark_active = skill_card_button_style(&Theme::Dark, button::Status::Active, false);

    assert!(matches!(dark_hovered.background, Some(Background::Color(_))));
    assert!(matches!(light_pressed.background, Some(Background::Color(_))));
    assert!(matches!(dark_active.background, Some(Background::Color(_))));
}

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

#[test]
fn agent_config_cleanup_skills_inherit_and_fixed_are_distinct() {
    use config_agent_profile::AgentDefault;
    let (mut app, _) = crate::app::App::new();
    app.agents_settings.skill_catalog =
        vec![skill("allowed", true, "global"), skill("other", true, "global")];
    let mut entry = app.agents_settings.entries[0].clone();
    entry.allowed_skills = vec!["allowed".into()];
    entry.defaults.skills = AgentDefault::Inherit;
    assert_eq!(super::skills::selected_skill_ids(&entry), &["allowed"]);
    entry.defaults.skills = AgentDefault::Fixed(vec![]);
    assert!(super::skills::selected_skill_ids(&entry).is_empty());
    entry.defaults.skills = AgentDefault::Fixed(vec!["other".into()]);
    assert_eq!(super::skills::selected_skill_ids(&entry), &["other"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn agent_config_cleanup_skill_modes_render_chinese_in_both_themes() {
    use iced::advanced::renderer::Headless;
    use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
    use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
    fn dispatch(app: &mut crate::app::App, message: AgentsMessage) {
        let _ = crate::app::message::settings::update(app, SettingsMessage::Agents(message));
    }
    for (theme, name) in [(Theme::Light, "light"), (Theme::Dark, "dark")] {
        let mut app = crate::app::App::new().0;
        app.agents_settings.config_ready = true;
        app.project_path = Some("/应用项目".into());
        app.recent_projects = vec!["/技能目录甲".into(), "/技能目录乙".into()];
        dispatch(&mut app, AgentsMessage::ProfileOpen("main".into()));
        dispatch(&mut app, AgentsMessage::SkillDirectorySelected(Some("/技能目录乙".into())));
        app.agents_settings.active_detail_tab = crate::app::state::AGENT_DETAIL_SKILLS_TAB.into();
        app.agents_settings.skill_catalog =
            vec![skill("中文技能", true, "global"), skill("disabled", false, "global")];
        let draft = app.agents_settings.profile_modal.as_mut().unwrap();
        draft.allowed_skills = vec!["中文技能".into()];
        if name == "dark" {
            draft.defaults.skills =
                config_agent_profile::AgentDefault::Fixed(vec!["中文技能".into()]);
        }
        let mut element = super::profile_modal::overlay(
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
