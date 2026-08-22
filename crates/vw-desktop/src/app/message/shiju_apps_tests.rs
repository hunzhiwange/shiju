use crate::app::account::{
    AccountIdentity, AccountOrigin, AccountRepository, SecretSkey, VerifiedAccount,
};
use crate::app::message::ShijuAppsMessage;
use crate::app::shiju_apps::{ShijuAppLaunchState, ShijuAppsLoadState, ShijuAppsTab};
use crate::app::{App, Message, Screen};
use vw_gateway_client::{
    ShijuAppApiKeyBindingDto, ShijuAppCatalogDto, ShijuAppLaunchDto, ShijuAppListDto,
    ShijuAppPersonalApiKeyBindingDto, ShijuIamApiKeyDto, ShijuIamApiKeyListDto,
    ShijuInstalledAppDto,
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
    assert!(matches!(app.expert_hub.shiju_apps.mine, ShijuAppsLoadState::Loading { .. }));
    let tabs = app.open_tabs.iter().filter(|tab| tab.id == "shiju_apps").collect::<Vec<_>>();
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
    assert!(matches!(app.expert_hub.shiju_apps.mine, ShijuAppsLoadState::Loading { .. }));

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::MineLoaded {
        generation: mine_generation,
        account_id: "company-1".to_string(),
        result: Ok(ShijuAppListDto { apps: Vec::new() }),
    }));
    assert!(matches!(app.expert_hub.shiju_apps.mine, ShijuAppsLoadState::Loaded(_)));

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::SelectTab(ShijuAppsTab::Store)));
    let store_generation = app.expert_hub.shiju_apps.generation;
    assert!(store_generation > mine_generation);
    assert!(matches!(app.expert_hub.shiju_apps.store, ShijuAppsLoadState::Loading { .. }));
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::StoreLoaded {
        generation: store_generation,
        account_id: "company-1".to_string(),
        result: Ok(ShijuAppCatalogDto { apps: Vec::new() }),
    }));
    assert!(matches!(app.expert_hub.shiju_apps.store, ShijuAppsLoadState::Loaded(_)));
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

fn binding(configured: bool, can_edit: bool) -> ShijuAppApiKeyBindingDto {
    ShijuAppApiKeyBindingDto {
        personal: ShijuAppPersonalApiKeyBindingDto {
            configured,
            can_edit,
            api_key_id: configured.then_some(42),
            key_name: configured.then(|| "主密钥".to_string()),
            api_key_mask: configured.then(|| "sk-****42".to_string()),
            status: configured.then(|| "enabled".to_string()),
            expires_at: None,
            updated_at: None,
        },
    }
}

fn api_key(api_key_id: u64, status: &str) -> ShijuIamApiKeyDto {
    ShijuIamApiKeyDto {
        api_key_id,
        key_name: format!("Key {api_key_id}"),
        api_key_mask: format!("sk-****{api_key_id}"),
        status: status.to_string(),
        expires_at: None,
        updated_at: None,
    }
}

fn app_ready_for_launch() -> App {
    let (mut app, _) = App::new();
    app.account.replace_repository(repository("account-secret", "company-1"));
    app.expert_hub.shiju_apps.account_id = Some("company-1".to_string());
    app.expert_hub.shiju_apps.mine =
        ShijuAppsLoadState::Loaded(ShijuAppListDto { apps: vec![ready_app()] });
    app
}

#[test]
fn open_shiju_app_checks_binding_before_launch() {
    let mut app = app_ready_for_launch();

    let _task = app.update(Message::ShijuApps(ShijuAppsMessage::OpenInstalled(
        "01K1APP0000000000000000001".to_string(),
    )));
    let generation = app.expert_hub.shiju_apps.generation;
    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::CheckingBinding {
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
fn configured_binding_continues_existing_launch() {
    let mut app = app_ready_for_launch();
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::OpenInstalled(
        "01K1APP0000000000000000001".to_string(),
    )));
    let generation = app.expert_hub.shiju_apps.generation;

    let _task = app.update(Message::ShijuApps(ShijuAppsMessage::BindingLoaded {
        generation,
        account_id: "company-1".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Ok(binding(true, true)),
    }));

    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::Preparing {
            generation: active_generation,
            ref installation_id,
        } if active_generation == generation
            && installation_id == "01K1APP0000000000000000001"
    ));
}

