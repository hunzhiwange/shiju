use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
use vw_gateway_client::{
    ShijuAppCatalogDto, ShijuAppListDto, ShijuCatalogAppDto, ShijuInstalledAppDto,
};

use super::shiju_apps;
use crate::app::shiju_apps::{
    ShijuAppDetailTarget, ShijuAppsLoadState, ShijuAppsTab,
};
use crate::app::{App, Message};

fn catalog_app(installed: bool) -> ShijuCatalogAppDto {
    ShijuCatalogAppDto {
        app_id: "meeting-notes".to_string(),
        name: "会议纪要助手".to_string(),
        description: Some("将会议内容整理成结构化中文纪要".to_string()),
        icon_url: Some("https://example.invalid/icon.png".to_string()),
        version: "1.2.3".to_string(),
        installed,
        installation_id: installed.then(|| "install-01".to_string()),
        user_id: installed.then(|| "user-01".to_string()),
        installed_at: installed.then(|| "2026-07-30T09:00:00Z".to_string()),
        permissions: vec!["documents:read".to_string(), "notes:write".to_string()],
    }
}

fn installed_app(ready: bool) -> ShijuInstalledAppDto {
    ShijuInstalledAppDto {
        installation_id: "install-01".to_string(),
        app_id: "meeting-notes".to_string(),
        name: "会议纪要助手".to_string(),
        description: Some("我的应用中文说明".to_string()),
        icon_url: Some("https://example.invalid/icon.png".to_string()),
        version: "1.2.3".to_string(),
        source: "store".to_string(),
        reconcile_status: if ready { "ready" } else { "pending" }.to_string(),
        runtime_status: if ready { "ready" } else { "unavailable" }.to_string(),
        runtime_url: "https://app.shiju.cloud/app/install-01/".to_string(),
        created_at: Some("2026-07-30T09:00:00Z".to_string()),
        updated_at: Some("2026-07-30T10:00:00Z".to_string()),
    }
}

fn app_with_lists() -> App {
    let (mut app, _) = App::new();
    app.expert_hub.shiju_apps.store =
        ShijuAppsLoadState::Loaded(ShijuAppCatalogDto {
            apps: vec![catalog_app(true), catalog_app(false)],
        });
    app.expert_hub.shiju_apps.mine = ShijuAppsLoadState::Loaded(ShijuAppListDto {
        apps: vec![installed_app(true), installed_app(false)],
    });
    app
}

fn keep_element(_element: iced::Element<'_, Message>) {}

#[test]
fn shiju_apps_view_builds_store_mine_and_details() {
    let mut app = app_with_lists();
    assert_eq!(
        shiju_apps::tab_specs(),
        &[
            ("我的应用", ShijuAppsTab::Mine),
            ("应用商店", ShijuAppsTab::Store),
        ]
    );

    keep_element(shiju_apps::view(&app));
    app.expert_hub.shiju_apps.tab = ShijuAppsTab::Store;
    keep_element(shiju_apps::view(&app));

    app.expert_hub.shiju_apps.detail = Some(ShijuAppDetailTarget::Store(0));
    keep_element(shiju_apps::view(&app));
    app.expert_hub.shiju_apps.detail = Some(ShijuAppDetailTarget::Mine(0));
    keep_element(shiju_apps::view(&app));
}

#[test]
fn shiju_apps_view_exposes_loading_empty_error_and_login_states() {
    let (mut app, _) = App::new();
    let states = [
        ShijuAppsLoadState::NeedsLogin,
        ShijuAppsLoadState::Loading { generation: 1 },
        ShijuAppsLoadState::Loaded(ShijuAppCatalogDto { apps: Vec::new() }),
        ShijuAppsLoadState::Error("目录加载失败".to_string()),
    ];
    for state in states {
        app.expert_hub.shiju_apps.store = state;
        keep_element(shiju_apps::view(&app));
    }
}

#[test]
fn shiju_apps_cards_surface_runtime_readiness() {
    assert_eq!(shiju_apps::mine::runtime_readiness(&installed_app(true)), Ok("运行就绪"));
    assert_eq!(
        shiju_apps::mine::runtime_readiness(&installed_app(false)),
        Err("协调状态：pending；运行状态：unavailable".to_string())
    );
    assert!(shiju_apps::mine::open_action_message(&installed_app(true)).is_some());
    assert!(shiju_apps::mine::open_action_message(&installed_app(false)).is_none());
    assert_eq!(shiju_apps::styles::SCROLLBAR_WIDTH, 4);
}

async fn assert_draws(app: &App, theme: Theme) {
    let mut element = shiju_apps::view(app);
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
async fn shiju_apps_dark_chinese_render_path_is_safe() {
    let mut app = app_with_lists();
    for detail in [
        None,
        Some(ShijuAppDetailTarget::Store(0)),
        Some(ShijuAppDetailTarget::Mine(0)),
    ] {
        app.expert_hub.shiju_apps.detail = detail;
        assert_draws(&app, Theme::Dark).await;
    }
}
