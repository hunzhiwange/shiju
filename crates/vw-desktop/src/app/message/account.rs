//! 处理账号登录、切换、资料更新及凭据同步的消息状态转换。

use crate::app::account::{
    AccountOrigin, AccountPasswordDraft, AccountPasswordField, AccountPasswordInput,
    AccountProfileDraft, AccountRepository, LoginChallenge, LoginPoll, ManualSkeyInput, SecretSkey,
    VerifiedAccount,
};
use crate::app::session_gateway::{
    gateway_shiju_account_login_async, gateway_shiju_account_login_poll_async,
    gateway_shiju_account_me_async, gateway_shiju_account_password_update_async,
    gateway_shiju_account_profile_async, gateway_shiju_account_update_async,
};
use crate::app::skill_hub::{ShijuKeySyncOutcome, sync_shiju_key};
use crate::app::state::{AccountOperation, AccountPasswordEditorState, AccountProfileEditorState};
use crate::app::{App, Message};
use iced::Task;
use std::time::Duration;
use vw_gateway_client::{ShijuAccountMeDto, ShijuAccountPasswordUpdateRequest};

const LOGIN_TIMEOUT: Duration = Duration::from_secs(30 * 60);

#[derive(Debug, Clone, Copy)]
/// 账号操作成功后用于生成反馈文案的动作类别。
pub(crate) enum AccountNotice {
    ManualLogin,
    BrowserLogin,
    Switch,
    SignOut,
    Delete,
}

#[derive(Debug, Clone)]
/// 账号登录、资料编辑和密码修改流程中的输入及异步结果消息。
pub(crate) enum AccountMessage {
    ResetModels {
        generation: u64,
    },
    ModelTokenLoaded {
        generation: u64,
        account_id: String,
        result: Result<vw_gateway_client::ShijuModelTokenDto, String>,
    },
    ModelsConfigured {
        generation: u64,
        result: Result<(), String>,
    },
    RepositoryLoaded(Result<AccountRepository, String>),
    ToggleMenu,
    CloseMenu,
    OpenDetails,
    CloseDetails,
    OpenProfileEditor,
    CloseProfileEditor,
    ProfileLoaded {
        generation: u64,
        result: Result<ShijuAccountMeDto, String>,
    },
    ProfileNameChanged(String),
    ProfileUsernameChanged(String),
    SubmitProfileEditor,
    ProfileUpdated {
        generation: u64,
        result: Result<ShijuAccountMeDto, String>,
    },
    ProfilePersisted {
        generation: u64,
        result: Result<AccountRepository, String>,
    },
    OpenPasswordEditor,
    ClosePasswordEditor,
    PasswordFieldChanged {
        field: AccountPasswordField,
        value: AccountPasswordInput,
    },
    DismissPasswordError,
    SubmitPasswordEditor,
    PasswordUpdated {
        generation: u64,
        result: Result<(), String>,
    },
    ManualInputChanged(ManualSkeyInput),
    SubmitManual,
    StartBrowserLogin,
    CancelBrowserLogin,
    LoginStarted {
        generation: u64,
        result: Result<LoginChallenge, String>,
    },
    PollLogin {
        generation: u64,
    },
    LoginTimedOut {
        generation: u64,
    },
    LoginPolled {
        generation: u64,
        result: Result<LoginPoll, String>,
    },
    AccountValidated {
        generation: u64,
        origin: AccountOrigin,
        result: Result<VerifiedAccount, String>,
    },
    SelectSaved(String),
    DeleteSaved(String),
    SignOut,
    RepositoryPersisted {
        generation: u64,
        notice: AccountNotice,
        result: Result<AccountRepository, String>,
    },
    ShijuKeySyncFinished(Result<ShijuKeySyncOutcome, String>),
}

/// 在后台加载账号仓储，并将结果送回账号消息循环。
pub(crate) fn load_repository() -> Task<Message> {
    Task::perform(
        async { AccountRepository::load().map_err(|error| error.to_string()) },
        |result| Message::Account(AccountMessage::RepositoryLoaded(result)),
    )
}

fn validate_task(skey: SecretSkey, generation: u64, origin: AccountOrigin) -> Task<Message> {
    Task::perform(
        async move {
            let credential = skey.gateway_credential().map_err(|error| error.to_string())?;
            let profile = gateway_shiju_account_profile_async(&credential).await?;
            VerifiedAccount::from_gateway_profile(profile, skey).map_err(|error| error.to_string())
        },
        move |result| {
            Message::Account(AccountMessage::AccountValidated { generation, origin, result })
        },
    )
}