#[test]
fn unconfigured_binding_selects_first_enabled_key() {
    let mut app = app_ready_for_launch();
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::OpenInstalled(
        "01K1APP0000000000000000001".to_string(),
    )));
    let generation = app.expert_hub.shiju_apps.generation;
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::BindingLoaded {
        generation,
        account_id: "company-1".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Ok(binding(false, true)),
    }));
    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::LoadingApiKeys { generation: active_generation, .. }
            if active_generation == generation
    ));

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::ApiKeysLoaded {
        generation,
        account_id: "company-1".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Ok(ShijuIamApiKeyListDto {
            api_keys: vec![api_key(41, "disabled"), api_key(42, "enabled")],
        }),
    }));
    assert!(matches!(
        &app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::SelectingApiKey {
            api_keys,
            selected_api_key_id: Some(42),
            error: None,
            ..
        } if api_keys.len() == 2
    ));

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::RefreshApiKeys));
    assert!(app.expert_hub.shiju_apps.generation > generation);
    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::CheckingBinding { .. }
    ));
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::CancelApiKeyBinding));
    assert!(matches!(app.expert_hub.shiju_apps.launch, ShijuAppLaunchState::Idle));
}

#[test]
fn unconfigured_binding_rejects_read_only_or_missing_enabled_keys() {
    let mut app = app_ready_for_launch();
    app.expert_hub.shiju_apps.generation = 12;
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::CheckingBinding {
        generation: 12,
        installation_id: "01K1APP0000000000000000001".to_string(),
    };

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::BindingLoaded {
        generation: 12,
        account_id: "company-1".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Ok(binding(false, false)),
    }));
    assert!(matches!(
        &app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::SelectingApiKey {
            can_edit: false,
            selected_api_key_id: None,
            error: Some(error),
            ..
        } if error.contains("不可修改")
    ));

    app.expert_hub.shiju_apps.generation = 13;
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::LoadingApiKeys {
        generation: 13,
        installation_id: "01K1APP0000000000000000001".to_string(),
        can_edit: true,
    };
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::ApiKeysLoaded {
        generation: 13,
        account_id: "company-1".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Ok(ShijuIamApiKeyListDto { api_keys: vec![api_key(41, "disabled")] }),
    }));
    assert!(matches!(
        &app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::SelectingApiKey {
            selected_api_key_id: None,
            error: Some(error),
            ..
        } if error.contains("没有已启用")
    ));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("没有已启用")));

    app.expert_hub.shiju_apps.generation = 14;
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::CheckingBinding {
        generation: 14,
        installation_id: "01K1APP0000000000000000001".to_string(),
    };
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::BindingLoaded {
        generation: 14,
        account_id: "company-1".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Err("gateway unavailable".to_string()),
    }));
    assert!(matches!(
        &app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::Error { message, .. } if message == "gateway unavailable"
    ));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("绑定检查失败")));
}

