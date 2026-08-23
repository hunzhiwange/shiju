//! 驱动识矩应用目录、已安装列表、密钥绑定与启动流程的消息生命周期。

use iced::Task;
use vw_gateway_client::{
    ShijuAppApiKeyBindingDto, ShijuAppApiKeyBindingSaveRequest, ShijuAppCatalogDto,
    ShijuAppLaunchDto, ShijuAppListDto, ShijuIamApiKeyListDto,
};

use crate::app::session_gateway::{
    gateway_shiju_app_api_key_binding_async, gateway_shiju_app_api_keys_async,
    gateway_shiju_app_catalog_async, gateway_shiju_app_prepare_launch_async,
    gateway_shiju_app_save_api_key_binding_async, gateway_shiju_apps_async,
};
use crate::app::shiju_apps::{
    ShijuAppDetailTarget, ShijuAppLaunchState, ShijuAppsLoadState, ShijuAppsTab,
};
use crate::app::{App, AppTab, Message, Screen};

/// 识矩应用在主标签栏中的稳定标识。
pub(crate) const SHIJU_APPS_TAB_ID: &str = "shiju_apps";
/// 识矩应用标签的用户可见标题。
pub(crate) const SHIJU_APPS_TITLE: &str = "识矩应用";

#[derive(Debug, Clone)]
/// 识矩应用目录、详情、密钥绑定和启动流程的消息集合。
pub(crate) enum ShijuAppsMessage {
    Open,
    SelectTab(ShijuAppsTab),
    OpenDetail(ShijuAppDetailTarget),
    CloseDetail,
    OpenInstalled(String),
    BindingLoaded {
        generation: u64,
        account_id: String,
        installation_id: String,
        result: Result<ShijuAppApiKeyBindingDto, String>,
    },
    ApiKeysLoaded {
        generation: u64,
        account_id: String,
        installation_id: String,
        result: Result<ShijuIamApiKeyListDto, String>,
    },
    #[allow(dead_code)] // Constructed by the API key binding overlay in the next batch.
    SelectApiKey(u64),
    #[allow(dead_code)] // Constructed by the API key binding overlay in the next batch.
    RefreshApiKeys,
    #[allow(dead_code)] // Constructed by the API key binding overlay in the next batch.
    SaveApiKeyBinding,
    ApiKeyBindingSaved {
        generation: u64,
        account_id: String,
        installation_id: String,
        result: Result<(), String>,
    },
    #[allow(dead_code)] // Constructed by the API key binding overlay in the next batch.
    CancelApiKeyBinding,
    LaunchPrepared {
        generation: u64,
        account_id: String,
        installation_id: String,
        result: Result<ShijuAppLaunchDto, String>,
    },
    Retry,
    ReloadAfterAccountChange,
    StoreLoaded {
        generation: u64,
        account_id: String,
        result: Result<ShijuAppCatalogDto, String>,
    },
    MineLoaded {
        generation: u64,
        account_id: String,
        result: Result<ShijuAppListDto, String>,
    },
}

