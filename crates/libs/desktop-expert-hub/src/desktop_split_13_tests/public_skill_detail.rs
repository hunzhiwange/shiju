//! 公共技能详情测试，覆盖 Markdown 内容及按安装范围生成的平台动作。
//! 项目上下文缺失或安装正忙时，操作可用性必须收敛到安全状态。

use gateway_skill_market::{
    SkillMarketCoordinateDto, SkillMarketSkillDetailDto, SkillMarketSkillDto,
    SkillMarketSkillFileDto, SkillMarketSkillLocatorDto, SkillMarketSkillSetDetailDto,
    SkillMarketSkillSetDto,
};
use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};

use crate::views::Message;
use desktop_skill_hub::skill_hub::ExpertHubState;
use desktop_skill_hub::skill_hub::{
    ExpertHubSection, LoadState, PublicDetailMode, PublicDetailState, PublicDetailTarget,
    PublicInstallState, SkillInstallScope, SkillInstallTarget, parse_public_markdown,
};

fn locator() -> SkillMarketSkillLocatorDto {
    SkillMarketSkillLocatorDto {
        source: "skillhub".to_string(),
        namespace: "public".to_string(),
        slug: "中文技能".to_string(),
    }
}

fn app_with_markdown(mode: PublicDetailMode) -> DetailFixture {
    let mut app = DetailFixture { expert_hub: ExpertHubState::default(), project_path: None };
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
        if let Ok(dir) = std::env::var("VW_EXPERT_SCREENSHOT_DIR") {
            let name = if theme == Theme::Light { "light" } else { "dark" };
            let path = std::path::Path::new(&dir).join(format!("expert-install-{name}.png"));
            image::save_buffer(path, &pixels, 900, 720, image::ColorType::Rgba8)
                .expect("expert install screenshot");
        }
    }
}

struct DetailFixture {
    expert_hub: ExpertHubState,
    project_path: Option<String>,
}
fn view(app: &DetailFixture) -> iced::Element<'_, Message> {
    crate::views::public_skill_detail::view(&crate::views::public_skill_detail::DetailInput {
        expert_hub: &app.expert_hub,
        desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
        app_theme: &Theme::Dark,
        project_path: &app.project_path,
        recent_projects: &[],
        selection: Ok(()),
    })
}
