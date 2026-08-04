use crate::app::account::{
    AccountIdentity, AccountOrigin, AccountRepository, SecretSkey, VerifiedAccount,
};
use crate::app::message::ShijuAppsMessage;
use crate::app::shiju_apps::{
    ShijuAppLaunchState, ShijuAppsLoadState, ShijuAppsTab,
};
use crate::app::{App, Message, Screen};
use vw_gateway_client::{
    ShijuAppCatalogDto, ShijuAppLaunchDto, ShijuAppListDto, ShijuInstalledAppDto,
};

fn repository(secret: &str, company_id: &str) -> AccountRepository {
    let mut repository = AccountRepository::default();
    repository.upsert_verified(
        VerifiedAccount::from_parts(
            AccountIdentity::new(company_id, "示例公司", "founder", "active").unwrap(),
            SecretSkey::new(secret).unwrap(),
        ),
        AccountOrigin::BrowserLogin,
    );
    repository
}

#[test]
fn shiju_apps_open_loads_mine_through_gateway() {
    let _ = crate::app::session_gateway::gateway_shiju_app_catalog_async;
    let _ = crate::app::session_gateway::gateway_shiju_apps_async;
    let _ = crate::app::session_gateway::gateway_shiju_app_prepare_launch_async;
    let (mut app, _) = App::new();
    app.account.replace_repository(repository("account-secret", "company-1"));

    let _task = app.update(Message::ShijuApps(ShijuAppsMessage::Open));

    assert_eq!(app.screen, Screen::ShijuApps);
    assert_eq!(app.expert_hub.shiju_apps.tab, ShijuAppsTab::Mine);
    assert_eq!(app.expert_hub.shiju_apps.account_id.as_deref(), Some("company-1"));
    assert!(matches!(
        app.expert_hub.shiju_apps.mine,
        ShijuAppsLoadState::Loading { .. }
    ));
    let tabs = app
        .open_tabs
        .iter()
        .filter(|tab| tab.id == "shiju_apps")
        .collect::<Vec<_>>();
    assert_eq!(tabs.len(), 1);
    assert_eq!(tabs[0].title, "识矩应用");
    assert_eq!(tabs[0].screen, Screen::ShijuApps);
}

#[test]
fn shiju_apps_tabs_ignore_stale_or_wrong_account_results() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository("account-secret", "company-1"));
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::Open));
    let mine_generation = app.expert_hub.shiju_apps.generation;

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::MineLoaded {
        generation: mine_generation.wrapping_sub(1),
        account_id: "company-1".to_string(),
        result: Ok(ShijuAppListDto { apps: Vec::new() }),
    }));
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::MineLoaded {
        generation: mine_generation,
        account_id: "wrong-account".to_string(),
        result: Ok(ShijuAppListDto { apps: Vec::new() }),
    }));
    assert!(matches!(
        app.expert_hub.shiju_apps.mine,
        ShijuAppsLoadState::Loading { .. }
    ));

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::MineLoaded {
        generation: mine_generation,
        account_id: "company-1".to_string(),
        result: Ok(ShijuAppListDto { apps: Vec::new() }),
    }));
    assert!(matches!(
        app.expert_hub.shiju_apps.mine,
        ShijuAppsLoadState::Loaded(_)
    ));

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::SelectTab(
        ShijuAppsTab::Store,
    )));
    let store_generation = app.expert_hub.shiju_apps.generation;
    assert!(store_generation > mine_generation);
    assert!(matches!(
        app.expert_hub.shiju_apps.store,
        ShijuAppsLoadState::Loading { .. }
    ));
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::StoreLoaded {
        generation: store_generation,
        account_id: "company-1".to_string(),
        result: Ok(ShijuAppCatalogDto { apps: Vec::new() }),
    }));
    assert!(matches!(
        app.expert_hub.shiju_apps.store,
        ShijuAppsLoadState::Loaded(_)
    ));
}

fn ready_app() -> ShijuInstalledAppDto {
    ShijuInstalledAppDto {
        installation_id: "01K1APP0000000000000000001".to_string(),
        app_id: "meeting-notes".to_string(),
        name: "会议纪要助手".to_string(),
        description: None,
        icon_url: None,
        version: "1.0.0".to_string(),
        source: "store".to_string(),
        reconcile_status: "ready".to_string(),
        runtime_status: "ready".to_string(),
        runtime_url: "https://app.shiju.cloud/app/01K1APP0000000000000000001/".to_string(),
        created_at: None,
        updated_at: None,
    }
}

#[test]
fn open_shiju_app_prepares_binding_before_spawn_and_blocks_duplicates() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository("account-secret", "company-1"));
    app.expert_hub.shiju_apps.account_id = Some("company-1".to_string());
    app.expert_hub.shiju_apps.mine =
        ShijuAppsLoadState::Loaded(ShijuAppListDto { apps: vec![ready_app()] });

    let _task = app.update(Message::ShijuApps(ShijuAppsMessage::OpenInstalled(
        "01K1APP0000000000000000001".to_string(),
    )));
    let generation = app.expert_hub.shiju_apps.generation;
    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::Preparing {
            generation: active_generation,
            ref installation_id,
        } if active_generation == generation
            && installation_id == "01K1APP0000000000000000001"
    ));

    let _duplicate = app.update(Message::ShijuApps(ShijuAppsMessage::OpenInstalled(
        "01K1APP0000000000000000001".to_string(),
    )));
    assert_eq!(app.expert_hub.shiju_apps.generation, generation);
}

#[test]
fn stale_or_wrong_account_launch_is_ignored() {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository("account-secret", "company-1"));
    app.expert_hub.shiju_apps.account_id = Some("company-1".to_string());
    app.expert_hub.shiju_apps.generation = 9;
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::Preparing {
        generation: 9,
        installation_id: "01K1APP0000000000000000001".to_string(),
    };
    let launch = ShijuAppLaunchDto {
        installation_id: "01K1APP0000000000000000001".to_string(),
        name: "会议纪要助手".to_string(),
        runtime_url: "https://app.shiju.cloud/app/01K1APP0000000000000000001/".to_string(),
        runtime_set_cookie:
            "app_runtime_session=runtime-session-secret; Path=/; HttpOnly; Secure; SameSite=None"
                .to_string(),
    };

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::LaunchPrepared {
        generation: 8,
        account_id: "company-1".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Ok(launch.clone()),
    }));
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::LaunchPrepared {
        generation: 9,
        account_id: "wrong-account".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Ok(launch),
    }));

    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::Preparing { generation: 9, .. }
    ));
    assert!(app.independent_webview_children.is_empty());
}
