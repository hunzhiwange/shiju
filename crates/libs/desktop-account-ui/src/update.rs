use crate::message::*;
use crate::state::*;
use desktop_account::*;
use desktop_i18n::format::Argument;
use desktop_ui_controls::toast::ToastKind;
use gateway_shiju_account::ShijuAccountPasswordUpdateRequest;
use iced::Task;
use std::time::Duration;
#[derive(Clone)]
pub struct Services {
    pub gateway: Result<vw_gateway_client::GatewayClient, String>,
    pub store_path: Result<std::path::PathBuf, String>,
}
impl Services {
    fn load(&self) -> Result<AccountRepository, String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            AccountRepository::load_from(self.store_path.as_ref().map_err(Clone::clone)?)
                .map_err(|e| e.to_string())
        }
        #[cfg(target_arch = "wasm32")]
        {
            AccountRepository::load().map_err(|e| e.to_string())
        }
    }
    fn save(&self, repository: &AccountRepository) -> Result<(), String> {
        #[cfg(not(target_arch = "wasm32"))]
        {
            repository
                .save_to(self.store_path.as_ref().map_err(Clone::clone)?)
                .map_err(|e| e.to_string())
        }
        #[cfg(target_arch = "wasm32")]
        {
            repository.save().map_err(|e| e.to_string())
        }
    }
}
pub enum Effect {
    Toast(ToastKind, &'static str, Vec<Argument>),
    Error(String),
    #[cfg(not(target_arch = "wasm32"))]
    Host(AccountMessage),
    Validation(desktop_i18n::workflow::Feedback),
    #[cfg(not(target_arch = "wasm32"))]
    RefreshDomains,
    #[cfg(not(target_arch = "wasm32"))]
    SyncModels,
    #[cfg(not(target_arch = "wasm32"))]
    PrepareSkill,
}
fn toast(
    effects: &mut Vec<Effect>,
    kind: ToastKind,
    text: &'static str,
    args: Vec<Argument>,
) -> Task<AccountMessage> {
    effects.push(Effect::Toast(kind, text, args));
    Task::none()
}
fn error(effects: &mut Vec<Effect>, message: String) -> Task<AccountMessage> {
    effects.push(Effect::Error(message));
    Task::none()
}
fn model_validation_feedback(
    effects: &mut Vec<Effect>,
    feedback: desktop_i18n::workflow::Feedback,
) -> Task<AccountMessage> {
    effects.push(Effect::Validation(feedback));
    Task::none()
}
fn open_authorization_url(url: &str) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        open::that(url).is_ok()
    }
    #[cfg(target_arch = "wasm32")]
    {
        desktop_account::account::browser::navigate(url)
    }
}

const LOGIN_TIMEOUT: Duration = Duration::from_secs(30 * 60);

/// 在后台加载账号仓储，并将结果送回账号消息循环。
pub fn load_repository(services: Services) -> Task<AccountMessage> {
    Task::perform(async move { services.load() }, |result| AccountMessage::RepositoryLoaded(result))
}

fn validate_task(
    services: Services,
    skey: SecretSkey,
    generation: u64,
    origin: AccountOrigin,
) -> Task<AccountMessage> {
    Task::perform(
        async move {
            let credential = skey.gateway_credential().map_err(|error| error.to_string())?;
            let profile = gateway_shiju_account::GatewayShijuAccountClient::new(
                services.gateway.as_ref().map_err(Clone::clone)?.transport().clone(),
            )
            .shiju_account_profile(&credential)
            .await?;
            VerifiedAccount::from_gateway_profile(profile, skey).map_err(|error| error.to_string())
        },
        move |result| AccountMessage::AccountValidated { generation, origin, result },
    )
}

fn persist_task(
    services: Services,
    repository: AccountRepository,
    generation: u64,
    notice: AccountNotice,
) -> Task<AccountMessage> {
    Task::perform(
        async move {
            services.save(&repository)?;
            Ok(repository)
        },
        move |result| AccountMessage::RepositoryPersisted { generation, notice, result },
    )
}