/// 应用识矩应用状态转换，并忽略不属于当前请求代次的异步结果。
pub(crate) fn update(app: &mut App, message: ShijuAppsMessage) -> Task<Message> {
    match message {
        ShijuAppsMessage::Open => {
            app.expert_hub.install_guide.close();
            app.expert_hub.shiju_apps.tab = ShijuAppsTab::Mine;
            open_shiju_apps_tab(app);
            ensure_current_tab_loaded(app)
        }
        ShijuAppsMessage::SelectTab(tab) => {
            app.expert_hub.shiju_apps.tab = tab;
            app.expert_hub.shiju_apps.detail = None;
            ensure_current_tab_loaded(app)
        }
        ShijuAppsMessage::OpenDetail(target) => {
            app.expert_hub.shiju_apps.detail = Some(target);
            Task::none()
        }
        ShijuAppsMessage::CloseDetail => {
            app.expert_hub.shiju_apps.detail = None;
            cancel_api_key_binding(app);
            Task::none()
        }
        ShijuAppsMessage::OpenInstalled(installation_id) => {
            prepare_installed_launch(app, installation_id)
        }
        ShijuAppsMessage::BindingLoaded {
            generation,
            account_id,
            installation_id,
            result,
        } => finish_binding_check(app, generation, &account_id, &installation_id, result),
        ShijuAppsMessage::ApiKeysLoaded {
            generation,
            account_id,
            installation_id,
            result,
        } => finish_api_key_load(app, generation, &account_id, &installation_id, result),
        ShijuAppsMessage::SelectApiKey(api_key_id) => {
            select_api_key(app, api_key_id);
            Task::none()
        }
        ShijuAppsMessage::RefreshApiKeys => refresh_api_keys(app),
        ShijuAppsMessage::SaveApiKeyBinding => save_api_key_binding(app),
        ShijuAppsMessage::ApiKeyBindingSaved {
            generation,
            account_id,
            installation_id,
            result,
        } => finish_api_key_binding_save(
            app,
            generation,
            &account_id,
            &installation_id,
            result,
        ),
        ShijuAppsMessage::CancelApiKeyBinding => {
            cancel_api_key_binding(app);
            Task::none()
        }
        ShijuAppsMessage::LaunchPrepared { generation, account_id, installation_id, result } => {
            finish_installed_launch(app, generation, &account_id, &installation_id, result)
        }
        ShijuAppsMessage::Retry | ShijuAppsMessage::ReloadAfterAccountChange => {
            load_current_tab(app)
        }
        ShijuAppsMessage::StoreLoaded { generation, account_id, result } => {
            if app.expert_hub.shiju_apps.response_is_current(generation, &account_id) {
                app.expert_hub.shiju_apps.store = result
                    .map(ShijuAppsLoadState::Loaded)
                    .unwrap_or_else(ShijuAppsLoadState::Error);
            }
            Task::none()
        }
        ShijuAppsMessage::MineLoaded { generation, account_id, result } => {
            if app.expert_hub.shiju_apps.response_is_current(generation, &account_id) {
                app.expert_hub.shiju_apps.mine = result
                    .map(ShijuAppsLoadState::Loaded)
                    .unwrap_or_else(ShijuAppsLoadState::Error);
            }
            Task::none()
        }
    }
}

fn open_shiju_apps_tab(app: &mut App) {
    if !app.open_tabs.iter().any(|tab| tab.id == SHIJU_APPS_TAB_ID) {
        app.open_tabs.push(AppTab {
            id: SHIJU_APPS_TAB_ID.to_string(),
            title: SHIJU_APPS_TITLE.to_string(),
            screen: Screen::ShijuApps,
            project_path: None,
        });
    }
    app.active_tab_id = Some(SHIJU_APPS_TAB_ID.to_string());
    app.screen = Screen::ShijuApps;
}

fn prepare_installed_launch(app: &mut App, installation_id: String) -> Task<Message> {
    if !matches!(
        app.expert_hub.shiju_apps.launch,
        ShijuAppLaunchState::Idle | ShijuAppLaunchState::Error { .. }
    ) {
        return Task::none();
    }
    begin_binding_check(app, installation_id)
}

fn begin_binding_check(app: &mut App, installation_id: String) -> Task<Message> {
    let ready = matches!(
        &app.expert_hub.shiju_apps.mine,
        ShijuAppsLoadState::Loaded(apps)
            if apps.apps.iter().any(|item| {
                item.installation_id == installation_id
                    && item.reconcile_status == "ready"
                    && item.runtime_status == "ready"
            })
    );
    if !ready {
        return app.show_error_toast("识矩应用尚未运行就绪");
    }
    let Some(account) = app.account.active_account() else {
        return app.show_error_toast("登录识矩账户后打开应用");
    };
    let account_id = account.id().to_string();
    if app.expert_hub.shiju_apps.account_id.as_deref() != Some(account_id.as_str()) {
        return app.show_error_toast("识矩应用账户已变化，请刷新后重试");
    }
    let credential = account.skill_hub_credential();
    let generation = app.expert_hub.shiju_apps.next_generation();
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::CheckingBinding {
        generation,
        installation_id: installation_id.clone(),
    };
    let requested_installation_id = installation_id.clone();
    Task::perform(
        async move {
            let gateway_credential =
                credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_app_api_key_binding_async(
                &gateway_credential,
                &requested_installation_id,
            )
            .await
        },
        move |result| {
            Message::ShijuApps(ShijuAppsMessage::BindingLoaded {
                generation,
                account_id,
                installation_id,
                result,
            })
        },
    )
}

