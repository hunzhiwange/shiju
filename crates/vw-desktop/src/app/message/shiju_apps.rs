//! SHIJU application desktop lifecycle messages.

use iced::Task;
use vw_gateway_client::{ShijuAppCatalogDto, ShijuAppLaunchDto, ShijuAppListDto};

use crate::app::session_gateway::{
    gateway_shiju_app_catalog_async, gateway_shiju_app_prepare_launch_async,
    gateway_shiju_apps_async,
};
use crate::app::shiju_apps::{
    ShijuAppDetailTarget, ShijuAppLaunchState, ShijuAppsLoadState, ShijuAppsTab,
};
use crate::app::{App, AppTab, Message, Screen};

pub(crate) const SHIJU_APPS_TAB_ID: &str = "shiju_apps";
pub(crate) const SHIJU_APPS_TITLE: &str = "识矩应用";

#[derive(Debug, Clone)]
pub(crate) enum ShijuAppsMessage {
    Open,
    SelectTab(ShijuAppsTab),
    OpenDetail(ShijuAppDetailTarget),
    CloseDetail,
    OpenInstalled(String),
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
            Task::none()
        }
        ShijuAppsMessage::OpenInstalled(installation_id) => {
            prepare_installed_launch(app, installation_id)
        }
        ShijuAppsMessage::LaunchPrepared {
            generation,
            account_id,
            installation_id,
            result,
        } => finish_installed_launch(app, generation, &account_id, &installation_id, result),
        ShijuAppsMessage::Retry | ShijuAppsMessage::ReloadAfterAccountChange => {
            load_current_tab(app)
        }
        ShijuAppsMessage::StoreLoaded {
            generation,
            account_id,
            result,
        } => {
            if app
                .expert_hub
                .shiju_apps
                .response_is_current(generation, &account_id)
            {
                app.expert_hub.shiju_apps.store = result
                    .map(ShijuAppsLoadState::Loaded)
                    .unwrap_or_else(ShijuAppsLoadState::Error);
            }
            Task::none()
        }
        ShijuAppsMessage::MineLoaded {
            generation,
            account_id,
            result,
        } => {
            if app
                .expert_hub
                .shiju_apps
                .response_is_current(generation, &account_id)
            {
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
    if matches!(app.expert_hub.shiju_apps.launch, ShijuAppLaunchState::Preparing { .. }) {
        return Task::none();
    }
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
    app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::Preparing {
        generation,
        installation_id: installation_id.clone(),
    };
    let requested_installation_id = installation_id.clone();
    Task::perform(
        async move {
            let gateway_credential =
                credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_app_prepare_launch_async(
                &gateway_credential,
                &requested_installation_id,
            )
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

fn finish_installed_launch(
    app: &mut App,
    generation: u64,
    account_id: &str,
    installation_id: &str,
    result: Result<ShijuAppLaunchDto, String>,
) -> Task<Message> {
    let current = app.expert_hub.shiju_apps.response_is_current(generation, account_id)
        && app.account.active_account().is_some_and(|account| account.id() == account_id)
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
            app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::Error(error.clone());
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
            app.expert_hub.shiju_apps.launch = ShijuAppLaunchState::Error(error.clone());
            app.show_error_toast(error)
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let _ = target;
        app.expert_hub.shiju_apps.launch =
            ShijuAppLaunchState::Error("当前平台不支持识矩应用 WebView".to_string());
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
    if needs_load {
        load_current_tab(app)
    } else {
        Task::none()
    }
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
        app.expert_hub
            .shiju_apps
            .clear_for_account(Some(account_id.clone()));
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
