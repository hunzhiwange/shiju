//! 驱动识矩应用目录、已安装列表、密钥绑定与启动流程的消息生命周期。

use gateway_shiju_app::{
    ShijuAppApiKeyBindingDto, ShijuAppApiKeyBindingSaveRequest, ShijuAppCatalogDto,
    ShijuAppLaunchDto, ShijuAppListDto, ShijuIamApiKeyListDto,
};
use iced::Task;

use self::requests::{
    gateway_shiju_app_api_key_binding_async, gateway_shiju_app_api_keys_async,
    gateway_shiju_app_catalog_async, gateway_shiju_app_prepare_launch_async,
    gateway_shiju_app_save_api_key_binding_async, gateway_shiju_apps_async,
};
use desktop_app_launch::{
    ShijuAppDetailTarget, ShijuAppLaunchState, ShijuAppsLoadState, ShijuAppsTab,
};
use desktop_app_launch::{ShijuAppsState, ShijuDetailMetadata, ShijuMarketplaceAction};
use gateway_shiju_app::{
    GatewayShijuAppClient, ShijuAppCategory, ShijuCreatedApp, ShijuFeaturedApp,
    ShijuMarketplaceList, valid_installation_id,
};

/// 识矩应用在主标签栏中的稳定标识。
pub const SHIJU_APPS_TAB_ID: &str = "shiju_apps";
/// 识矩应用标签的用户可见标题。
pub const SHIJU_APPS_TITLE: &str = "识矩应用";