fn finish_binding_check(
    app: &mut App,
    generation: u64,
    account_id: &str,
    installation_id: &str,
    result: Result<ShijuAppApiKeyBindingDto, String>,
) -> Task<Message> {
    let current = launch_result_is_current(app, generation, account_id, installation_id)
        && matches!(
            app.expert_hub.shiju_apps.launch,
            ShijuAppLaunchState::CheckingBinding { .. }
        );
    if !current {
        return Task::none();
    }

    let binding = match result {
        Ok(binding) => binding,
        Err(error) => {
            set_launch_error(app, generation, installation_id, error.clone());
            return app.show_error_toast(format!("识矩应用密钥绑定检查失败：{error}"));
        }
    };
    if binding.personal.configured {
        return continue_installed_launch(
            app,
            generation,
            account_id.to_string(),
            installation_id.to_string(),
        );
    }

    if !binding.personal.can_edit {
        let message = "当前应用的个人 API 密钥绑定不可修改".to_string();
        app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::SelectingApiKey {
            generation,
            installation_id: installation_id.to_string(),
            can_edit: false,
            api_keys: Vec::new(),
            selected_api_key_id: None,
            error: Some(message.clone()),
        };
        return app.show_error_toast(message);
    }

    load_api_keys(app, generation, account_id.to_string(), installation_id.to_string(), true)
}

fn load_api_keys(
    app: &mut App,
    generation: u64,
    account_id: String,
    installation_id: String,
    can_edit: bool,
) -> Task<Message> {
    let Some(account) = app.account.active_account() else {
        return app.show_error_toast("登录识矩账户后选择 API 密钥");
    };
    if account.id() != account_id {
        return Task::none();
    }
    let credential = account.skill_hub_credential();
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::LoadingApiKeys {
        generation,
        installation_id: installation_id.clone(),
        can_edit,
    };
    Task::perform(
        async move {
            let gateway_credential =
                credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_app_api_keys_async(&gateway_credential).await
        },
        move |result| {
            Message::ShijuApps(ShijuAppsMessage::ApiKeysLoaded {
                generation,
                account_id,
                installation_id,
                result,
            })
        },
    )
}

fn finish_api_key_load(
    app: &mut App,
    generation: u64,
    account_id: &str,
    installation_id: &str,
    result: Result<ShijuIamApiKeyListDto, String>,
) -> Task<Message> {
    let can_edit = match &app.expert_hub.shiju_apps.launch {
        ShijuAppLaunchState::LoadingApiKeys {
            generation: active_generation,
            installation_id: active_installation_id,
            can_edit,
        } if *active_generation == generation && active_installation_id == installation_id => {
            *can_edit
        }
        _ => return Task::none(),
    };
    if !launch_result_is_current(app, generation, account_id, installation_id) {
        return Task::none();
    }

    let (api_keys, selected_api_key_id, error, toast_message) = match result {
        Ok(keys) => {
            let selected = keys
                .api_keys
                .iter()
                .find(|key| key.status == "enabled")
                .map(|key| key.api_key_id);
            let error = selected.is_none().then(|| {
                if keys.api_keys.is_empty() {
                    "当前账户没有可选择的 IAM API 密钥".to_string()
                } else {
                    "当前账户没有已启用的 IAM API 密钥".to_string()
                }
            });
            let toast_message = error.clone();
            (keys.api_keys, selected, error, toast_message)
        }
        Err(error) => {
            let toast_message = Some(format!("识矩 IAM API 密钥加载失败：{error}"));
            (Vec::new(), None, Some(error), toast_message)
        }
    };
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::SelectingApiKey {
        generation,
        installation_id: installation_id.to_string(),
        can_edit,
        api_keys,
        selected_api_key_id,
        error: error.clone(),
    };
    match toast_message {
        Some(message) => app.show_error_toast(message),
        None => Task::none(),
    }
}

fn select_api_key(app: &mut App, api_key_id: u64) {
    let ShijuAppLaunchState::SelectingApiKey {
        can_edit,
        api_keys,
        selected_api_key_id,
        error,
        ..
    } = &mut app.expert_hub.shiju_apps.launch
    else {
        return;
    };
    if *can_edit && api_keys.iter().any(|key| key.api_key_id == api_key_id) {
        *selected_api_key_id = Some(api_key_id);
        *error = None;
    }
}

fn refresh_api_keys(app: &mut App) -> Task<Message> {
    let installation_id = match &app.expert_hub.shiju_apps.launch {
        ShijuAppLaunchState::SelectingApiKey { installation_id, .. }
        | ShijuAppLaunchState::Error { installation_id, .. } => installation_id.clone(),
        _ => return Task::none(),
    };
    begin_binding_check(app, installation_id)
}

