//! 公共技能详情测试，覆盖 Markdown 内容及按安装范围生成的平台动作。
//! 项目上下文缺失或安装正忙时，操作可用性必须收敛到安全状态。

use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
use vw_gateway_client::{
    DesktopSkillCatalogEntryDto, SkillMarketCoordinateDto, SkillMarketSkillDetailDto,
    SkillMarketSkillDto, SkillMarketSkillFileDto, SkillMarketSkillLocatorDto,
    SkillMarketSkillSetDetailDto, SkillMarketSkillSetDto,
};

use super::public_skill_detail::{
    detail_project_action_message, detail_scope_action_message, detail_target_action_message,
    installed_in_catalog, view,
};
use crate::app::message::skill_hub::{PublicSkillMessage, SkillHubMessage};
use crate::app::skill_hub::{
    ExpertHubSection, LoadState, PublicDetailMode, PublicDetailState, PublicDetailTarget,
    PublicInstallState, PublicSkillMarketState, SkillInstallScope, SkillInstallTarget,
    parse_public_markdown,
};
use crate::app::{App, Message};

fn locator() -> SkillMarketSkillLocatorDto {
    SkillMarketSkillLocatorDto {
        source: "skillhub".to_string(),
        namespace: "public".to_string(),
        slug: "中文技能".to_string(),
    }
}

fn app_with_markdown(mode: PublicDetailMode) -> App {
    let (mut app, _) = App::new();
    app.expert_hub.section = ExpertHubSection::Skills;
    let coordinate = SkillMarketCoordinateDto { locator: locator(), version: "1.0.0".to_string() };
    let markdown = concat!(
        "# 中文标题\n\n",
        "- 第一项\n",
        "- 第二项\n\n",
        "行内代码：`中文变量`\n\n",
        "```bash\n",
        "# 中文注释会被高亮器设为斜体等宽字体\n",
        "value=\"中文\"\n",
        "```\n",
    );
    app.expert_hub.public_market.detail = Some(PublicDetailState {
        target: PublicDetailTarget::Skill(locator()),
        generation: 1,
        mode,
        skill_detail: LoadState::Loaded(SkillMarketSkillDetailDto {
            skill: SkillMarketSkillDto {
                coordinate: coordinate.clone(),
                name: "中文技能".to_string(),
                description: "安全地处理中文内容".to_string(),
                author: "作者".to_string(),
                category: "开发".to_string(),
                tags: vec!["中文".to_string()],
                icon: None,
                downloads: 10,
                installs: 5,
                rating: 4.5,
                updated_at: "2026-07-30".to_string(),
            },
            license: Some("MIT".to_string()),
            homepage: None,
            repository: None,
            requires_api_key: false,
            security: None,
        }),
        skill_file: LoadState::Loaded(SkillMarketSkillFileDto {
            coordinate,
            content: markdown.to_string(),
        }),
        suite_detail: LoadState::Idle,
        markdown_items: parse_public_markdown(markdown),
        install: PublicInstallState::Idle,
        install_confirmation_open: false,
        install_scope: SkillInstallScope::Project,
        selected_install_project_path: Some("/repo/current".to_string()),
        selected_install_targets: vec![SkillInstallTarget::VibeWindow],
    });
    app
}

#[test]
fn expert_package_install_catalog_requires_matching_origin_and_meta_skill() {
    let target = PublicDetailTarget::Suite {
        source: "skillhub".to_string(),
        slug: "fixture-suite".to_string(),
    };
    let mut state = PublicSkillMarketState::default();
    let mut entry = DesktopSkillCatalogEntryDto {
        id: "skillhub--skillset--fixture-suite".to_string(),
        title: "fixture-suite".to_string(),
        description: String::new(),
        kind: "personal".to_string(),
        resource_count: 0,
        installed: true,
        enabled: true,
        source: "other".to_string(),
        source_path: Some("/project/.agents/skills/fixture-suite".to_string()),
        origin: Some("workspace".to_string()),
        namespace: Some("skillset".to_string()),
        package_type: Some("meta-skill".to_string()),
        source_pack: Some("fixture-suite".to_string()),
    };
    state.installed = LoadState::Loaded(vec![entry.clone()]);
    assert!(!installed_in_catalog(&state, &target));
    entry.source = "skillhub".to_string();
    state.installed = LoadState::Loaded(vec![entry.clone()]);
    assert!(installed_in_catalog(&state, &target));
    entry.package_type = Some("skill".to_string());
    state.installed = LoadState::Loaded(vec![entry]);
    assert!(!installed_in_catalog(&state, &target));
}

