//! 时聚应用视图测试，覆盖商店、已安装列表、详情及 API 密钥绑定覆盖层。
//! 加载、空、错误和未登录状态均需可渲染，运行就绪与密钥选择决定动作可用性。

use iced::advanced::renderer::Headless;
use iced::advanced::{Layout, layout, mouse, renderer, widget::Tree};
use iced::{Color, Font, Pixels, Rectangle, Size, Theme};
use vw_gateway_client::{
    ShijuAppCatalogDto, ShijuAppListDto, ShijuCatalogAppDto, ShijuIamApiKeyDto,
    ShijuInstalledAppDto,
};

use super::shiju_apps;
use crate::app::message::ShijuAppsMessage;
use crate::app::shiju_apps::{
    ShijuAppDetailTarget, ShijuAppLaunchState, ShijuAppsLoadState, ShijuAppsTab,
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

fn api_key(api_key_id: u64, key_name: &str, status: &str) -> ShijuIamApiKeyDto {
    ShijuIamApiKeyDto {
        api_key_id,
        key_name: key_name.to_string(),
        api_key_mask: format!("sk-****{api_key_id:04}"),
        status: status.to_string(),
        expires_at: Some("2027-08-22T12:00:00Z".to_string()),
        updated_at: Some("2026-08-22T12:00:00Z".to_string()),
    }
}

fn app_with_lists() -> App {
    let (mut app, _) = App::new();
    app.expert_hub.shiju_apps.store = ShijuAppsLoadState::Loaded(ShijuAppCatalogDto {
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
        &[("我的应用", ShijuAppsTab::Mine), ("应用商店", ShijuAppsTab::Store),]
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

#[test]
fn shiju_apps_grid_limits_each_row_to_four_cards() {
    let cards = (0..9)
        .map(|index| iced::widget::text(index.to_string()).into())
        .collect::<Vec<iced::Element<'static, Message>>>();

    let row_sizes = shiju_apps::card_grid(cards)
        .as_widget()
        .children()
        .into_iter()
        .map(|row| row.children.len())
        .collect::<Vec<_>>();

    assert_eq!(row_sizes, vec![4, 4, 1]);
}

#[test]
fn shiju_apps_title_uses_single_line_ellipsis_when_too_long() {
    assert_eq!(shiju_apps::app_title("会议纪要助手"), "会议纪要助手");
    assert_eq!(shiju_apps::app_title("AI 智能托管数据日志查询"), "AI 智能托管数据日志...");
}

#[test]
fn api_key_binding_overlay_appears_only_after_unconfigured_binding() {
    let mut app = app_with_lists();
    let states = [
        ShijuAppLaunchState::LoadingApiKeys {
            generation: 7,
            installation_id: "install-01".to_string(),
            can_edit: true,
        },
        ShijuAppLaunchState::SelectingApiKey {
            generation: 7,
            installation_id: "install-01".to_string(),
            can_edit: true,
            api_keys: Vec::new(),
            selected_api_key_id: None,
            error: Some("当前账户没有可选择的 IAM API 密钥".to_string()),
        },
        ShijuAppLaunchState::SavingApiKeyBinding {
            generation: 7,
            installation_id: "install-01".to_string(),
            can_edit: true,
            api_keys: vec![api_key(11, "中文主密钥", "enabled")],
            selected_api_key_id: 11,
        },
        ShijuAppLaunchState::SelectingApiKey {
            generation: 7,
            installation_id: "install-01".to_string(),
            can_edit: true,
            api_keys: vec![api_key(11, "中文主密钥", "enabled")],
            selected_api_key_id: Some(11),
            error: None,
        },
    ];

    for state in states {
        app.expert_hub.shiju_apps.launch = state;
        assert!(shiju_apps::api_key_binding::is_visible(&app.expert_hub.shiju_apps.launch));
        let element = shiju_apps::view(&app);
        assert_eq!(
            element.as_widget().children().len(),
            2,
            "base and modal must both remain mounted"
        );
    }

    for state in [
        ShijuAppLaunchState::Idle,
        ShijuAppLaunchState::CheckingBinding {
            generation: 7,
            installation_id: "install-01".to_string(),
        },
        ShijuAppLaunchState::Preparing { generation: 7, installation_id: "install-01".to_string() },
        ShijuAppLaunchState::Error {
            generation: 7,
            installation_id: "install-01".to_string(),
            message: "绑定检查失败".to_string(),
        },
    ] {
        assert!(!shiju_apps::api_key_binding::is_visible(&state));
    }
}

#[test]
fn api_key_binding_actions_follow_selected_enabled_key() {
    let enabled = api_key(11, "中文主密钥", "enabled");
    let disabled = api_key(12, "历史密钥", "disabled");
    let mut state = ShijuAppLaunchState::SelectingApiKey {
        generation: 7,
        installation_id: "install-01".to_string(),
        can_edit: true,
        api_keys: vec![disabled.clone(), enabled.clone()],
        selected_api_key_id: Some(disabled.api_key_id),
        error: None,
    };

    assert!(!shiju_apps::api_key_binding::save_enabled(&state));
    if let ShijuAppLaunchState::SelectingApiKey { selected_api_key_id, .. } = &mut state {
        *selected_api_key_id = None;
    }
    assert!(!shiju_apps::api_key_binding::save_enabled(&state));
    if let ShijuAppLaunchState::SelectingApiKey { selected_api_key_id, .. } = &mut state {
        *selected_api_key_id = Some(enabled.api_key_id);
    }
    assert!(shiju_apps::api_key_binding::save_enabled(&state));
    assert_eq!(
        shiju_apps::api_key_binding::selected_key(&state).map(|key| key.api_key_id),
        Some(enabled.api_key_id)
    );
    assert_eq!(
        shiju_apps::api_key_binding::selected_label(&state).as_deref(),
        Some("中文主密钥 (sk-****0011)")
    );

    assert!(matches!(
        shiju_apps::api_key_binding::select_action(enabled.api_key_id),
        ShijuAppsMessage::SelectApiKey(11)
    ));
    assert!(matches!(
        shiju_apps::api_key_binding::refresh_action(),
        ShijuAppsMessage::RefreshApiKeys
    ));
    assert!(matches!(
        shiju_apps::api_key_binding::save_action(),
        ShijuAppsMessage::SaveApiKeyBinding
    ));
    assert!(matches!(
        shiju_apps::api_key_binding::cancel_action(),
        ShijuAppsMessage::CancelApiKeyBinding
    ));

    if let ShijuAppLaunchState::SelectingApiKey { can_edit, selected_api_key_id, .. } = &mut state {
        *can_edit = false;
        *selected_api_key_id = Some(enabled.api_key_id);
    }
    assert!(!shiju_apps::api_key_binding::save_enabled(&state));
    assert!(!shiju_apps::api_key_binding::save_enabled(
        &ShijuAppLaunchState::SavingApiKeyBinding {
            generation: 7,
            installation_id: "install-01".to_string(),
            can_edit: true,
            api_keys: vec![enabled],
            selected_api_key_id: 11,
        }
    ));
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
    for detail in [None, Some(ShijuAppDetailTarget::Store(0)), Some(ShijuAppDetailTarget::Mine(0))]
    {
        app.expert_hub.shiju_apps.detail = detail;
        assert_draws(&app, Theme::Dark).await;
    }
}

#[tokio::test]
async fn api_key_binding_dark_and_light_render_paths_are_safe() {
    let mut app = app_with_lists();
    let states = [
        ShijuAppLaunchState::CheckingBinding {
            generation: 8,
            installation_id: "install-01".to_string(),
        },
        ShijuAppLaunchState::LoadingApiKeys {
            generation: 8,
            installation_id: "install-01".to_string(),
            can_edit: true,
        },
        ShijuAppLaunchState::SelectingApiKey {
            generation: 8,
            installation_id: "install-01".to_string(),
            can_edit: true,
            api_keys: Vec::new(),
            selected_api_key_id: None,
            error: Some("当前账户没有已启用的 IAM API 密钥".to_string()),
        },
        ShijuAppLaunchState::SelectingApiKey {
            generation: 8,
            installation_id: "install-01".to_string(),
            can_edit: true,
            api_keys: vec![
                api_key(11, "中文主密钥", "enabled"),
                api_key(12, "已停用密钥", "disabled"),
            ],
            selected_api_key_id: Some(11),
            error: None,
        },
        ShijuAppLaunchState::SelectingApiKey {
            generation: 8,
            installation_id: "install-01".to_string(),
            can_edit: false,
            api_keys: Vec::new(),
            selected_api_key_id: None,
            error: Some("当前应用的个人 API 密钥绑定不可修改".to_string()),
        },
        ShijuAppLaunchState::SavingApiKeyBinding {
            generation: 8,
            installation_id: "install-01".to_string(),
            can_edit: true,
            api_keys: vec![api_key(11, "中文主密钥", "enabled")],
            selected_api_key_id: 11,
        },
        ShijuAppLaunchState::Error {
            generation: 8,
            installation_id: "install-01".to_string(),
            message: "识矩应用 API 密钥绑定保存失败".to_string(),
        },
    ];

    for state in states {
        app.expert_hub.shiju_apps.launch = state;
        assert_draws(&app, Theme::Dark).await;
        assert_draws(&app, Theme::Light).await;
    }
}