#[test]
fn binding_save_rejects_missing_or_disabled_selection() {
    let mut app = app_ready_for_launch();
    app.expert_hub.shiju_apps.generation = 4;
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::SelectingApiKey {
        generation: 4,
        installation_id: "01K1APP0000000000000000001".to_string(),
        can_edit: true,
        api_keys: vec![api_key(41, "disabled"), api_key(42, "enabled")],
        selected_api_key_id: None,
        error: None,
    };

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::SaveApiKeyBinding));
    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::SelectingApiKey { selected_api_key_id: None, .. }
    ));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("选择")));

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::SelectApiKey(41)));
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::SaveApiKeyBinding));
    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::SelectingApiKey { selected_api_key_id: Some(41), .. }
    ));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("启用")));

    if let ShijuAppLaunchState::SelectingApiKey { can_edit, .. } =
        &mut app.expert_hub.shiju_apps.launch
    {
        *can_edit = false;
    }
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::SelectApiKey(42)));
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::SaveApiKeyBinding));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("不可修改")));

    app.expert_hub.shiju_apps.generation = 5;
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::SelectingApiKey {
        generation: 4,
        installation_id: "01K1APP0000000000000000001".to_string(),
        can_edit: true,
        api_keys: vec![api_key(42, "enabled")],
        selected_api_key_id: Some(42),
        error: None,
    };
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::SaveApiKeyBinding));
    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::SelectingApiKey { generation: 4, .. }
    ));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("状态已失效")));
}

#[test]
fn binding_save_success_continues_launch_and_ignores_stale_results() {
    let mut app = app_ready_for_launch();
    app.expert_hub.shiju_apps.generation = 9;
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::SelectingApiKey {
        generation: 9,
        installation_id: "01K1APP0000000000000000001".to_string(),
        can_edit: true,
        api_keys: vec![api_key(42, "enabled"), api_key(43, "enabled")],
        selected_api_key_id: Some(42),
        error: None,
    };

    let _task = app.update(Message::ShijuApps(ShijuAppsMessage::SaveApiKeyBinding));
    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::SavingApiKeyBinding { selected_api_key_id: 42, .. }
    ));
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::SelectApiKey(43)));
    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::SavingApiKeyBinding { selected_api_key_id: 42, .. }
    ));

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::ApiKeyBindingSaved {
        generation: 8,
        account_id: "company-1".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Ok(()),
    }));
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::ApiKeyBindingSaved {
        generation: 9,
        account_id: "wrong-account".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Ok(()),
    }));
    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::SavingApiKeyBinding { .. }
    ));

    let _task = app.update(Message::ShijuApps(ShijuAppsMessage::ApiKeyBindingSaved {
        generation: 9,
        account_id: "company-1".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Ok(()),
    }));
    assert!(matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::Preparing { generation: 9, .. }
    ));
}

#[test]
fn binding_failures_preserve_selection_and_close_detail_cancels_stale_work() {
    let mut app = app_ready_for_launch();
    app.expert_hub.shiju_apps.generation = 11;
    app.expert_hub.shiju_apps.detail = Some(crate::app::shiju_apps::ShijuAppDetailTarget::Mine(0));
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::SavingApiKeyBinding {
        generation: 11,
        installation_id: "01K1APP0000000000000000001".to_string(),
        can_edit: true,
        api_keys: vec![api_key(42, "enabled")],
        selected_api_key_id: 42,
    };

    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::ApiKeyBindingSaved {
        generation: 11,
        account_id: "company-1".to_string(),
        installation_id: "01K1APP0000000000000000001".to_string(),
        result: Err("gateway unavailable".to_string()),
    }));
    assert!(matches!(
        &app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::SelectingApiKey {
            selected_api_key_id: Some(42),
            error: Some(error),
            ..
        } if error == "gateway unavailable"
    ));
    assert!(app.active_toast.as_ref().is_some_and(|toast| toast.message.contains("保存失败")));

    let generation = app.expert_hub.shiju_apps.generation;
    let _ = app.update(Message::ShijuApps(ShijuAppsMessage::CloseDetail));
    assert!(app.expert_hub.shiju_apps.detail.is_none());
    assert!(app.expert_hub.shiju_apps.generation > generation);
    assert!(matches!(app.expert_hub.shiju_apps.launch, ShijuAppLaunchState::Idle));
}

#[test]
fn stale_or_wrong_account_launch_is_ignored() {
    let mut app = app_ready_for_launch();
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