fn save_api_key_binding(app: &mut App) -> Task<Message> {
    let (generation, installation_id, can_edit, api_keys, selected_api_key_id) =
        match &app.expert_hub.shiju_apps.launch {
            ShijuAppLaunchState::SelectingApiKey {
                generation,
                installation_id,
                can_edit,
                api_keys,
                selected_api_key_id,
                ..
            } => (
                *generation,
                installation_id.clone(),
                *can_edit,
                api_keys.clone(),
                *selected_api_key_id,
            ),
            _ => return Task::none(),
        };
    if !can_edit {
        return reject_api_key_save(app, "当前应用的个人 API 密钥绑定不可修改");
    }
    let Some(selected_api_key_id) = selected_api_key_id else {
        return reject_api_key_save(app, "请选择一个已启用的 IAM API 密钥");
    };
    if !api_keys
        .iter()
        .any(|key| key.api_key_id == selected_api_key_id && key.status == "enabled")
    {
        return reject_api_key_save(app, "所选 IAM API 密钥不存在或未启用");
    }

    let Some(account) = app.account.active_account() else {
        return app.show_error_toast("登录识矩账户后保存 API 密钥绑定");
    };
    let account_id = account.id().to_string();
    if app.expert_hub.shiju_apps.account_id.as_deref() != Some(account_id.as_str()) {
        return app.show_error_toast("识矩应用账户已变化，请刷新后重试");
    }
    if !launch_result_is_current(app, generation, &account_id, &installation_id) {
        return reject_api_key_save(app, "识矩应用授权状态已失效，请刷新后重试");
    }
    let credential = account.skill_hub_credential();
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::SavingApiKeyBinding {
        generation,
        installation_id: installation_id.clone(),
        can_edit,
        api_keys,
        selected_api_key_id,
    };
    let requested_installation_id = installation_id.clone();
    Task::perform(
        async move {
            let gateway_credential =
                credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_app_save_api_key_binding_async(
                &gateway_credential,
                &requested_installation_id,
                &ShijuAppApiKeyBindingSaveRequest { api_key_id: selected_api_key_id },
            )
            .await
        },
        move |result| {
            Message::ShijuApps(ShijuAppsMessage::ApiKeyBindingSaved {
                generation,
                account_id,
                installation_id,
                result,
            })
        },
    )
}

fn reject_api_key_save(app: &mut App, message: &str) -> Task<Message> {
    if let ShijuAppLaunchState::SelectingApiKey { error, .. } =
        &mut app.expert_hub.shiju_apps.launch
    {
        *error = Some(message.to_string());
    }
    app.show_error_toast(message)
}

fn finish_api_key_binding_save(
    app: &mut App,
    generation: u64,
    account_id: &str,
    installation_id: &str,
    result: Result<(), String>,
) -> Task<Message> {
    let (can_edit, api_keys, selected_api_key_id) =
        match &app.expert_hub.shiju_apps.launch {
            ShijuAppLaunchState::SavingApiKeyBinding {
                generation: active_generation,
                installation_id: active_installation_id,
                can_edit,
                api_keys,
                selected_api_key_id,
            } if *active_generation == generation && active_installation_id == installation_id => {
                (*can_edit, api_keys.clone(), *selected_api_key_id)
            }
            _ => return Task::none(),
        };
    if !launch_result_is_current(app, generation, account_id, installation_id) {
        return Task::none();
    }

    match result {
        Ok(()) => continue_installed_launch(
            app,
            generation,
            account_id.to_string(),
            installation_id.to_string(),
        ),
        Err(error) => {
            app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::SelectingApiKey {
                generation,
                installation_id: installation_id.to_string(),
                can_edit,
                api_keys,
                selected_api_key_id: Some(selected_api_key_id),
                error: Some(error.clone()),
            };
            app.show_error_toast(format!("识矩应用 API 密钥绑定保存失败：{error}"))
        }
    }
}

fn continue_installed_launch(
    app: &mut App,
    generation: u64,
    account_id: String,
    installation_id: String,
) -> Task<Message> {
    if !launch_result_is_current(app, generation, &account_id, &installation_id) {
        return Task::none();
    }
    let Some(account) = app.account.active_account() else {
        return app.show_error_toast("登录识矩账户后打开应用");
    };
    let credential = account.skill_hub_credential();
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::Preparing {
        generation,
        installation_id: installation_id.clone(),
    };
    let requested_installation_id = installation_id.clone();
    Task::perform(
        async move {
            let gateway_credential =
                credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_app_prepare_launch_async(&gateway_credential, &requested_installation_id)
                .await
        },
        move |result| {
            Message::ShijuApps(ShijuAppsMessage::LaunchPrepared {
                generation,
                account_id,
                installation_id,
                result,
            })
        },
    )
}

fn launch_result_is_current(
    app: &App,
    generation: u64,
    account_id: &str,
    installation_id: &str,
) -> bool {
    app.expert_hub.shiju_apps.launch_response_is_current(
        generation,
        account_id,
        installation_id,
    ) && app.account.active_account().is_some_and(|account| account.id() == account_id)
}