#[derive(Debug, Clone)]
/// 识矩应用目录、详情、密钥绑定和启动流程的消息集合。
pub enum ShijuAppsMessage {
    Open,
    OpenStudio,
    SelectTab(ShijuAppsTab),
    OpenDetail(ShijuAppDetailTarget),
    CloseDetail,
    DetailAction(operations::DetailAction),
    Search(String),
    Category(String),
    Source(String),
    Recent,
    Lifecycle(String),
    Page(usize),
    PageSize(usize),
    MarketplaceAction(ShijuMarketplaceAction),
    MarketplaceActionFinished {
        generation: u64,
        account_id: String,
        result: Result<(), String>,
    },
    FeaturedLoaded {
        generation: u64,
        account_id: String,
        result: Result<Vec<ShijuFeaturedApp>, String>,
    },
    CategoriesLoaded {
        account_id: String,
        generation: u64,
        result: Result<Vec<ShijuAppCategory>, String>,
    },
    CreatedLoaded {
        generation: u64,
        account_id: String,
        result: Result<Vec<ShijuCreatedApp>, String>,
    },
    DetailMetadataLoaded {
        generation: u64,
        account_id: String,
        result: Result<ShijuDetailMetadata, String>,
    },
    DetailBindingLoaded {
        generation: u64,
        account_id: String,
        result: Result<ShijuAppApiKeyBindingDto, String>,
    },
    ArtworkLoaded {
        account_id: String,
        items: Vec<(u64, Option<iced::widget::image::Handle>)>,
    },
    RefreshDetailBinding,
    ChangeBinding,
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
pub fn update(app: &mut Input<'_>, message: ShijuAppsMessage) -> Task<ShijuAppsMessage> {
    match message {
        ShijuAppsMessage::Open => {
            app.effects.push(Effect::Open);
            if app.shiju_apps.tab != ShijuAppsTab::Mine {
                reset_page(app);
                app.shiju_apps.tab = ShijuAppsTab::Mine;
            }

            ensure_current_tab_loaded(app)
        }
        ShijuAppsMessage::OpenStudio => open_studio(app),
        ShijuAppsMessage::SelectTab(tab) => {
            if app.shiju_apps.tab != tab {
                reset_page(app);
                app.shiju_apps.tab = tab;
            }
            ensure_current_tab_loaded(app)
        }
        ShijuAppsMessage::OpenDetail(target) => {
            app.shiju_apps.detail = Some(target);
            Task::batch([load_detail_metadata(app), load_detail_binding(app)])
        }
        ShijuAppsMessage::DetailAction(event) => operations::update(app, event),
        ShijuAppsMessage::CloseDetail => {
            app.shiju_apps.copy_form = None;
            app.shiju_apps.detail = None;
            app.shiju_apps.detail_metadata = ShijuAppsLoadState::Idle;
            app.shiju_apps.binding = ShijuAppsLoadState::Idle;
            cancel_api_key_binding(app);
            Task::none()
        }
        ShijuAppsMessage::Search(query) => {
            app.shiju_apps.browse.query = query;
            app.shiju_apps.browse.page = 0;
            Task::none()
        }
        ShijuAppsMessage::Category(code) => {
            app.shiju_apps.browse.category = code;
            app.shiju_apps.browse.recent = false;
            app.shiju_apps.browse.page = 0;
            Task::none()
        }
        ShijuAppsMessage::Source(source) => {
            app.shiju_apps.browse.source = source;
            app.shiju_apps.browse.page = 0;
            Task::none()
        }
        ShijuAppsMessage::Recent => {
            app.shiju_apps.browse.category.clear();
            app.shiju_apps.browse.recent = true;
            app.shiju_apps.browse.page = 0;
            Task::none()
        }
        ShijuAppsMessage::Lifecycle(status) => {
            app.shiju_apps.browse.lifecycle = status;
            app.shiju_apps.browse.page = 0;
            Task::none()
        }
        ShijuAppsMessage::Page(page) => {
            app.shiju_apps.browse.page = page;
            Task::none()
        }
        ShijuAppsMessage::PageSize(size) => {
            if matches!(size, 10 | 20 | 50 | 100) {
                app.shiju_apps.browse.page_size = size;
                app.shiju_apps.browse.page = 0;
            }
            Task::none()
        }
        ShijuAppsMessage::MarketplaceAction(action) => marketplace_action(app, action),
        ShijuAppsMessage::MarketplaceActionFinished { generation, account_id, result } => {
            finish_marketplace_action(app, generation, &account_id, result)
        }
        ShijuAppsMessage::CreatedLoaded { generation, account_id, result } => {
            accept_list(app.account, &account_id, generation, &mut app.shiju_apps.created, result);
            artwork::load(app)
        }
        ShijuAppsMessage::FeaturedLoaded { generation, account_id, result } => {
            accept_list(app.account, &account_id, generation, &mut app.shiju_apps.featured, result);
            artwork::load(app)
        }
        ShijuAppsMessage::CategoriesLoaded { account_id, generation, result } => {
            if app.account.is_some_and(|account| account.id() == account_id)
                && app.shiju_apps.featured_generation == generation
                && let Ok(items) = result {
                    app.shiju_apps.categories = items;
                }
            Task::none()
        }
        ShijuAppsMessage::ArtworkLoaded { account_id, items } => {
            if app.shiju_apps.account_id.as_deref() == Some(account_id.as_str())
                && app.account.is_some_and(|account| account.id() == account_id)
            {
                for (key, artwork) in items {
                    if let Some(slot) = app.shiju_apps.artwork.get_mut(&key) {
                        *slot = artwork;
                    }
                }
            }
            Task::none()
        }
        ShijuAppsMessage::RefreshDetailBinding => load_detail_binding(app),
        ShijuAppsMessage::DetailMetadataLoaded { generation, account_id, result } => {
            accept_list(
                app.account,
                &account_id,
                generation,
                &mut app.shiju_apps.detail_metadata,
                result,
            );
            artwork::load(app)
        }
        ShijuAppsMessage::DetailBindingLoaded { generation, account_id, result } => {
            accept_list(app.account, &account_id, generation, &mut app.shiju_apps.binding, result);
            Task::none()
        }
        ShijuAppsMessage::ChangeBinding => change_binding(app),
        ShijuAppsMessage::OpenInstalled(installation_id) => {
            prepare_installed_launch(app, installation_id)
        }
        ShijuAppsMessage::BindingLoaded { generation, account_id, installation_id, result } => {
            finish_binding_check(app, generation, &account_id, &installation_id, result)
        }
        ShijuAppsMessage::ApiKeysLoaded { generation, account_id, installation_id, result } => {
            finish_api_key_load(app, generation, &account_id, &installation_id, result)
        }
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
        } => finish_api_key_binding_save(app, generation, &account_id, &installation_id, result),
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
            accept_list(app.account, &account_id, generation, &mut app.shiju_apps.store, result);
            artwork::load(app)
        }
        ShijuAppsMessage::MineLoaded { generation, account_id, result } => {
            let studio_lookup = app.shiju_apps.studio_lookup_generation == Some(generation)
                && app.shiju_apps.account_id.as_deref() == Some(account_id.as_str())
                && app.account.is_some_and(|account| account.id() == account_id)
                && matches!(&app.shiju_apps.mine, ShijuAppsLoadState::Loading { generation: current } if *current == generation);
            accept_list(app.account, &account_id, generation, &mut app.shiju_apps.mine, result);
            if studio_lookup {
                app.shiju_apps.studio_lookup_generation = None;
                let open = launch_studio_from_mine(app);
                return Task::batch([open, artwork::load(app)]);
            }
            artwork::load(app)
        }
    }
}