fn persist_task(
    repository: AccountRepository,
    generation: u64,
    notice: AccountNotice,
) -> Task<Message> {
    Task::perform(
        async move {
            repository.save().map_err(|error| error.to_string())?;
            Ok(repository)
        },
        move |result| {
            Message::Account(AccountMessage::RepositoryPersisted { generation, notice, result })
        },
    )
}

fn poll_task(generation: u64, state: String) -> Task<Message> {
    Task::perform(
        async move { gateway_shiju_account_login_poll_async(&state).await.map(LoginPoll::from) },
        move |result| Message::Account(AccountMessage::LoginPolled { generation, result }),
    )
}

fn start_browser_login_task(generation: u64) -> Task<Message> {
    Task::perform(
        async {
            gateway_shiju_account_login_async()
                .await
                .and_then(|dto| LoginChallenge::try_from(dto).map_err(|error| error.to_string()))
        },
        move |result| Message::Account(AccountMessage::LoginStarted { generation, result }),
    )
}

fn profile_load_task(skey: SecretSkey, generation: u64) -> Task<Message> {
    Task::perform(
        async move {
            let credential = skey.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_account_me_async(&credential).await
        },
        move |result| Message::Account(AccountMessage::ProfileLoaded { generation, result }),
    )
}

fn profile_update_task(
    skey: SecretSkey,
    request: vw_gateway_client::ShijuAccountUpdateRequest,
    generation: u64,
) -> Task<Message> {
    Task::perform(
        async move {
            let credential = skey.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_account_update_async(&credential, &request).await?;
            gateway_shiju_account_me_async(&credential).await
        },
        move |result| Message::Account(AccountMessage::ProfileUpdated { generation, result }),
    )
}

fn profile_persist_task(repository: AccountRepository, generation: u64) -> Task<Message> {
    Task::perform(
        async move {
            repository.save().map_err(|error| error.to_string())?;
            Ok(repository)
        },
        move |result| Message::Account(AccountMessage::ProfilePersisted { generation, result }),
    )
}

fn password_update_task(
    skey: SecretSkey,
    request: ShijuAccountPasswordUpdateRequest,
    generation: u64,
) -> Task<Message> {
    Task::perform(
        async move {
            let credential = skey.gateway_credential().map_err(|error| error.to_string())?;
            gateway_shiju_account_password_update_async(&credential, &request).await
        },
        move |result| Message::Account(AccountMessage::PasswordUpdated { generation, result }),
    )
}

fn shiju_key_sync_task(app: &App, notice: AccountNotice) -> Task<Message> {
    if !matches!(
        notice,
        AccountNotice::ManualLogin | AccountNotice::BrowserLogin | AccountNotice::Switch
    ) {
        return Task::none();
    }
    let Some(credential) =
        app.account.active_account().map(|account| account.skill_hub_credential())
    else {
        return Task::none();
    };
    Task::perform(
        async move { sync_shiju_key(credential).await.map_err(|error| error.to_string()) },
        |result| Message::Account(AccountMessage::ShijuKeySyncFinished(result)),
    )
}

