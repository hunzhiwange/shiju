use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
use vw_gateway_client::DesktopSkillCatalogEntryDto;

use super::installed_skills::{view, with_delete_confirmation};
use crate::app::skill_hub::{ExpertHubSection, LoadState};
use crate::app::{App, Message};

fn item(
    id: &str,
    source: &str,
    package_type: Option<&str>,
    source_pack: Option<&str>,
) -> DesktopSkillCatalogEntryDto {
    DesktopSkillCatalogEntryDto {
        id: id.to_string(),
        title: format!("中文技能 {id}"),
        description: "用于验证来源徽标、套件徽标和中文确认层".to_string(),
        kind: "personal".to_string(),
        resource_count: 1,
        installed: true,
        enabled: true,
        source: source.to_string(),
        source_path: Some(format!("/project/{id}")),
        origin: Some("workspace".to_string()),
        namespace: Some("public".to_string()),
        package_type: package_type.map(str::to_string),
        source_pack: source_pack.map(str::to_string),
    }
}

async fn assert_draws(app: &App, theme: Theme) {
    let mut element = with_delete_confirmation(app, view(app));
    let mut renderer =
        <iced::Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia"))
            .await
            .expect("headless renderer");
    let bounds = Size::new(960.0, 720.0);
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
    let pixels = renderer.screenshot(Size::new(960, 720), 1.0, Color::TRANSPARENT);
    assert!(pixels.chunks_exact(4).any(|pixel| pixel[3] > 0));
}

#[tokio::test]
async fn installed_skills_render_local_provider_and_suite_cards_with_confirmation() {
    let (mut app, _) = App::new();
    app.expert_hub.section = ExpertHubSection::Skills;
    app.expert_hub.public_market.installed_open = true;
    app.expert_hub.public_market.installed = LoadState::Loaded(vec![
        item("local", "workspace", None, None),
        item("market", "skillhub", Some("skill"), Some("safe-suite")),
        item("suite", "skillhub", Some("meta-skill"), Some("safe-suite")),
    ]);
    app.expert_hub.public_market.select_all_installed();
    app.expert_hub.public_market.delete_confirmation_open = true;

    for theme in [Theme::Light, Theme::Dark] {
        assert_draws(&app, theme).await;
    }
    let _: iced::Element<'_, Message> = view(&app);
}