fn open_studio(app: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    let Some(account) = app.account else {
        return app.show_localized_toast((), "请先登录识矩账户，再配置应用 Studio", vec![]);
    };
    let account_id = account.id().to_owned();
    if app.shiju_apps.account_id.as_deref() != Some(account_id.as_str()) {
        app.shiju_apps.clear_for_account(Some(account_id.clone()));
    }
    match &app.shiju_apps.mine {
        ShijuAppsLoadState::Loaded(_) => return launch_studio_from_mine(app),
        ShijuAppsLoadState::Loading { generation } => {
            app.shiju_apps.studio_lookup_generation = Some(*generation);
            return Task::none();
        }
        _ => {}
    }
    let generation = app.shiju_apps.next_generation();
    app.shiju_apps.studio_lookup_generation = Some(generation);
    app.shiju_apps.mine = ShijuAppsLoadState::Loading { generation };
    load_mine(app.client.clone(), account.skill_hub_credential(), account_id, generation)
}

fn launch_studio_from_mine(app: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    const STUDIO_APP_ID: &str = "app-studio";
    let installation_id = match &app.shiju_apps.mine {
        ShijuAppsLoadState::Loaded(mine) => mine.apps.iter().find_map(|item| {
            (item.app_id == STUDIO_APP_ID && valid_installation_id(&item.installation_id))
                .then(|| item.installation_id.clone())
        }),
        _ => return app.show_localized_toast((), "无法读取应用 Studio 的安装状态，请重试", vec![]),
    };
    let Some(installation_id) = installation_id else {
        return app.show_localized_toast((), "请在识矩应用中安装或配置应用 Studio", vec![]);
    };
    if app.shiju_apps.tab != ShijuAppsTab::Mine {
        app.shiju_apps.tab = ShijuAppsTab::Mine;
        app.shiju_apps.browse = Default::default();
    }
    app.effects.push(Effect::Open);
    prepare_installed_launch(app, installation_id)
}

fn prepare_installed_launch(
    app: &mut Input<'_>,
    installation_id: String,
) -> Task<ShijuAppsMessage> {
    if !matches!(
        app.shiju_apps.launch,
        ShijuAppLaunchState::Idle | ShijuAppLaunchState::Error { .. }
    ) {
        return Task::none();
    }
    app.shiju_apps.changing_binding = false;
    begin_binding_check(app, installation_id)
}

fn begin_binding_check(app: &mut Input<'_>, installation_id: String) -> Task<ShijuAppsMessage> {
    let installed = matches!(
        &app.shiju_apps.mine,
        ShijuAppsLoadState::Loaded(apps)
            if apps.apps.iter().any(|item| item.installation_id == installation_id)
    );
    if !valid_installation_id(&installation_id) || (!installed && !app.shiju_apps.changing_binding) {
        return app.show_localized_toast((), "应用条目不存在", vec![]);
    }
    let Some(account) = app.account else {
        return app.show_localized_toast((), "登录识矩账户后打开应用", vec![]);
    };
    let account_id = account.id().to_string();
    if app.shiju_apps.account_id.as_deref() != Some(account_id.as_str()) {
        return app.show_localized_toast((), "识矩应用账户已变化，请刷新后重试", vec![]);
    }
    let credential = account.skill_hub_credential();
    let generation = app.shiju_apps.next_generation();
    app.shiju_apps.launch = ShijuAppLaunchState::CheckingBinding {
        generation,
        installation_id: installation_id.clone(),
    };
    let requested_installation_id = installation_id.clone();
    let client = app.client.clone();
    Task::perform(
        async move {
            let gateway_credential =
                credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_app_api_key_binding_async(
                client,
                &gateway_credential,
                &requested_installation_id,
            )
            .await
        },
        move |result| ShijuAppsMessage::BindingLoaded {
            generation,
            account_id,
            installation_id,
            result,
        },
    )
}

fn finish_binding_check(
    app: &mut Input<'_>,
    generation: u64,
    account_id: &str,
    installation_id: &str,
    result: Result<ShijuAppApiKeyBindingDto, String>,
) -> Task<ShijuAppsMessage> {
    let current = launch_result_is_current(app, generation, account_id, installation_id)
        && matches!(app.shiju_apps.launch, ShijuAppLaunchState::CheckingBinding { .. });
    if !current {
        return Task::none();
    }

    let binding = match result {
        Ok(binding) => binding,
        Err(error) => {
            set_launch_error(app, generation, installation_id, error.clone());
            return app.show_localized_toast(
                (),
                "识矩应用密钥绑定检查失败：%s",
                vec![error.to_string().into()],
            );
        }
    };
    if binding.personal.configured && !app.shiju_apps.changing_binding {
        return continue_installed_launch(
            app,
            generation,
            account_id.to_string(),
            installation_id.to_string(),
        );
    }

    if !binding.personal.can_edit {
        let message = "当前应用的个人 API 密钥绑定不可修改".to_string();
        app.shiju_apps.launch = ShijuAppLaunchState::SelectingApiKey {
            generation,
            installation_id: installation_id.to_string(),
            can_edit: false,
            api_keys: Vec::new(),
            selected_api_key_id: None,
            error: Some(message.clone()),
        };
        app.shiju_apps.launch_feedback = Some((
            generation,
            message.clone(),
            desktop_skill_hub::SkillHubError::localized(&message, vec![]),
        ));
        return app.show_localized_toast((), &message, vec![]);
    }

    load_api_keys(app, generation, account_id.to_string(), installation_id.to_string(), true)
}