fn set_launch_error(app: &mut App, generation: u64, installation_id: &str, message: String) {
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::Error {
        generation,
        installation_id: installation_id.to_string(),
        message,
    };
}

fn cancel_api_key_binding(app: &mut App) {
    if !matches!(app.expert_hub.shiju_apps.launch, ShijuAppLaunchState::Idle) {
        app.expert_hub.shiju_apps.next_generation();
        app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::Idle;
    }
}

fn finish_installed_launch(
    app: &mut App,
    generation: u64,
    account_id: &str,
    installation_id: &str,
    result: Result<ShijuAppLaunchDto, String>,
) -> Task<Message> {
    let current = launch_result_is_current(app, generation, account_id, installation_id)
        && matches!(
            &app.expert_hub.shiju_apps.launch,
            ShijuAppLaunchState::Preparing {
                generation: active_generation,
                installation_id: active_installation_id,
            } if *active_generation == generation && active_installation_id == installation_id
        );
    if !current {
        return Task::none();
    }

    let target = match result {
        Ok(target) => target,
        Err(error) => {
            set_launch_error(app, generation, installation_id, error.clone());
            return app.show_error_toast(format!("识矩应用启动预检失败：{error}"));
        }
    };
    #[cfg(not(target_arch = "wasm32"))]
    match crate::app::shiju_apps::launch_shiju_app(&target, installation_id) {
        Ok(child) => {
            app.independent_webview_children.push(child);
            app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::Idle;
            app.show_success_toast("识矩应用已安全打开")
        }
        Err(error) => {
            set_launch_error(app, generation, installation_id, error.clone());
            app.show_error_toast(error)
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = target;
        set_launch_error(
            app,
            generation,
            installation_id,
            "当前平台不支持识矩应用 WebView".to_string(),
        );
        app.show_error_toast("当前平台不支持识矩应用 WebView")
    }
}

fn ensure_current_tab_loaded(app: &mut App) -> Task<Message> {
    let needs_load = match app.expert_hub.shiju_apps.tab {
        ShijuAppsTab::Store => !matches!(
            app.expert_hub.shiju_apps.store,
            ShijuAppsLoadState::Loading { .. } | ShijuAppsLoadState::Loaded(_)
        ),
        ShijuAppsTab::Mine => !matches!(
            app.expert_hub.shiju_apps.mine,
            ShijuAppsLoadState::Loading { .. } | ShijuAppsLoadState::Loaded(_)
        ),
    };
    if needs_load { load_current_tab(app) } else { Task::none() }
}

fn load_current_tab(app: &mut App) -> Task<Message> {
    let active = app
        .account
        .active_account()
        .map(|account| (account.id().to_string(), account.skill_hub_credential()));
    let Some((account_id, credential)) = active else {
        if app.expert_hub.shiju_apps.account_id.is_some() {
            app.expert_hub.shiju_apps.clear_for_account(None);
        }
        match app.expert_hub.shiju_apps.tab {
            ShijuAppsTab::Store => {
                app.expert_hub.shiju_apps.store = ShijuAppsLoadState::NeedsLogin;
            }
            ShijuAppsTab::Mine => {
                app.expert_hub.shiju_apps.mine = ShijuAppsLoadState::NeedsLogin;
            }
        }
        return Task::none();
    };

    if app.expert_hub.shiju_apps.account_id.as_deref() != Some(account_id.as_str()) {
        app.expert_hub.shiju_apps.clear_for_account(Some(account_id.clone()));
    }
    let generation = app.expert_hub.shiju_apps.next_generation();
    match app.expert_hub.shiju_apps.tab {
        ShijuAppsTab::Store => {
            app.expert_hub.shiju_apps.store = ShijuAppsLoadState::Loading { generation };
            Task::perform(
                async move {
                    let credential =
                        credential.gateway_credential().map_err(|error| error.to_string())?;
                    gateway_shiju_app_catalog_async(&credential).await
                },
                move |result| {
                    Message::ShijuApps(ShijuAppsMessage::StoreLoaded {
                        generation,
                        account_id,
                        result,
                    })
                },
            )
        }
        ShijuAppsTab::Mine => {
            app.expert_hub.shiju_apps.mine = ShijuAppsLoadState::Loading { generation };
            Task::perform(
                async move {
                    let credential =
                        credential.gateway_credential().map_err(|error| error.to_string())?;
                    gateway_shiju_apps_async(&credential).await
                },
                move |result| {
                    Message::ShijuApps(ShijuAppsMessage::MineLoaded {
                        generation,
                        account_id,
                        result,
                    })
                },
            )
        }
    }
}

#[cfg(test)]
#[path = "shiju_apps_tests.rs"]
mod shiju_apps_tests;