/// 应用账号状态转换，并为需要的网关或持久化工作返回异步任务。
pub(crate) fn update(app: &mut App, message: AccountMessage) -> Task<Message> {
    match message {
        AccountMessage::ResetModels { generation } => {
            super::settings::providers::managed_shiju::reset(app, generation)
        }
        AccountMessage::ModelTokenLoaded { generation, account_id, result } => {
            super::settings::providers::managed_shiju::token_loaded(
                app, generation, account_id, result,
            )
        }
        AccountMessage::ModelsConfigured { generation, result } => {
            super::settings::providers::managed_shiju::configured(app, generation, result)
        }
        AccountMessage::RepositoryLoaded(result) => {
            match result {
                Ok(repository) => app.account.replace_repository(repository),
                Err(error) => return app.show_error_toast(format!("账户配置加载失败：{error}")),
            }
            let reload = crate::app::message::skill_hub::account_changed(app);
            let models = super::settings::providers::managed_shiju::sync(app);
            Task::batch([reload, models])
        }
        AccountMessage::ToggleMenu => {
            app.account.menu_open = !app.account.menu_open;
            Task::none()
        }
        AccountMessage::CloseMenu => {
            app.account.menu_open = false;
            Task::none()
        }
        AccountMessage::OpenDetails => {
            if app.account.active_account().is_some() {
                app.account.menu_open = false;
                app.account.details_open = true;
            }
            Task::none()
        }
        AccountMessage::CloseDetails => {
            app.account.details_open = false;
            app.account.cancel_profile_editor();
            app.account.cancel_password_editor();
            Task::none()
        }
        AccountMessage::OpenProfileEditor => {
            let Some(skey) =
                app.account.active_account().map(|account| account.skill_hub_credential())
            else {
                return app.show_warning_toast("当前没有可更新的账户");
            };
            app.account.cancel_password_editor();
            let generation = app.account.begin_profile_editor(AccountProfileEditorState::Loading);
            profile_load_task(skey, generation)
        }
        AccountMessage::CloseProfileEditor => {
            app.account.cancel_profile_editor();
            Task::none()
        }
        AccountMessage::ProfileLoaded { generation, result } => {
            if !app.account.profile_is_current(generation) {
                return Task::none();
            }
            match result {
                Ok(profile) => match AccountProfileDraft::from_me(&profile) {
                    Ok(draft) => {
                        app.account.profile_editor = AccountProfileEditorState::Editing(draft);
                        Task::none()
                    }
                    Err(error) => {
                        let message = error.to_string();
                        app.account.profile_editor =
                            AccountProfileEditorState::Failed(message.clone());
                        app.show_error_toast(format!("账户资料加载失败：{message}"))
                    }
                },
                Err(error) => {
                    app.account.profile_editor = AccountProfileEditorState::Failed(error.clone());
                    app.show_error_toast(format!("账户资料加载失败：{error}"))
                }
            }
        }
        AccountMessage::ProfileNameChanged(value) => {
            if let Some(draft) = app.account.profile_editor.draft_mut() {
                draft.set_name(value);
            }
            Task::none()
        }
        AccountMessage::ProfileUsernameChanged(value) => {
            if let Some(draft) = app.account.profile_editor.draft_mut() {
                draft.set_username(value);
            }
            Task::none()
        }
        AccountMessage::SubmitProfileEditor => {
            let Some(draft) = app.account.profile_editor.draft().cloned() else {
                return Task::none();
            };
            if app.account.profile_editor.is_busy() {
                return Task::none();
            }
            let request = match draft.update_request() {
                Ok(request) => request,
                Err(error) => return app.show_warning_toast(error.to_string()),
            };
            let Some(skey) =
                app.account.active_account().map(|account| account.skill_hub_credential())
            else {
                return app.show_warning_toast("当前没有可更新的账户");
            };
            let generation =
                app.account.begin_profile_editor(AccountProfileEditorState::Saving(draft));
            profile_update_task(skey, request, generation)
        }
        AccountMessage::ProfileUpdated { generation, result } => {
            if !app.account.profile_is_current(generation) {
                return Task::none();
            }
            let draft = app.account.profile_editor.draft().cloned().unwrap_or_default();
            match result {
                Ok(profile) => {
                    let mut repository = app.account.repository().clone();
                    if let Err(error) = repository.refresh_active_profile(profile) {
                        app.account.profile_editor = AccountProfileEditorState::Editing(draft);
                        return app.show_error_toast(format!("账户资料更新失败：{error}"));
                    }
                    profile_persist_task(repository, generation)
                }
                Err(error) => {
                    app.account.profile_editor = AccountProfileEditorState::Editing(draft);
                    app.show_error_toast(format!("账户资料更新失败：{error}"))
                }
            }
        }
        AccountMessage::ProfilePersisted { generation, result } => {
            if !app.account.profile_is_current(generation) {
                return Task::none();
            }
            let fallback = app.account.profile_editor.draft().cloned().unwrap_or_default();
            match result {
                Ok(repository) => {
                    let draft = repository
                        .active_account()
                        .and_then(|account| account.profile())
                        .map(AccountProfileDraft::from_profile)
                        .unwrap_or(fallback);
                    app.account.replace_repository(repository);
                    app.account.profile_editor = AccountProfileEditorState::Editing(draft);
                    app.show_success_toast("账户资料已更新")
                }
                Err(error) => {
                    app.account.profile_editor = AccountProfileEditorState::Editing(fallback);
                    app.show_error_toast(format!("账户资料保存失败：{error}"))
                }
            }
        }
        AccountMessage::OpenPasswordEditor => {
            if app.account.active_account().is_none() {
                return app.show_warning_toast("当前没有可更新的账户");
            }
            app.account.cancel_profile_editor();
            app.account.begin_password_editor(AccountPasswordEditorState::Editing(
                AccountPasswordDraft::default(),
            ));
            Task::none()
        }
        AccountMessage::ClosePasswordEditor => {
            app.account.cancel_password_editor();
            Task::none()
        }
        AccountMessage::PasswordFieldChanged { field, value } => {
            let Some(mut draft) = app.account.password_editor.draft().cloned() else {
                return Task::none();
            };
            draft.set(field, value);
            app.account.password_editor = AccountPasswordEditorState::Editing(draft);
            Task::none()
        }
        AccountMessage::DismissPasswordError => {
            let Some(draft) = app.account.password_editor.draft().cloned() else {
                return Task::none();
            };
            app.account.password_editor = AccountPasswordEditorState::Editing(draft);
            Task::none()
        }
        AccountMessage::SubmitPasswordEditor => {
            if app.account.password_editor.is_busy() {
                return Task::none();
            }
            let Some(draft) = app.account.password_editor.draft().cloned() else {
                return Task::none();
            };
            let request = match draft.update_request() {
                Ok(request) => request,
                Err(error) => return app.show_warning_toast(error.to_string()),
            };
            let Some(skey) =
                app.account.active_account().map(|account| account.skill_hub_credential())
            else {
                return app.show_warning_toast("当前没有可更新的账户");
            };
            let generation =
                app.account.begin_password_editor(AccountPasswordEditorState::Saving(draft));
            password_update_task(skey, request, generation)
        }
        AccountMessage::PasswordUpdated { generation, result } => {
            if !app.account.password_is_current(generation) {
                return Task::none();
            }
            let draft = app.account.password_editor.draft().cloned().unwrap_or_default();
            match result {
                Ok(()) => {
                    app.account.cancel_password_editor();
                    app.show_success_toast("密码已修改")
                }
                Err(error) => {
                    app.account.password_editor =
                        AccountPasswordEditorState::Failed { draft, message: error };
                    Task::none()
                }
            }
        }
        AccountMessage::ManualInputChanged(input) => {
            app.account.manual_input = input;
            Task::none()
        }
        AccountMessage::SubmitManual => {
            if app.account.operation.is_busy() {
                return Task::none();
            }
            let skey = match app.account.manual_input.clone().into_secret() {
                Ok(skey) => skey,
                Err(error) => return app.show_warning_toast(error.to_string()),
            };
            let generation = app.account.begin(AccountOperation::Validating);
            validate_task(skey, generation, AccountOrigin::ManualSkey)
        }
        AccountMessage::StartBrowserLogin => {
            if app.account.operation.is_busy() {
                return Task::none();
            }
            let generation = app.account.begin(AccountOperation::StartingBrowser);
            start_browser_login_task(generation)
        }
        AccountMessage::CancelBrowserLogin => {
            if !matches!(&app.account.operation, AccountOperation::WaitingAuthorization { .. }) {
                return Task::none();
            }
            app.account.cancel();
            app.show_info_toast("已取消等待授权")
        }
        AccountMessage::LoginStarted { generation, result } => {
            if !app.account.is_current(generation) {
                return Task::none();
            }
            match result {
                Ok(challenge) => {
                    if open::that(challenge.authorization_url().as_str()).is_err() {
                        app.account.finish();
                        return app.show_error_toast("无法打开登录授权页面");
                    }
                    let interval = challenge.poll_interval();
                    app.account.operation = AccountOperation::WaitingAuthorization {
                        challenge,
                        started_at: std::time::Instant::now(),
                    };
                    Task::batch([
                        crate::app::message::after(
                            interval,
                            Message::Account(AccountMessage::PollLogin { generation }),
                        ),
                        crate::app::message::after(
                            LOGIN_TIMEOUT,
                            Message::Account(AccountMessage::LoginTimedOut { generation }),
                        ),
                    ])
                }
                Err(error) => {
                    app.account.finish();
                    app.show_error_toast(format!("登录失败：{error}"))
                }
            }
        }
        AccountMessage::PollLogin { generation } => {
            if !app.account.is_current(generation) {
                return Task::none();
            }
            let AccountOperation::WaitingAuthorization { challenge, started_at } =
                &app.account.operation
            else {
                return Task::none();
            };
            if started_at.elapsed() >= LOGIN_TIMEOUT {
                app.account.cancel();
                return app.show_warning_toast("登录授权已超时，已取消等待");
            }
            poll_task(generation, challenge.state().to_string())
        }
        AccountMessage::LoginTimedOut { generation } => {
            if !app.account.is_current(generation)
                || !matches!(&app.account.operation, AccountOperation::WaitingAuthorization { .. })
            {
                return Task::none();
            }
            app.account.cancel();
            app.show_warning_toast("登录授权已超时，已取消等待")
        }
        AccountMessage::LoginPolled { generation, result } => {
            if !app.account.is_current(generation) {
                return Task::none();
            }
            match result {
                Ok(LoginPoll::Pending) => {
                    let AccountOperation::WaitingAuthorization { challenge, .. } =
                        &app.account.operation
                    else {
                        return Task::none();
                    };
                    crate::app::message::after(
                        challenge.poll_interval(),
                        Message::Account(AccountMessage::PollLogin { generation }),
                    )
                }
                Ok(LoginPoll::Approved(skey)) => {
                    app.account.operation = AccountOperation::Validating;
                    let validation = validate_task(skey, generation, AccountOrigin::BrowserLogin);
                    let feedback = app.show_info_toast("授权成功，正在加载账户信息");
                    Task::batch([validation, feedback])
                }
                Err(error) => {
                    app.account.finish();
                    app.show_error_toast(format!("登录失败：{error}"))
                }
            }
        }
        AccountMessage::AccountValidated { generation, origin, result } => {
            if !app.account.is_current(generation) {
                return Task::none();
            }
            match result {
                Ok(verified) => {
                    let mut repository = app.account.repository().clone();
                    repository.upsert_verified(verified, origin);
                    app.account.operation = AccountOperation::Saving;
                    let notice = match origin {
                        AccountOrigin::ManualSkey => AccountNotice::ManualLogin,
                        AccountOrigin::BrowserLogin => AccountNotice::BrowserLogin,
                    };
                    persist_task(repository, generation, notice)
                }
                Err(error) => {
                    app.account.finish();
                    app.show_error_toast(format!("账户校验失败：{error}"))
                }
            }
        }
        AccountMessage::SelectSaved(id) => {
            if app.account.operation.is_busy() {
                return Task::none();
            }
            let mut repository = app.account.repository().clone();
            if let Err(error) = repository.activate(&id) {
                return app.show_error_toast(error.to_string());
            }
            app.account.cancel_profile_editor();
            app.account.cancel_password_editor();
            let generation = app.account.begin(AccountOperation::Saving);
            persist_task(repository, generation, AccountNotice::Switch)
        }
        AccountMessage::DeleteSaved(id) => {
            if app.account.operation.is_busy() {
                return Task::none();
            }
            let mut repository = app.account.repository().clone();
            if let Err(error) = repository.remove(&id) {
                return app.show_error_toast(error.to_string());
            }
            app.account.cancel_profile_editor();
            app.account.cancel_password_editor();
            let generation = app.account.begin(AccountOperation::Saving);
            persist_task(repository, generation, AccountNotice::Delete)
        }
        AccountMessage::SignOut => {
            if app.account.operation.is_busy() {
                return Task::none();
            }
            let mut repository = app.account.repository().clone();
            repository.sign_out();
            app.account.cancel_profile_editor();
            app.account.cancel_password_editor();
            let generation = app.account.begin(AccountOperation::Saving);
            persist_task(repository, generation, AccountNotice::SignOut)
        }
        AccountMessage::RepositoryPersisted { generation, notice, result } => {
            if !app.account.is_current(generation) {
                return Task::none();
            }
            app.account.finish();
            match result {
                Ok(repository) => {
                    app.account.replace_repository(repository);
                    app.account.manual_input = ManualSkeyInput::default();
                    app.account.menu_open = false;
                    if app.account.active_account().is_none() {
                        app.account.details_open = false;
                    }
                    let feedback = match notice {
                        AccountNotice::ManualLogin => app.show_success_toast("账户已登录"),
                        AccountNotice::BrowserLogin => {
                            app.show_success_toast("登录成功，请手动关闭授权页面")
                        }
                        AccountNotice::Switch => app.show_success_toast("账户已切换"),
                        AccountNotice::SignOut => app.show_success_toast("已退出登录"),
                        AccountNotice::Delete => app.show_success_toast("账户已删除"),
                    };
                    let reload = crate::app::message::skill_hub::account_changed(app);
                    let sync = shiju_key_sync_task(app, notice);
                    let models = super::settings::providers::managed_shiju::sync(app);
                    Task::batch([feedback, reload, sync, models])
                }
                Err(error) => app.show_error_toast(format!("账户配置保存失败：{error}")),
            }
        }
        AccountMessage::ShijuKeySyncFinished(result) => match result {
            Ok(ShijuKeySyncOutcome::Updated | ShijuKeySyncOutcome::SkillNotInstalled) => {
                Task::none()
            }
            Err(_) => {
                app.show_warning_toast("识矩登录信息自动同步失败，可在安装页点击同步登录信息重试")
            }
        },
    }
}

#[cfg(test)]
#[path = "account_tests.rs"]
mod account_tests;