fn poll_task(services: Services, generation: u64, state: String) -> Task<AccountMessage> {
    Task::perform(
        async move {
            gateway_shiju_account::GatewayShijuAccountClient::new(
                services.gateway.as_ref().map_err(Clone::clone)?.transport().clone(),
            )
            .shiju_account_login_poll(&state)
            .await
            .map(LoginPoll::from)
        },
        move |result| AccountMessage::LoginPolled { generation, result },
    )
}

fn start_browser_login_task(services: Services, generation: u64) -> Task<AccountMessage> {
    Task::perform(
        async move {
            gateway_shiju_account::GatewayShijuAccountClient::new(
                services.gateway.as_ref().map_err(Clone::clone)?.transport().clone(),
            )
            .shiju_account_login()
            .await
            .and_then(|dto| LoginChallenge::try_from(dto).map_err(|error| error.to_string()))
        },
        move |result| AccountMessage::LoginStarted { generation, result },
    )
}

fn refresh_monthly_quota(account: &mut AccountState, services: &Services) -> Task<AccountMessage> {
    if matches!(account.monthly_quota, MonthlyQuotaState::Loading(_)) {
        return Task::none();
    }
    let Some(active) = account.active_account() else {
        return Task::none();
    };
    let skey = active.skill_hub_credential();
    let generation = account.monthly_quota_generation;
    let previous = match account.monthly_quota {
        MonthlyQuotaState::Ready(quota) => Some(quota),
        _ => None,
    };
    account.monthly_quota = MonthlyQuotaState::Loading(previous);
    let services = services.clone();
    Task::perform(
        async move {
            let credential = skey.gateway_credential().ok()?;
            gateway_shiju_account::GatewayShijuAccountClient::new(
                services.gateway.as_ref().ok()?.transport().clone(),
            )
            .shiju_account_monthly_quota(&credential)
            .await
            .ok()
        },
        move |quota| AccountMessage::MonthlyQuotaLoaded { generation, quota },
    )
}

fn profile_load_task(
    services: Services,
    skey: SecretSkey,
    generation: u64,
) -> Task<AccountMessage> {
    Task::perform(
        async move {
            let credential = skey.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_account::GatewayShijuAccountClient::new(
                services.gateway.as_ref().map_err(Clone::clone)?.transport().clone(),
            )
            .shiju_account_me(&credential)
            .await
        },
        move |result| AccountMessage::ProfileLoaded { generation, result },
    )
}

fn profile_update_task(
    services: Services,
    skey: SecretSkey,
    request: gateway_shiju_account::ShijuAccountUpdateRequest,
    generation: u64,
) -> Task<AccountMessage> {
    Task::perform(
        async move {
            let credential = skey.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_account::GatewayShijuAccountClient::new(
                services.gateway.as_ref().map_err(Clone::clone)?.transport().clone(),
            )
            .shiju_account_update(&credential, &request)
            .await?;
            gateway_shiju_account::GatewayShijuAccountClient::new(
                services.gateway.as_ref().map_err(Clone::clone)?.transport().clone(),
            )
            .shiju_account_me(&credential)
            .await
        },
        move |result| AccountMessage::ProfileUpdated { generation, result },
    )
}

fn profile_persist_task(
    services: Services,
    repository: AccountRepository,
    generation: u64,
) -> Task<AccountMessage> {
    Task::perform(
        async move {
            services.save(&repository)?;
            Ok(repository)
        },
        move |result| AccountMessage::ProfilePersisted { generation, result },
    )
}

fn password_update_task(
    services: Services,
    skey: SecretSkey,
    request: ShijuAccountPasswordUpdateRequest,
    generation: u64,
) -> Task<AccountMessage> {
    Task::perform(
        async move {
            let credential = skey.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_account::GatewayShijuAccountClient::new(
                services.gateway.as_ref().map_err(Clone::clone)?.transport().clone(),
            )
            .shiju_account_password_update(&credential, &request)
            .await
        },
        move |result| AccountMessage::PasswordUpdated { generation, result },
    )
}

