//! 公共技能市场测试，覆盖来源选择、推荐卡片和三类工作助手标签。
//! 来源选择器仅在确有多个来源时出现，窄布局仍需保持卡片背景与中文文本可读。

use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Color, Element, Font, Pixels, Rectangle, Size, Theme};
use vw_gateway_client::{
    SkillMarketCoordinateDto, SkillMarketSkillDto, SkillMarketSkillLocatorDto,
    SkillMarketSkillSetDto, SkillMarketSkillSetPageDto, SkillMarketSourceDto,
};

use super::public_skills::{grid_column_count, public_tab_specs, source_selector_visible, view};
use super::styles::{card_style, featured_card_style};
use crate::app::skill_hub::{ExpertHubSection, LoadState, PublicSkillTab};
use crate::app::{App, Message};

fn source(source: &str) -> SkillMarketSourceDto {
    SkillMarketSourceDto { source: source.to_string(), display_name: source.to_string() }
}

fn skill() -> SkillMarketSkillDto {
    SkillMarketSkillDto {
        coordinate: SkillMarketCoordinateDto {
            locator: SkillMarketSkillLocatorDto {
                source: "skillhub".to_string(),
                namespace: "公开命名空间".to_string(),
                slug: "long-title".to_string(),
            },
            version: "1.0.0".to_string(),
        },
        name: "超长中文技能标题用于验证窄窗口渲染不会崩溃".to_string(),
        description: "这是一个包含中文的技能介绍，用于验证默认字体整形和暗黑主题显示。".repeat(4),
        author: "技能作者".to_string(),
        category: "开发工具".to_string(),
        tags: vec!["中文".to_string(), "自动化".to_string()],
        icon: None,
        downloads: 120,
        installs: 45,
        rating: 4.8,
        updated_at: "2026-07-30".to_string(),
    }
}

fn populated_app(tab: PublicSkillTab) -> App {
    let (mut app, _) = App::new();
    app.expert_hub.section = ExpertHubSection::Skills;
    app.expert_hub.public_market.tab = tab;
    app.expert_hub.public_market.sources =
        LoadState::Loaded(vec![source("skillhub"), source("second")]);
    app.expert_hub.public_market.current_source = Some("skillhub".to_string());
    app.expert_hub.public_market.recommended = LoadState::Loaded(vec![skill()]);
    app.expert_hub.public_market.market =
        LoadState::Loaded(vw_gateway_client::SkillMarketSkillPageDto {
            skills: vec![skill()],
            total: 1,
            page: 1,
            page_size: 24,
        });
    app.expert_hub.public_market.suites = LoadState::Loaded(SkillMarketSkillSetPageDto {
        skillsets: vec![SkillMarketSkillSetDto {
            source: "skillhub".to_string(),
            slug: "automation-suite".to_string(),
            name: "中文自动化测试套件".to_string(),
            description: "覆盖完整研发流程的技能套件".to_string(),
            author: "SkillHub".to_string(),
            icon: None,
            member_count: 6,
            updated_at: "2026-07-30".to_string(),
        }],
        total: 1,
        page: 1,
        page_size: 24,
    });
    app
}

async fn assert_draws(mut element: Element<'_, Message>, theme: &Theme, width: f32) {
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let bounds = Size::new(width, 800.0);
    let limits = layout::Limits::new(Size::ZERO, bounds);
    let mut tree = Tree::new(element.as_widget());
    let node = element.as_widget_mut().layout(&mut tree, &renderer, &limits);
    let viewport = Rectangle::with_size(bounds);
    element.as_widget().draw(
        &tree,
        &mut renderer,
        theme,
        &renderer::Style::default(),
        Layout::new(&node),
        mouse::Cursor::Unavailable,
        &viewport,
    );
    let pixels = renderer.screenshot(Size::new(width as u32, 800), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[test]
fn public_skill_page_builds_three_workbuddy_tabs() {
    assert_eq!(
        public_tab_specs(),
        &[
            ("推荐", PublicSkillTab::Recommended),
            ("SkillHub", PublicSkillTab::SkillHub),
            ("套件", PublicSkillTab::Suites),
        ]
    );
    assert_eq!(grid_column_count(720.0), 2);
    assert_eq!(grid_column_count(1_440.0), 4);
}

#[test]
fn source_selector_appears_only_for_multiple_sources() {
    assert!(!source_selector_visible(&LoadState::Idle));
    assert!(!source_selector_visible(&LoadState::Loaded(vec![source("skillhub")])));
    assert!(source_selector_visible(&LoadState::Loaded(vec![
        source("skillhub"),
        source("second"),
    ])));
}

#[test]
fn featured_skill_cards_keep_the_recommended_card_background() {
    for theme in [Theme::Light, Theme::Dark] {
        assert_eq!(featured_card_style(&theme).background, card_style(&theme).background);
    }
}

#[tokio::test]
async fn public_skill_page_renders_chinese_in_both_themes() {
    for tab in [PublicSkillTab::Recommended, PublicSkillTab::SkillHub, PublicSkillTab::Suites] {
        let app = populated_app(tab);
        for theme in [Theme::Light, Theme::Dark] {
            for width in [720.0, 1_440.0] {
                assert_draws(view(&app), &theme, width).await;
            }
        }
    }
}
