//! 验证专家包列表状态和中文详情的真实绘制路径。

use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
use vw_gateway_client::{
    SkillMarketSkillSetDetailDto, SkillMarketSkillSetDto, SkillMarketSkillSetPageDto,
    SkillMarketSourceDto,
};

use super::experts::view;
use crate::app::App;
use crate::app::message::skill_hub::{PublicSkillMessage, SkillHubMessage, update};
use crate::app::skill_hub::{ExpertHubSection, LoadState, PublicSkillTab};

fn suite() -> SkillMarketSkillSetDto {
    SkillMarketSkillSetDto {
        source: "skillhub".to_string(),
        slug: "chinese-expert".to_string(),
        name: "超长中文专家标题用于验证窄窗口显示和字体整形不会崩溃".to_string(),
        description: "这个专家包提供完整的中文工作流介绍。".repeat(6),
        author: "SkillHub".to_string(),
        icon: None,
        member_count: 2,
        updated_at: "2026-10-25".to_string(),
    }
}

fn app() -> App {
    let (mut app, _) = App::new();
    app.expert_hub.section = ExpertHubSection::Experts;
    app.expert_hub.public_market.tab = PublicSkillTab::Suites;
    app.expert_hub.public_market.current_source = Some("skillhub".to_string());
    app.expert_hub.public_market.sources = LoadState::Loaded(vec![SkillMarketSourceDto {
        source: "skillhub".to_string(),
        display_name: "SkillHub".to_string(),
    }]);
    app
}

async fn assert_draws(app: &App, theme: &Theme) {
    let mut element = view(app);
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let bounds = Size::new(720.0, 800.0);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(
        &mut tree,
        &renderer,
        &layout::Limits::new(Size::ZERO, bounds),
    );
    element.as_widget().draw(
        &tree,
        &mut renderer,
        theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &Rectangle::with_size(bounds),
    );
    let pixels = renderer.screenshot(Size::new(720, 800), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[tokio::test(flavor = "multi_thread")]
async fn expert_package_view_renders_loading_empty_error_and_detail() {
    let mut app = app();
    for theme in [Theme::Light, Theme::Dark] {
        app.expert_hub.public_market.suites = LoadState::Loading { generation: 1 };
        assert_draws(&app, &theme).await;
        app.expert_hub.public_market.suites = LoadState::Loaded(SkillMarketSkillSetPageDto {
            skillsets: Vec::new(),
            total: 0,
            page: 1,
            page_size: 24,
        });
        assert_draws(&app, &theme).await;
        app.expert_hub.public_market.suites = LoadState::Error("网络异常，请重试".to_string());
        assert_draws(&app, &theme).await;
        app.expert_hub.public_market.suites = LoadState::Loaded(SkillMarketSkillSetPageDto {
            skillsets: vec![suite()],
            total: 1,
            page: 1,
            page_size: 24,
        });
        assert_draws(&app, &theme).await;
    }

    let _ = update(
        &mut app,
        SkillHubMessage::Public(PublicSkillMessage::OpenSuiteDetail {
            source: "skillhub".to_string(),
            slug: "chinese-expert".to_string(),
        }),
    );
    let generation = app.expert_hub.public_market.detail_generation;
    let _ = update(
        &mut app,
        SkillHubMessage::Public(PublicSkillMessage::SuiteDetailLoaded {
            source: "skillhub".to_string(),
            slug: "chinese-expert".to_string(),
            generation,
            result: Ok(SkillMarketSkillSetDetailDto {
                skill_set: suite(),
                members: Vec::new(),
                content: "# 中文专家工作流\n\n这是完整的工作流正文。".to_string(),
            }),
        }),
    );
    for theme in [Theme::Light, Theme::Dark] {
        assert_draws(&app, &theme).await;
    }
    let _ = update(&mut app, SkillHubMessage::Public(PublicSkillMessage::CloseDetail));
    assert!(app.expert_hub.public_market.detail.is_none());
    assert_draws(&app, &Theme::Dark).await;
}