/// 应用账号状态转换，并为需要的网关或持久化工作返回异步任务。
pub fn update(
    account: &mut AccountState,
    message: AccountMessage,
    services: &Services,
    effects: &mut Vec<Effect>,
) -> Task<AccountMessage> {
    match message {
        AccountMessage::RepositoryLoaded(result) => {
            match result {
                Ok(repository) => account.replace_repository(repository),
                Err(error) => {
                    return toast(
                        effects,
                        ToastKind::Error,
                        "账户配置加载失败：%s",
                        vec![error.to_string().into()],
                    );
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            let reload = {
                effects.push(Effect::RefreshDomains);
                Task::none()
            };
            #[cfg(not(target_arch = "wasm32"))]
            let models = {
                effects.push(Effect::SyncModels);
                Task::none()
            };
            #[cfg(not(target_arch = "wasm32"))]
            let native_skill = {
                effects.push(Effect::PrepareSkill);
                Task::none()
            };
            let quota = refresh_monthly_quota(account, services);
            #[cfg(not(target_arch = "wasm32"))]
            {
                Task::batch([reload, models, native_skill, quota])
            }
            #[cfg(target_arch = "wasm32")]
            {
                quota
            }
        }
        AccountMessage::ToggleMenu => {
            account.menu_open = !account.menu_open;
            Task::none()
        }
        AccountMessage::RefreshMonthlyQuota => refresh_monthly_quota(account, services),
        AccountMessage::MonthlyQuotaLoaded { generation, quota } => {
            if account.monthly_quota_generation == generation
                && matches!(account.monthly_quota, MonthlyQuotaState::Loading(_))
            {
                account.monthly_quota = match quota {
                    Some(quota) => MonthlyQuotaState::Ready(quota),
                    None => MonthlyQuotaState::Unavailable,
                };
            }
            Task::none()
        }
        AccountMessage::CloseMenu => {
            account.menu_open = false;
            Task::none()
        }
        AccountMessage::OpenDetails => {
            if account.active_account().is_some() {
                account.menu_open = false;
                account.details_open = true;
            }
            Task::none()
        }
        AccountMessage::CloseDetails => {
            account.details_open = false;
            account.cancel_profile_editor();
            account.cancel_password_editor();
            Task::none()
        }
        AccountMessage::OpenProfileEditor => {
            let Some(skey) = account.active_account().map(|account| account.skill_hub_credential())
            else {
                return toast(effects, ToastKind::Warning, "当前没有可更新的账户", vec![]);
            };
            account.cancel_password_editor();
            let generation = account.begin_profile_editor(AccountProfileEditorState::Loading);
            profile_load_task(services.clone(), skey, generation)
        }
        AccountMessage::CloseProfileEditor => {
            account.cancel_profile_editor();
            Task::none()
        }
        AccountMessage::ProfileLoaded { generation, result } => {
            if !account.profile_is_current(generation) {
                return Task::none();
            }
            match result {
                Ok(profile) => match AccountProfileDraft::from_me(&profile) {
                    Ok(draft) => {
                        account.profile_editor = AccountProfileEditorState::Editing(draft);
                        Task::none()
                    }
                    Err(error) => {
                        let message = error.to_string();
                        account.profile_editor = AccountProfileEditorState::Failed(message.clone());
                        toast(
                            effects,
                            ToastKind::Error,
                            "账户资料加载失败：%s",
                            vec![message.to_string().into()],
                        )
                    }
                },
                Err(error) => {
                    account.profile_editor = AccountProfileEditorState::Failed(error.clone());
                    toast(
                        effects,
                        ToastKind::Error,
                        "账户资料加载失败：%s",
                        vec![error.to_string().into()],
                    )
                }
            }
        }
        AccountMessage::ProfileNameChanged(value) => {
            if let Some(draft) = account.profile_editor.draft_mut() {
                draft.set_name(value);
            }
            Task::none()
        }
        AccountMessage::ProfileUsernameChanged(value) => {
            if let Some(draft) = account.profile_editor.draft_mut() {
                draft.set_username(value);
            }
            Task::none()
        }
        AccountMessage::SubmitProfileEditor => {
            let Some(draft) = account.profile_editor.draft().cloned() else {
                return Task::none();
            };
            if account.profile_editor.is_busy() {
                return Task::none();
            }
            let request = match draft.update_request() {
                Ok(request) => request,
                Err(error) => return model_validation_feedback(effects, error.feedback()),
            };
            let Some(skey) = account.active_account().map(|account| account.skill_hub_credential())
            else {
                return toast(effects, ToastKind::Warning, "当前没有可更新的账户", vec![]);
            };
            let generation = account.begin_profile_editor(AccountProfileEditorState::Saving(draft));
            profile_update_task(services.clone(), skey, request, generation)
        }
        AccountMessage::ProfileUpdated { generation, result } => {
            if !account.profile_is_current(generation) {
                return Task::none();
            }
            let draft = account.profile_editor.draft().cloned().unwrap_or_default();
            match result {
                Ok(profile) => {
                    let mut repository = account.repository().clone();
                    if let Err(error) = repository.refresh_active_profile(profile) {
                        account.profile_editor = AccountProfileEditorState::Editing(draft);
                        return toast(
                            effects,
                            ToastKind::Error,
                            "账户资料更新失败：%s",
                            vec![error.to_string().into()],
                        );
                    }
                    profile_persist_task(services.clone(), repository, generation)
                }
                Err(error) => {
                    account.profile_editor = AccountProfileEditorState::Editing(draft);
                    toast(
                        effects,
                        ToastKind::Error,
                        "账户资料更新失败：%s",
                        vec![error.to_string().into()],
                    )
                }
            }
        }
        AccountMessage::ProfilePersisted { generation, result } => {
            if !account.profile_is_current(generation) {
                return Task::none();
            }
            let fallback = account.profile_editor.draft().cloned().unwrap_or_default();
            match result {
                Ok(repository) => {
                    let draft = repository
                        .active_account()
                        .and_then(|account| account.profile())
                        .map(AccountProfileDraft::from_profile)
                        .unwrap_or(fallback);
                    account.replace_repository(repository);
                    account.profile_editor = AccountProfileEditorState::Editing(draft);
                    toast(effects, ToastKind::Success, "账户资料已更新", vec![])
                }
                Err(error) => {
                    account.profile_editor = AccountProfileEditorState::Editing(fallback);
                    toast(
                        effects,
                        ToastKind::Error,
                        "账户资料保存失败：%s",
                        vec![error.to_string().into()],
                    )
                }
            }
        }
        AccountMessage::OpenPasswordEditor => {
            if account.active_account().is_none() {
                return toast(effects, ToastKind::Warning, "当前没有可更新的账户", vec![]);
            }
            account.cancel_profile_editor();
            account.begin_password_editor(AccountPasswordEditorState::Editing(
                AccountPasswordDraft::default(),
            ));
            Task::none()
        }
        AccountMessage::ClosePasswordEditor => {
            account.cancel_password_editor();
            Task::none()
        }
        AccountMessage::PasswordFieldChanged { field, value } => {
            let Some(mut draft) = account.password_editor.draft().cloned() else {
                return Task::none();
            };
            draft.set(field, value);
            account.password_editor = AccountPasswordEditorState::Editing(draft);
            Task::none()
        }
        AccountMessage::DismissPasswordError => {
            let Some(draft) = account.password_editor.draft().cloned() else {
                return Task::none();
            };
            account.password_editor = AccountPasswordEditorState::Editing(draft);
            Task::none()
        }
        AccountMessage::SubmitPasswordEditor => {
            if account.password_editor.is_busy() {
                return Task::none();
            }
            let Some(draft) = account.password_editor.draft().cloned() else {
                return Task::none();
            };
            let request = match draft.update_request() {
                Ok(request) => request,
                Err(error) => return model_validation_feedback(effects, error.feedback()),
            };
            let Some(skey) = account.active_account().map(|account| account.skill_hub_credential())
            else {
                return toast(effects, ToastKind::Warning, "当前没有可更新的账户", vec![]);
            };
            let generation =
                account.begin_password_editor(AccountPasswordEditorState::Saving(draft));
            password_update_task(services.clone(), skey, request, generation)
        }
        AccountMessage::PasswordUpdated { generation, result } => {
            if !account.password_is_current(generation) {
                return Task::none();
            }
            let draft = account.password_editor.draft().cloned().unwrap_or_default();
            match result {
                Ok(()) => {
                    account.cancel_password_editor();
                    toast(effects, ToastKind::Success, "密码已修改", vec![])
                }
                Err(error) => {
                    account.password_editor =
                        AccountPasswordEditorState::Failed { draft, message: error };
                    Task::none()
                }
            }
        }
        AccountMessage::ManualInputChanged(input) => {
            account.manual_input = input;
            Task::none()
        }
        AccountMessage::SubmitManual => {
            if account.operation.is_busy() {
                return Task::none();
            }
            let skey = match account.manual_input.clone().into_secret() {
                Ok(skey) => skey,
                Err(error) => return model_validation_feedback(effects, error.feedback()),
            };
            let generation = account.begin(AccountOperation::Validating);
            validate_task(services.clone(), skey, generation, AccountOrigin::ManualSkey)
        }
        AccountMessage::StartBrowserLogin => {
            if account.operation.is_busy() {
                return Task::none();
            }
            #[cfg(target_arch = "wasm32")]
            if !desktop_account::account::browser::prepare() {
                return toast(effects, ToastKind::Error, "无法打开登录授权页面", vec![]);
            }
            let generation = account.begin(AccountOperation::StartingBrowser);
            start_browser_login_task(services.clone(), generation)
        }
        AccountMessage::CancelBrowserLogin => {
            if !matches!(&account.operation, AccountOperation::WaitingAuthorization { .. }) {
                return Task::none();
            }
            #[cfg(target_arch = "wasm32")]
            desktop_account::account::browser::close();
            account.cancel();
            toast(effects, ToastKind::Info, "已取消等待授权", vec![])
        }
        AccountMessage::LoginStarted { generation, result } => {
            if !account.is_current(generation) {
                return Task::none();
            }
            match result {
                Ok(challenge) => {
                    if !open_authorization_url(challenge.authorization_url().as_str()) {
                        account.finish();
                        return toast(effects, ToastKind::Error, "无法打开登录授权页面", vec![]);
                    }
                    let interval = challenge.poll_interval();
                    account.operation = AccountOperation::WaitingAuthorization {
                        challenge,
                        started_at: web_time::Instant::now(),
                    };
                    Task::batch([
                        after(interval, AccountMessage::PollLogin { generation }),
                        after(LOGIN_TIMEOUT, AccountMessage::LoginTimedOut { generation }),
                    ])
                }
                Err(error) => {
                    #[cfg(target_arch = "wasm32")]
                    desktop_account::account::browser::close();
                    account.finish();
                    toast(effects, ToastKind::Error, "登录失败：%s", vec![error.to_string().into()])
                }
            }
        }
        AccountMessage::PollLogin { generation } => {
            if !account.is_current(generation) {
                return Task::none();
            }
            let AccountOperation::WaitingAuthorization { challenge, started_at } =
                &account.operation
            else {
                return Task::none();
            };
            if started_at.elapsed() >= LOGIN_TIMEOUT {
                #[cfg(target_arch = "wasm32")]
                desktop_account::account::browser::close();
                account.cancel();
                return toast(effects, ToastKind::Warning, "登录授权已超时，已取消等待", vec![]);
            }
            poll_task(services.clone(), generation, challenge.state().to_string())
        }
        AccountMessage::LoginTimedOut { generation } => {
            if !account.is_current(generation)
                || !matches!(&account.operation, AccountOperation::WaitingAuthorization { .. })
            {
                return Task::none();
            }
            #[cfg(target_arch = "wasm32")]
            desktop_account::account::browser::close();
            account.cancel();
            toast(effects, ToastKind::Warning, "登录授权已超时，已取消等待", vec![])
        }
        AccountMessage::LoginPolled { generation, result } => {
            if !account.is_current(generation) {
                return Task::none();
            }
            match result {
                Ok(LoginPoll::Pending) => {
                    let AccountOperation::WaitingAuthorization { challenge, .. } =
                        &account.operation
                    else {
                        return Task::none();
                    };
                    after(challenge.poll_interval(), AccountMessage::PollLogin { generation })
                }
                Ok(LoginPoll::Approved(skey)) => {
                    #[cfg(target_arch = "wasm32")]
                    desktop_account::account::browser::close();
                    account.operation = AccountOperation::Validating;
                    let validation = validate_task(
                        services.clone(),
                        skey,
                        generation,
                        AccountOrigin::BrowserLogin,
                    );
                    let feedback =
                        toast(effects, ToastKind::Info, "授权成功，正在加载账户信息", vec![]);
                    Task::batch([validation, feedback])
                }
                Err(error) => {
                    #[cfg(target_arch = "wasm32")]
                    desktop_account::account::browser::close();
                    account.finish();
                    toast(effects, ToastKind::Error, "登录失败：%s", vec![error.to_string().into()])
                }
            }
        }
        AccountMessage::AccountValidated { generation, origin, result } => {
            if !account.is_current(generation) {
                return Task::none();
            }
            match result {
                Ok(verified) => {
                    let mut repository = account.repository().clone();
                    repository.upsert_verified(verified, origin);
                    account.operation = AccountOperation::Saving;
                    let notice = match origin {
                        AccountOrigin::ManualSkey => AccountNotice::ManualLogin,
                        AccountOrigin::BrowserLogin => AccountNotice::BrowserLogin,
                    };
                    persist_task(services.clone(), repository, generation, notice)
                }
                Err(error) => {
                    account.finish();
                    toast(
                        effects,
                        ToastKind::Error,
                        "账户校验失败：%s",
                        vec![error.to_string().into()],
                    )
                }
            }
        }
        AccountMessage::SelectSaved(id) => {
            if account.operation.is_busy() {
                return Task::none();
            }
            let mut repository = account.repository().clone();
            if let Err(cause) = repository.activate(&id) {
                return error(effects, cause.to_string());
            }
            account.cancel_profile_editor();
            account.cancel_password_editor();
            let generation = account.begin(AccountOperation::Saving);
            persist_task(services.clone(), repository, generation, AccountNotice::Switch)
        }
        AccountMessage::DeleteSaved(id) => {
            if account.operation.is_busy() {
                return Task::none();
            }
            let mut repository = account.repository().clone();
            if let Err(cause) = repository.remove(&id) {
                return error(effects, cause.to_string());
            }
            account.cancel_profile_editor();
            account.cancel_password_editor();
            let generation = account.begin(AccountOperation::Saving);
            persist_task(services.clone(), repository, generation, AccountNotice::Delete)
        }
        AccountMessage::SignOut => {
            if account.operation.is_busy() {
                return Task::none();
            }
            let mut repository = account.repository().clone();
            repository.sign_out();
            account.cancel_profile_editor();
            account.cancel_password_editor();
            let generation = account.begin(AccountOperation::Saving);
            persist_task(services.clone(), repository, generation, AccountNotice::SignOut)
        }
        AccountMessage::RepositoryPersisted { generation, notice, result } => {
            if !account.is_current(generation) {
                return Task::none();
            }
            account.finish();
            match result {
                Ok(repository) => {
                    account.replace_repository(repository);
                    account.manual_input = ManualSkeyInput::default();
                    account.menu_open = false;
                    if account.active_account().is_none() {
                        account.details_open = false;
                    }
                    let feedback = match notice {
                        AccountNotice::ManualLogin => {
                            toast(effects, ToastKind::Success, "账户已登录", vec![])
                        }
                        AccountNotice::BrowserLogin => toast(
                            effects,
                            ToastKind::Success,
                            "登录成功，请手动关闭授权页面",
                            vec![],
                        ),
                        AccountNotice::Switch => {
                            toast(effects, ToastKind::Success, "账户已切换", vec![])
                        }
                        AccountNotice::SignOut => {
                            toast(effects, ToastKind::Success, "已退出登录", vec![])
                        }
                        AccountNotice::Delete => {
                            toast(effects, ToastKind::Success, "账户已删除", vec![])
                        }
                    };
                    #[cfg(not(target_arch = "wasm32"))]
                    let reload = {
                        effects.push(Effect::RefreshDomains);
                        Task::none()
                    };
                    #[cfg(not(target_arch = "wasm32"))]
                    let sync = {
                        if matches!(
                            notice,
                            AccountNotice::ManualLogin
                                | AccountNotice::BrowserLogin
                                | AccountNotice::Switch
                        ) && account.active_account().is_some()
                        {
                            effects.push(Effect::PrepareSkill);
                        }
                        Task::none()
                    };
                    #[cfg(not(target_arch = "wasm32"))]
                    let models = {
                        effects.push(Effect::SyncModels);
                        Task::none()
                    };
                    let quota = refresh_monthly_quota(account, services);
                    #[cfg(not(target_arch = "wasm32"))]
                    let quota = Task::batch([reload, sync, models, quota]);
                    Task::batch([feedback, quota])
                }
                Err(error) => toast(
                    effects,
                    ToastKind::Error,
                    "账户配置保存失败：%s",
                    vec![error.to_string().into()],
                ),
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        AccountMessage::ResetModels { .. }
        | AccountMessage::ModelTokenLoaded { .. }
        | AccountMessage::ModelsConfigured { .. }
        | AccountMessage::PrepareNativeShijuSkill
        | AccountMessage::NativeShijuSkillPrepared { .. } => {
            effects.push(Effect::Host(message));
            Task::none()
        }
    }
}

fn after(duration: Duration, message: AccountMessage) -> Task<AccountMessage> {
    Task::perform(sleep(duration), move |_| message)
}
async fn sleep(duration: std::time::Duration) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::{JsCast, JsValue, closure::Closure};

        // 将持续时间转换为毫秒，并限制在 i32 范围内
        let millis = duration.as_millis().min(i32::MAX as u128) as i32;

        // 获取浏览器窗口对象，如果失败则直接返回
        let Some(window) = web_sys::window() else {
            return;
        };

        // 创建一个 Promise，在其内部设置 setTimeout
        let promise = js_sys::Promise::new(&mut |resolve, _reject| {
            // 将 resolve 参数转换为 JavaScript 函数
            let Ok(resolve) = resolve.dyn_into::<js_sys::Function>() else {
                return;
            };

            // 创建一次性回调，在定时器触发时 resolve Promise
            let cb = Closure::once(move || {
                let _ = resolve.call0(&JsValue::NULL);
            });

            // 设置定时器，使用回调作为定时器完成时的处理函数
            let _ = window.set_timeout_with_callback_and_timeout_and_arguments_0(
                cb.as_ref().unchecked_ref(),
                millis,
            );

            // 使用 forget() 让回调在 JavaScript 侧保持存活直到被执行
            // 这是必要的，因为 Rust 无法知道 JavaScript 何时会调用这个回调
            cb.forget();
        });

        // 等待 Promise 完成（即定时器触发）
        let _ = wasm_bindgen_futures::JsFuture::from(promise).await;
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        tokio::time::sleep(duration).await;
    }
}

#[cfg(test)]
#[path = "update_tests.rs"]
mod tests;