#[test]
fn public_skill_detail_platform_actions_follow_scope_and_busy_state() {
    let mut app = app_with_markdown(PublicDetailMode::Visual);
    let detail = app.expert_hub.public_market.detail.as_ref().expect("detail");
    assert!(detail_scope_action_message(detail, SkillInstallScope::Project, false).is_none());
    assert!(matches!(
        detail_scope_action_message(detail, SkillInstallScope::Global, false),
        Some(Message::SkillHub(SkillHubMessage::Public(
            PublicSkillMessage::SelectDetailInstallScope(SkillInstallScope::Global)
        )))
    ));
    assert!(matches!(
        detail_target_action_message(detail, SkillInstallTarget::Codex),
        Some(Message::SkillHub(SkillHubMessage::Public(
            PublicSkillMessage::ToggleDetailInstallTarget(SkillInstallTarget::Codex)
        )))
    ));
    assert!(matches!(
        detail_project_action_message(
            detail.selected_install_project_path.as_deref(),
            detail.install_is_busy(),
            "/repo/other".to_string(),
        ),
        Some(Message::SkillHub(SkillHubMessage::Public(
            PublicSkillMessage::SelectDetailInstallProject(path)
        ))) if path == "/repo/other"
    ));

    app.expert_hub.public_market.detail.as_mut().expect("detail").install =
        PublicInstallState::Installing { generation: 2 };
    let detail = app.expert_hub.public_market.detail.as_ref().expect("detail");
    assert!(detail_target_action_message(detail, SkillInstallTarget::ClaudeCode).is_none());
    assert!(
        detail_project_action_message(
            detail.selected_install_project_path.as_deref(),
            detail.install_is_busy(),
            "/repo/other".to_string(),
        )
        .is_none()
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn public_market_detail_renders_chinese_markdown_in_both_modes_and_themes() {
    for mode in [PublicDetailMode::Visual, PublicDetailMode::Markdown] {
        let app = app_with_markdown(mode);
        for theme in [Theme::Light, Theme::Dark] {
            let mut element = view(&app);
            let mut renderer =
                <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                    .await
                    .expect("headless renderer");
            let bounds = Size::new(900.0, 720.0);
            let mut tree = Tree::new(element.as_widget());
            let node = element.as_widget_mut().layout(
                &mut tree,
                &renderer,
                &layout::Limits::new(Size::ZERO, bounds),
            );
            let viewport = Rectangle::with_size(bounds);
            element.as_widget().draw(
                &tree,
                &mut renderer,
                &theme,
                &renderer::Style::default(),
                Layout::new(&node),
                mouse::Cursor::Unavailable,
                &viewport,
            );
            let pixels = renderer.screenshot(Size::new(900, 720), 1.0, Color::TRANSPARENT);
            assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
        }
    }
    let _: iced::Element<'_, Message> = view(&app_with_markdown(PublicDetailMode::Visual));
}

#[tokio::test(flavor = "multi_thread")]
async fn expert_package_install_modal_renders_chinese_in_both_themes() {
    let mut app = app_with_markdown(PublicDetailMode::Visual);
    app.expert_hub.section = ExpertHubSection::Experts;
    app.project_path = Some("/repo/current".to_string());
    let detail = app.expert_hub.public_market.detail.as_mut().expect("detail");
    detail.target = PublicDetailTarget::Suite {
        source: "skillhub".to_string(),
        slug: "中文专家".to_string(),
    };
    detail.skill_detail = LoadState::Idle;
    detail.suite_detail = LoadState::Loaded(SkillMarketSkillSetDetailDto {
        skill_set: SkillMarketSkillSetDto {
            source: "skillhub".to_string(),
            slug: "中文专家".to_string(),
            name: "中文专家安装反馈".to_string(),
            description: "选择项目后安装".to_string(),
            author: String::new(),
            icon: None,
            member_count: 0,
            updated_at: String::new(),
        },
        members: Vec::new(),
        content: "# 中文工作流".to_string(),
    });
    detail.install_confirmation_open = true;
    for theme in [Theme::Light, Theme::Dark] {
        let mut element = view(&app);
        let mut renderer =
            <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
                .await
                .expect("headless renderer");
        let bounds = Size::new(900.0, 720.0);
        let mut tree = Tree::new(element.as_widget());
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, bounds),
        );
        let viewport = Rectangle::with_size(bounds);
        element.as_widget().draw(
            &tree,
            &mut renderer,
            &theme,
            &renderer::Style::default(),
            Layout::new(&node),
            mouse::Cursor::Unavailable,
            &viewport,
        );
        let pixels = renderer.screenshot(Size::new(900, 720), 1.0, Color::TRANSPARENT);
        assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
    }
}
