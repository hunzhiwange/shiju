//! 每个连接器独立保存请求代次与授权流程，账号切换时全部失效。
use super::SkillHubMessage;
use crate::app::{
    App, Message, config,
    skill_hub::{
        LoadState,
        connectors::{ConnectorForm, ConnectorWork},
    },
};
use iced::Task;
use std::time::{Duration, Instant};
use vw_gateway_client::{
    ShijuConnector, ShijuConnectorAuthorizationStatus, ShijuConnectorBinding,
    ShijuConnectorChallenge, ShijuConnectorPoll,
};

#[derive(Clone)]
pub(crate) enum ConnectorMessage {
    Refresh,
    Edit(u64),
    Cancel,
    CancelAuthorization(u64),
    AccountId(String),
    AccountName(String),
    Variable(String, String),
    Save,
    Authorize(u64),
    Loaded(u64, String, Result<Vec<ShijuConnector>, String>),
    Saved(u64, String, u64, Result<ShijuConnectorBinding, String>),
    Started(u64, String, u64, Result<ShijuConnectorChallenge, String>),
    Poll(u64, String, u64),
    Polled(u64, String, u64, Result<ShijuConnectorPoll, String>),
}
impl std::fmt::Debug for ConnectorMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("ConnectorMessage(<redacted>)")
    }
}
pub(crate) fn message(message: ConnectorMessage) -> Message {
    Message::SkillHub(SkillHubMessage::Connectors(message))
}
fn same_account(app: &App, account_id: &str) -> bool {
    app.expert_hub.connectors.account_id.as_deref() == Some(account_id)
        && app.account.active_account().is_some_and(|account| account.id() == account_id)
}
fn current(app: &App, generation: u64, account_id: &str, id: u64) -> bool {
    same_account(app, account_id)
        && app
            .expert_hub
            .connectors
            .work
            .get(&id)
            .is_some_and(|operation| operation.generation == generation)
}
pub(crate) fn ensure_loaded(app: &mut App) -> Task<Message> {
    if matches!(app.expert_hub.connectors.list, LoadState::Idle | LoadState::NeedsLogin) {
        load(app)
    } else {
        Task::none()
    }
}
pub(crate) fn load(app: &mut App) -> Task<Message> {
    let Some(account) = app.account.active_account() else {
        app.expert_hub.connectors.clear(None);
        return Task::none();
    };
    let account_id = account.id().to_string();
    let credential = account.skill_hub_credential();
    let state = &mut app.expert_hub.connectors;
    if state.account_id.as_deref() != Some(&account_id) {
        state.clear(Some(account_id.clone()));
    }
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    state.list_generation = generation;
    state.list_loading = true;
    // 后台刷新不移除卡片、其它授权等待或正在编辑的弹窗。
    if !matches!(state.list, LoadState::Loaded(_)) {
        state.list = LoadState::Loading { generation };
    }
    Task::perform(
        async move {
            let credential = credential.gateway_credential().map_err(|error| error.to_string())?;
            config::gateway_client()?.shiju_connectors(&credential).await
        },
        move |result| message(ConnectorMessage::Loaded(generation, account_id, result)),
    )
}
pub(crate) fn update(app: &mut App, msg: ConnectorMessage) -> Task<Message> {
    match msg {
        ConnectorMessage::Refresh => {
            if app.expert_hub.connectors.list_loading {
                return Task::none();
            }
            return load(app);
        }
        ConnectorMessage::Cancel => {
            if !app.expert_hub.connectors.form_is_saving() {
                app.expert_hub.connectors.form = None;
            }
        }
        ConnectorMessage::CancelAuthorization(id) => {
            let state = &mut app.expert_hub.connectors;
            if state
                .work
                .get(&id)
                .is_some_and(|operation| !matches!(operation.work, ConnectorWork::Saving))
            {
                state.work.remove(&id);
            }
        }
        ConnectorMessage::Edit(id) => {
            let state = &mut app.expert_hub.connectors;
            if state.form_is_saving() || state.work.contains_key(&id) {
                return Task::none();
            }
            if let LoadState::Loaded(connectors) = &state.list {
                state.form = connectors
                    .iter()
                    .find(|connector| connector.company_platform_id == id)
                    .cloned()
                    .map(ConnectorForm::new);
            }
        }
        ConnectorMessage::AccountId(value) => {
            if let Some(form) = editable_form(app) {
                form.account_id = value;
            }
        }
        ConnectorMessage::AccountName(value) => {
            if let Some(form) = editable_form(app) {
                form.account_name = value;
            }
        }
        ConnectorMessage::Variable(name, value) => {
            if let Some(form) = editable_form(app) {
                if let Some(input) = form.values.get_mut(&name) {
                    *input = value;
                }
            }
        }
        ConnectorMessage::Loaded(generation, account_id, result) => {
            if !same_account(app, &account_id)
                || app.expert_hub.connectors.list_generation != generation
            {
                return Task::none();
            }
            app.expert_hub.connectors.list_loading = false;
            match result {
                Ok(items) => app.expert_hub.connectors.list = LoadState::Loaded(items),
                Err(error) => {
                    if matches!(app.expert_hub.connectors.list, LoadState::Loaded(_)) {
                        return app.show_error_toast(format!("连接器刷新失败：{error}"));
                    }
                    app.expert_hub.connectors.list = LoadState::Error(error);
                }
            }
        }
        ConnectorMessage::Save => {
            let state = &app.expert_hub.connectors;
            let Some(form) = &state.form else {
                return Task::none();
            };
            let id = form.connector.company_platform_id;
            if state.work.contains_key(&id) {
                return Task::none();
            }
            let request = match form.request() {
                Ok(request) => request,
                Err(error) => return app.show_error_toast(error),
            };
            let Some(account) = app.account.active_account() else {
                return load(app);
            };
            let account_id = account.id().to_string();
            if !same_account(app, &account_id) {
                return load(app);
            }
            let credential = account.skill_hub_credential();
            let generation = app.expert_hub.connectors.begin(id, ConnectorWork::Saving);
            return Task::perform(
                async move {
                    let credential =
                        credential.gateway_credential().map_err(|error| error.to_string())?;
                    config::gateway_client()?.shiju_connector_save(&credential, &request).await
                },
                move |result| message(ConnectorMessage::Saved(generation, account_id, id, result)),
            );
        }
        ConnectorMessage::Saved(generation, account_id, id, result) => {
            if !current(app, generation, &account_id, id) {
                return Task::none();
            }
            app.expert_hub.connectors.work.remove(&id);
            return match result {
                Ok(_) => {
                    if app
                        .expert_hub
                        .connectors
                        .form
                        .as_ref()
                        .is_some_and(|form| form.connector.company_platform_id == id)
                    {
                        app.expert_hub.connectors.form = None;
                    }
                    let toast = app.show_success_toast("账号绑定已保存");
                    Task::batch([toast, load(app)])
                }
                Err(error) => app.show_error_toast(format!("绑定保存失败：{error}")),
            };
        }
        ConnectorMessage::Authorize(id) => {
            let state = &app.expert_hub.connectors;
            if state.work.contains_key(&id)
                || !matches!(&state.list, LoadState::Loaded(items) if items.iter().any(|item| item.company_platform_id == id && item.authorization_available))
            {
                return Task::none();
            }
            let Some(account) = app.account.active_account() else {
                return load(app);
            };
            let account_id = account.id().to_string();
            if !same_account(app, &account_id) {
                return load(app);
            }
            let credential = account.skill_hub_credential();
            let generation = app.expert_hub.connectors.begin(id, ConnectorWork::Starting);
            return Task::perform(
                async move {
                    let credential =
                        credential.gateway_credential().map_err(|error| error.to_string())?;
                    config::gateway_client()?.shiju_connector_authorize(&credential, id).await
                },
                move |result| {
                    message(ConnectorMessage::Started(generation, account_id, id, result))
                },
            );
        }
        ConnectorMessage::Started(generation, account_id, id, result) => {
            if !current(app, generation, &account_id, id) {
                return Task::none();
            }
            let challenge = match result {
                Ok(challenge) => challenge,
                Err(error) => {
                    app.expert_hub.connectors.work.remove(&id);
                    return app.show_error_toast(format!("授权发起失败：{error}"));
                }
            };
            let valid_url = reqwest::Url::parse(&challenge.authorization_url).is_ok_and(|url| {
                url.scheme() == "https"
                    && url.host_str().is_some()
                    && url.username().is_empty()
                    && url.password().is_none()
            });
            if !valid_url || open::that(&challenge.authorization_url).is_err() {
                app.expert_hub.connectors.work.remove(&id);
                return app.show_error_toast("无法打开连接器授权页面，请重试");
            }
            let deadline =
                Instant::now() + Duration::from_secs(challenge.expires_in_seconds.min(3600));
            if let Some(operation) = app.expert_hub.connectors.work.get_mut(&id) {
                operation.work = ConnectorWork::Authorizing { challenge, deadline };
            }
            return schedule_poll(app, generation, account_id, id);
        }
        ConnectorMessage::Poll(generation, account_id, id) => {
            if !current(app, generation, &account_id, id) {
                return Task::none();
            }
            let Some(operation) = app.expert_hub.connectors.work.get(&id) else {
                return Task::none();
            };
            let ConnectorWork::Authorizing { challenge, deadline } = &operation.work else {
                return Task::none();
            };
            if Instant::now() >= *deadline {
                app.expert_hub.connectors.work.remove(&id);
                return app.show_error_toast("授权已过期，请重新授权");
            }
            let flow_id = challenge.flow_id.clone();
            let Some(account) = app.account.active_account() else {
                return Task::none();
            };
            let credential = account.skill_hub_credential();
            return Task::perform(
                async move {
                    let credential =
                        credential.gateway_credential().map_err(|error| error.to_string())?;
                    config::gateway_client()?.shiju_connector_poll(&credential, &flow_id).await
                },
                move |result| message(ConnectorMessage::Polled(generation, account_id, id, result)),
            );
        }
        ConnectorMessage::Polled(generation, account_id, id, result) => {
            if !current(app, generation, &account_id, id)
                || !app.expert_hub.connectors.work.get(&id).is_some_and(|operation| {
                    matches!(operation.work, ConnectorWork::Authorizing { .. })
                })
            {
                return Task::none();
            }
            let outcome = match result {
                Ok(poll) => match poll.status {
                    ShijuConnectorAuthorizationStatus::Pending => {
                        return schedule_poll(app, generation, account_id, id);
                    }
                    ShijuConnectorAuthorizationStatus::Approved => {
                        app.expert_hub.connectors.work.remove(&id);
                        let toast = app.show_success_toast("账号授权成功");
                        return Task::batch([toast, load(app)]);
                    }
                    ShijuConnectorAuthorizationStatus::Denied => "授权已取消".to_string(),
                    ShijuConnectorAuthorizationStatus::Expired => {
                        "授权已过期，请重新授权".to_string()
                    }
                },
                Err(error) => format!("授权查询失败：{error}"),
            };
            app.expert_hub.connectors.work.remove(&id);
            return app.show_error_toast(outcome);
        }
    }
    Task::none()
}
fn editable_form(app: &mut App) -> Option<&mut ConnectorForm> {
    if app.expert_hub.connectors.form_is_saving() {
        None
    } else {
        app.expert_hub.connectors.form.as_mut()
    }
}
fn schedule_poll(app: &App, generation: u64, account_id: String, id: u64) -> Task<Message> {
    let Some(operation) = app.expert_hub.connectors.work.get(&id) else {
        return Task::none();
    };
    let ConnectorWork::Authorizing { challenge, deadline } = &operation.work else {
        return Task::none();
    };
    let delay = Duration::from_secs(challenge.poll_interval_seconds.clamp(1, 30))
        .min(deadline.saturating_duration_since(Instant::now()));
    Task::perform(
        async move {
            tokio::time::sleep(delay).await;
        },
        move |_| message(ConnectorMessage::Poll(generation, account_id, id)),
    )
}