fn load_api_keys(
    app: &mut Input<'_>,
    generation: u64,
    account_id: String,
    installation_id: String,
    can_edit: bool,
) -> Task<ShijuAppsMessage> {
    let Some(account) = app.account else {
        return app.show_localized_toast((), "登录识矩账户后选择 API 密钥", vec![]);
    };
    if account.id() != account_id {
        return Task::none();
    }
    let credential = account.skill_hub_credential();
    app.shiju_apps.launch = ShijuAppLaunchState::LoadingApiKeys {
        generation,
        installation_id: installation_id.clone(),
        can_edit,
    };
    let client = app.client.clone();
    Task::perform(
        async move {
            let gateway_credential =
                credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_app_api_keys_async(client, &gateway_credential).await
        },
        move |result| ShijuAppsMessage::ApiKeysLoaded {
            generation,
            account_id,
            installation_id,
            result,
        },
    )
}

fn finish_api_key_load(
    app: &mut Input<'_>,
    generation: u64,
    account_id: &str,
    installation_id: &str,
    result: Result<ShijuIamApiKeyListDto, String>,
) -> Task<ShijuAppsMessage> {
    let can_edit = match &app.shiju_apps.launch {
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

    let failed = result.is_err();
    let (api_keys, selected_api_key_id, error, toast_message) = match result {
        Ok(keys) => {
            let selected =
                keys.api_keys.iter().find(|key| key.status == "enabled").map(|key| key.api_key_id);
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
    app.shiju_apps.launch = ShijuAppLaunchState::SelectingApiKey {
        generation,
        installation_id: installation_id.to_string(),
        can_edit,
        api_keys,
        selected_api_key_id,
        error: error.clone(),
    };
    app.shiju_apps.launch_feedback = error.as_ref().filter(|_| !failed).map(|source| {
        (generation, source.clone(), desktop_skill_hub::SkillHubError::localized(source, vec![]))
    });
    match toast_message {
        Some(_) if failed => app.show_localized_toast(
            (),
            "识矩 IAM API 密钥加载失败：%s",
            vec![error.unwrap_or_default().into()],
        ),
        Some(message) => app.show_localized_toast((), &message, vec![]),
        None => Task::none(),
    }
}

fn select_api_key(app: &mut Input<'_>, api_key_id: u64) {
    let ShijuAppLaunchState::SelectingApiKey {
        can_edit, api_keys, selected_api_key_id, error, ..
    } = &mut app.shiju_apps.launch
    else {
        return;
    };
    if *can_edit && api_keys.iter().any(|key| key.api_key_id == api_key_id) {
        *selected_api_key_id = Some(api_key_id);
        *error = None;
    }
}

fn refresh_api_keys(app: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    let installation_id = match &app.shiju_apps.launch {
        ShijuAppLaunchState::SelectingApiKey { installation_id, .. }
        | ShijuAppLaunchState::Error { installation_id, .. } => installation_id.clone(),
        _ => return Task::none(),
    };
    begin_binding_check(app, installation_id)
}

fn save_api_key_binding(app: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    let (generation, installation_id, can_edit, api_keys, selected_api_key_id) =
        match &app.shiju_apps.launch {
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
    if !api_keys.iter().any(|key| key.api_key_id == selected_api_key_id && key.status == "enabled")
    {
        return reject_api_key_save(app, "所选 IAM API 密钥不存在或未启用");
    }

    let Some(account) = app.account else {
        return app.show_localized_toast((), "登录识矩账户后保存 API 密钥绑定", vec![]);
    };
    let account_id = account.id().to_string();
    if app.shiju_apps.account_id.as_deref() != Some(account_id.as_str()) {
        return app.show_localized_toast((), "识矩应用账户已变化，请刷新后重试", vec![]);
    }
    if !launch_result_is_current(app, generation, &account_id, &installation_id) {
        return reject_api_key_save(app, "识矩应用授权状态已失效，请刷新后重试");
    }
    let credential = account.skill_hub_credential();
    app.shiju_apps.launch = ShijuAppLaunchState::SavingApiKeyBinding {
        generation,
        installation_id: installation_id.clone(),
        can_edit,
        api_keys,
        selected_api_key_id,
    };
    let requested_installation_id = installation_id.clone();
    let client = app.client.clone();
    Task::perform(
        async move {
            let gateway_credential =
                credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_app_save_api_key_binding_async(
                client,
                &gateway_credential,
                &requested_installation_id,
                &ShijuAppApiKeyBindingSaveRequest { api_key_id: selected_api_key_id },
            )
            .await
        },
        move |result| ShijuAppsMessage::ApiKeyBindingSaved {
            generation,
            account_id,
            installation_id,
            result,
        },
    )
}

fn reject_api_key_save(app: &mut Input<'_>, message: &str) -> Task<ShijuAppsMessage> {
    if let ShijuAppLaunchState::SelectingApiKey { generation, error, .. } =
        &mut app.shiju_apps.launch
    {
        *error = Some(message.to_string());
        app.shiju_apps.launch_feedback = Some((
            *generation,
            message.to_string(),
            desktop_skill_hub::SkillHubError::localized(message, vec![]),
        ));
    }
    app.show_localized_toast((), message, vec![])
}

fn finish_api_key_binding_save(
    app: &mut Input<'_>,
    generation: u64,
    account_id: &str,
    installation_id: &str,
    result: Result<(), String>,
) -> Task<ShijuAppsMessage> {
    let (can_edit, api_keys, selected_api_key_id) = match &app.shiju_apps.launch {
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
        Ok(()) if app.shiju_apps.changing_binding => {
            cancel_api_key_binding(app);
            load_detail_binding(app)
        }
        Ok(()) => continue_installed_launch(
            app,
            generation,
            account_id.to_string(),
            installation_id.to_string(),
        ),
        Err(error) => {
            app.shiju_apps.launch = ShijuAppLaunchState::SelectingApiKey {
                generation,
                installation_id: installation_id.to_string(),
                can_edit,
                api_keys,
                selected_api_key_id: Some(selected_api_key_id),
                error: Some(error.clone()),
            };
            app.show_localized_toast(
                (),
                "识矩应用 API 密钥绑定保存失败：%s",
                vec![error.to_string().into()],
            )
        }
    }
}

fn continue_installed_launch(
    app: &mut Input<'_>,
    generation: u64,
    account_id: String,
    installation_id: String,
) -> Task<ShijuAppsMessage> {
    if !launch_result_is_current(app, generation, &account_id, &installation_id) {
        return Task::none();
    }
    let Some(account) = app.account else {
        return app.show_localized_toast((), "登录识矩账户后打开应用", vec![]);
    };
    let credential = account.skill_hub_credential();
    app.shiju_apps.launch =
        ShijuAppLaunchState::Preparing { generation, installation_id: installation_id.clone() };
    let requested_installation_id = installation_id.clone();
    let client = app.client.clone();
    Task::perform(
        async move {
            let gateway_credential =
                credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_app_prepare_launch_async(
                client,
                &gateway_credential,
                &requested_installation_id,
            )
            .await
        },
        move |result| ShijuAppsMessage::LaunchPrepared {
            generation,
            account_id,
            installation_id,
            result,
        },
    )
}

fn launch_result_is_current(
    app: &Input<'_>,
    generation: u64,
    account_id: &str,
    installation_id: &str,
) -> bool {
    app.shiju_apps.launch_response_is_current(generation, account_id, installation_id)
        && app.account.is_some_and(|account| account.id() == account_id)
}

fn set_launch_error(app: &mut Input<'_>, generation: u64, installation_id: &str, message: String) {
    app.shiju_apps.launch_feedback = None;
    app.shiju_apps.launch = ShijuAppLaunchState::Error {
        generation,
        installation_id: installation_id.to_string(),
        message,
    };
}

fn cancel_api_key_binding(app: &mut Input<'_>) {
    app.shiju_apps.changing_binding = false;
    if !matches!(app.shiju_apps.launch, ShijuAppLaunchState::Idle) {
        app.shiju_apps.next_generation();
        app.shiju_apps.launch = ShijuAppLaunchState::Idle;
    }
}

fn finish_installed_launch(
    app: &mut Input<'_>,
    generation: u64,
    account_id: &str,
    installation_id: &str,
    result: Result<ShijuAppLaunchDto, String>,
) -> Task<ShijuAppsMessage> {
    let current = launch_result_is_current(app, generation, account_id, installation_id)
        && matches!(
            &app.shiju_apps.launch,
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
            return app.show_localized_toast(
                (),
                "识矩应用启动预检失败：%s",
                vec![error.to_string().into()],
            );
        }
    };
    app.effects.push(Effect::Launch {
        target,
        generation,
        installation_id: installation_id.to_string(),
    });
    Task::none()
}

fn accept_list<T>(
    account: Option<&desktop_account::StoredAccount>,
    account_id: &str,
    generation: u64,
    state: &mut ShijuAppsLoadState<T>,
    result: Result<T, String>,
) {
    if account.is_some_and(|account| account.id() == account_id)
        && matches!(state, ShijuAppsLoadState::Loading { generation: current } if *current == generation)
    {
        *state = result.map(ShijuAppsLoadState::Loaded).unwrap_or_else(ShijuAppsLoadState::Error);
    }
}

fn reset_page(app: &mut Input<'_>) {
    app.shiju_apps.copy_form = None;
    app.shiju_apps.browse = Default::default();
    app.shiju_apps.detail = None;
    app.shiju_apps.detail_metadata = ShijuAppsLoadState::Idle;
    app.shiju_apps.binding = ShijuAppsLoadState::Idle;
    cancel_api_key_binding(app);
}

fn cached<T>(state: &ShijuAppsLoadState<T>) -> bool {
    matches!(state, ShijuAppsLoadState::Loading { .. } | ShijuAppsLoadState::Loaded(_))
}

fn ensure_current_tab_loaded(app: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    let needs_load = app.shiju_apps.account_id.as_deref()
        != app.account.map(|account| account.id())
        || match app.shiju_apps.tab {
            ShijuAppsTab::Store => !cached(&app.shiju_apps.store),
            ShijuAppsTab::Mine => {
                !cached(&app.shiju_apps.mine) || !cached(&app.shiju_apps.featured)
            }
            ShijuAppsTab::Favorites => !cached(&app.shiju_apps.mine),
            ShijuAppsTab::Created => {
                !cached(&app.shiju_apps.created) || !cached(&app.shiju_apps.mine)
            }
        };
    if needs_load { load_current_tab(app) } else { Task::none() }
}

fn load_current_tab(app: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    let Some(account) = app.account else {
        app.shiju_apps.clear_for_account(None);
        app.shiju_apps.store = ShijuAppsLoadState::NeedsLogin;
        app.shiju_apps.mine = ShijuAppsLoadState::NeedsLogin;
        app.shiju_apps.created = ShijuAppsLoadState::NeedsLogin;
        return Task::none();
    };

    let account_id = account.id().to_string();
    if app.shiju_apps.account_id.as_deref() != Some(account_id.as_str()) {
        app.shiju_apps.clear_for_account(Some(account_id.clone()));
    }
    app.shiju_apps.detail = None;
    app.shiju_apps.detail_metadata = ShijuAppsLoadState::Idle;
    app.shiju_apps.binding = ShijuAppsLoadState::Idle;
    let generation = app.shiju_apps.next_generation();
    let credential = account.skill_hub_credential();
    let client = app.client.clone();
    match app.shiju_apps.tab {
        ShijuAppsTab::Store => {
            app.shiju_apps.store = ShijuAppsLoadState::Loading { generation };
            let mine = if !cached(&app.shiju_apps.mine) {
                app.shiju_apps.mine = ShijuAppsLoadState::Loading { generation };
                load_mine(client.clone(), credential.clone(), account_id.clone(), generation)
            } else {
                Task::none()
            };
            let store = Task::perform(
                async move {
                    let credential =
                        credential.gateway_credential().map_err(|error| error.to_string())?;
                    gateway_shiju_app_catalog_async(client, &credential).await
                },
                move |result| ShijuAppsMessage::StoreLoaded { generation, account_id, result },
            );
            Task::batch([store, mine])
        }
        ShijuAppsTab::Mine | ShijuAppsTab::Favorites | ShijuAppsTab::Created => {
            let mut tasks = Vec::new();
            if app.shiju_apps.tab != ShijuAppsTab::Created || !cached(&app.shiju_apps.mine) {
                if app.shiju_apps.studio_lookup_generation.is_some() {
                    app.shiju_apps.studio_lookup_generation = Some(generation);
                }
                app.shiju_apps.mine = ShijuAppsLoadState::Loading { generation };
                tasks.push(load_mine(
                    client.clone(),
                    credential.clone(),
                    account_id.clone(),
                    generation,
                ));
            }
            if app.shiju_apps.tab == ShijuAppsTab::Mine && !cached(&app.shiju_apps.featured) {
                app.shiju_apps.featured = ShijuAppsLoadState::Loading { generation };
                app.shiju_apps.featured_generation = generation;
                let (featured_client, featured_credential, featured_account) =
                    (client.clone(), credential.clone(), account_id.clone());
                tasks.push(Task::perform(
                    read_marketplace(
                        featured_client,
                        featured_credential,
                        ShijuMarketplaceList::Featured,
                    ),
                    move |result| ShijuAppsMessage::FeaturedLoaded {
                        generation,
                        account_id: featured_account,
                        result,
                    },
                ));
                tasks.push(Task::perform(
                    read_marketplace(client, credential, ShijuMarketplaceList::Categories),
                    move |result| ShijuAppsMessage::CategoriesLoaded {
                        generation,
                        account_id,
                        result,
                    },
                ));
            } else if app.shiju_apps.tab == ShijuAppsTab::Created {
                app.shiju_apps.created = ShijuAppsLoadState::Loading { generation };
                tasks.push(Task::perform(
                    read_marketplace(client, credential, ShijuMarketplaceList::Created),
                    move |result| ShijuAppsMessage::CreatedLoaded {
                        generation,
                        account_id,
                        result,
                    },
                ));
            }
            Task::batch(tasks)
        }
    }
}

fn load_mine(
    client: Result<vw_gateway_client::GatewayClient, String>,
    credential: desktop_account::SecretSkey,
    account_id: String,
    generation: u64,
) -> Task<ShijuAppsMessage> {
    Task::perform(
        async move {
            let credential = credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_apps_async(client, &credential).await
        },
        move |result| ShijuAppsMessage::MineLoaded { generation, account_id, result },
    )
}

async fn read_marketplace<T: serde::de::DeserializeOwned>(
    client: Result<vw_gateway_client::GatewayClient, String>,
    credential: desktop_account::SecretSkey,
    list: ShijuMarketplaceList,
) -> Result<Vec<T>, String> {
    let credential = credential.gateway_credential().map_err(|error| error.to_string())?;
    let client = client?;
    let result = GatewayShijuAppClient::new(client.transport().clone())
        .marketplace_list(&credential, list)
        .await?;
    Ok(result.items)
}

fn marketplace_action(
    app: &mut Input<'_>,
    action: ShijuMarketplaceAction,
) -> Task<ShijuAppsMessage> {
    let Some(account) = app.account else { return Task::none() };
    let account_id = account.id().to_string();
    if matches!(action, ShijuMarketplaceAction::Download(_))
        || app.shiju_apps.action.is_some()
        || app.shiju_apps.account_id.as_deref() != Some(account_id.as_str())
    {
        return Task::none();
    }
    let generation = app.shiju_apps.next_generation();
    app.shiju_apps.action = Some((generation, action.clone()));
    let credential = account.skill_hub_credential();
    let client = app.client.clone();
    Task::perform(
        async move {
            let client = GatewayShijuAppClient::new(client?.transport().clone());
            let credential = credential.gateway_credential().map_err(|error| error.to_string())?;
            match action {
                ShijuMarketplaceAction::Favorite(id, value) => {
                    client.set_favorite(&credential, &id, value).await
                }
                ShijuMarketplaceAction::Install(id) => {
                    client.install_catalog(&credential, &id).await
                }
                ShijuMarketplaceAction::Copy(id, request) => {
                    client.copy_installation(&credential, &id, &request).await
                }
                ShijuMarketplaceAction::Download(_) => Ok(()),
            }
        },
        move |result| ShijuAppsMessage::MarketplaceActionFinished {
            generation,
            account_id,
            result,
        },
    )
}

fn finish_marketplace_action(
    app: &mut Input<'_>,
    generation: u64,
    account_id: &str,
    result: Result<(), String>,
) -> Task<ShijuAppsMessage> {
    if !app.account.is_some_and(|account| account.id() == account_id)
        || !matches!(app.shiju_apps.action, Some((current, _)) if current == generation)
    {
        return Task::none();
    }
    let Some((_, action)) = app.shiju_apps.action.take() else { return Task::none() };
    if let Err(error) = result {
        return app.show_localized_toast((), "应用操作失败：%s", vec![error.into()]);
    }
    match action {
        ShijuMarketplaceAction::Copy(_, _) => {
            app.shiju_apps.copy_form = None;
            app.shiju_apps.created = ShijuAppsLoadState::Idle;
            app.effects.push(Effect::Success("已复制应用，可在我创建的应用中查看".into()));
            if app.shiju_apps.tab == ShijuAppsTab::Created {
                load_current_tab(app)
            } else {
                Task::none()
            }
        }
        ShijuMarketplaceAction::Download(_) => Task::none(),
        ShijuMarketplaceAction::Favorite(id, value) => {
            if let ShijuAppsLoadState::Loaded(mine) = &mut app.shiju_apps.mine {
                for item in &mut mine.apps {
                    if item.installation_id == id {
                        item.presentation.is_favorite = value;
                    }
                }
            }
            if let ShijuAppsLoadState::Loaded(featured) = &mut app.shiju_apps.featured {
                for item in featured {
                    if item.app.installation_id == id {
                        item.app.presentation.is_favorite = value;
                    }
                }
            }
            Task::none()
        }
        ShijuMarketplaceAction::Install(_) => {
            app.shiju_apps.store = ShijuAppsLoadState::Idle;
            app.shiju_apps.mine = ShijuAppsLoadState::Idle;
            app.shiju_apps.featured = ShijuAppsLoadState::Idle;
            app.shiju_apps.featured_generation = generation;
            app.effects.push(Effect::Success("应用已安装".into()));
            load_current_tab(app)
        }
    }
}

fn load_detail_metadata(app: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    app.shiju_apps.detail_metadata = ShijuAppsLoadState::Idle;
    let installation = app.shiju_apps.detail_installation().map(str::to_owned);
    let development = match (app.shiju_apps.detail, &app.shiju_apps.created) {
        (Some(ShijuAppDetailTarget::Created(index)), ShijuAppsLoadState::Loaded(items)) => {
            items.get(index).map(|item| item.app_zid.clone())
        }
        _ => None,
    };
    if development.is_none() && installation.is_none() {
        return Task::none();
    }
    let Some(account) = app.account else { return Task::none() };
    if app.shiju_apps.account_id.as_deref() != Some(account.id()) {
        return Task::none();
    }
    let account_id = account.id().to_owned();
    let generation = app.shiju_apps.next_generation();
    app.shiju_apps.detail_metadata = ShijuAppsLoadState::Loading { generation };
    let client = app.client.clone();
    let credential = account.skill_hub_credential();
    Task::perform(
        async move {
            let client = GatewayShijuAppClient::new(client?.transport().clone());
            let credential = credential.gateway_credential().map_err(|error| error.to_string())?;
            if let Some(zid) = development {
                return client
                    .get_development_detail(&credential, &zid)
                    .await
                    .map(ShijuDetailMetadata::Development);
            }
            let id = installation.ok_or_else(|| "应用详情不存在".to_string())?;
            client.get_installation(&credential, &id).await.map(ShijuDetailMetadata::Installation)
        },
        move |result| ShijuAppsMessage::DetailMetadataLoaded { generation, account_id, result },
    )
}

fn load_detail_binding(app: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    app.shiju_apps.binding = ShijuAppsLoadState::Idle;
    let Some(installation_id) = app.shiju_apps.detail_installation().map(str::to_owned) else {
        return Task::none();
    };
    let Some(account) = app.account else { return Task::none() };
    let account_id = account.id().to_string();
    if app.shiju_apps.account_id.as_deref() != Some(account_id.as_str()) {
        return Task::none();
    }
    let generation = app.shiju_apps.next_generation();
    app.shiju_apps.binding = ShijuAppsLoadState::Loading { generation };
    let client = app.client.clone();
    let credential = account.skill_hub_credential();
    Task::perform(
        async move {
            let credential = credential.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_app_api_key_binding_async(client, &credential, &installation_id).await
        },
        move |result| ShijuAppsMessage::DetailBindingLoaded { generation, account_id, result },
    )
}

fn change_binding(app: &mut Input<'_>) -> Task<ShijuAppsMessage> {
    let Some(id) = app.shiju_apps.detail_installation().map(str::to_owned) else {
        return Task::none();
    };
    if !matches!(&app.shiju_apps.binding, ShijuAppsLoadState::Loaded(binding) if binding.personal.can_edit)
        || !matches!(
            app.shiju_apps.launch,
            ShijuAppLaunchState::Idle | ShijuAppLaunchState::Error { .. }
        )
    {
        return Task::none();
    }
    app.shiju_apps.changing_binding = true;
    begin_binding_check(app, id)
}

pub struct Input<'a> {
    pub i18n: desktop_i18n::I18n,
    pub shiju_apps: &'a mut ShijuAppsState,
    pub account: Option<&'a desktop_account::StoredAccount>,
    pub client: Result<vw_gateway_client::GatewayClient, String>,
    pub effects: Vec<Effect>,
}
pub enum Effect {
    Open,
    Toast(String, Vec<desktop_i18n::format::Argument>),
    Success(String),
    Launch { target: ShijuAppLaunchDto, generation: u64, installation_id: String },
}
impl Input<'_> {
    fn show_localized_toast(
        &mut self,
        _: (),
        message: &str,
        args: Vec<desktop_i18n::format::Argument>,
    ) -> Task<ShijuAppsMessage> {
        self.effects.push(Effect::Toast(message.to_string(), args));
        Task::none()
    }
}
#[path = "shiju_apps_requests.rs"]
mod requests;

#[path = "shiju_apps_artwork.rs"]
mod artwork;

#[path = "shiju_apps_operations.rs"]
pub mod operations;
