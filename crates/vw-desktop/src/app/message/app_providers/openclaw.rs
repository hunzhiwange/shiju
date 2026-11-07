use super::list::{self, ListMessage};
use crate::app::{
    App, Message,
    state::app_providers::{
        edit::{EditForm, Mode},
        list::{ListState, Scope},
    },
};
use iced::{Task, widget::text_editor};
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::{
        agent_apps::AppId,
        app_providers::{catalog::ProviderMembership, create::ProviderSecretWrite, openclaw::*},
    },
};

#[derive(Clone)]
pub enum Field {
    Declaration(text_editor::Action),
    UserAgent(bool),
    Apply(bool),
}

pub(crate) fn change(form: &mut EditForm, field: Field) {
    let Some(draft) = form.openclaw.as_mut() else { return };
    match field {
        Field::Declaration(action) => draft.declaration.perform(action),
        Field::UserAgent(value) => draft.user_agent = value,
        Field::Apply(value) => draft.apply = value,
    }
}

pub(crate) async fn load(
    client: GatewayClient,
    scope: Scope,
    id: &str,
    token: &str,
) -> Result<OpenClawFormView, String> {
    client
        .openclaw_form_read(id, &OpenClawTarget { target_host_id: scope.host }, token)
        .await
        .map_err(error)
}

pub(crate) async fn save(
    client: GatewayClient,
    scope: Scope,
    form: EditForm,
    token: String,
) -> Result<OpenClawLiveResult, String> {
    let draft = form.openclaw.ok_or("此档案的 OpenClaw 节点不可编辑。")?;
    let api_key = if form.clear_key {
        ProviderSecretWrite::Clear
    } else if form.replace_key || (!draft.has_saved_key && !form.key.is_empty()) {
        ProviderSecretWrite::Replace { value: form.key }
    } else {
        ProviderSecretWrite::Keep
    };
    let create = matches!(form.mode, Mode::Create);
    let activation = match (draft.apply, create) {
        (false, _) => OpenClawActivation::CatalogOnly,
        (true, true) => OpenClawActivation::Add,
        (true, false) => OpenClawActivation::IfMember,
    };
    let request = OpenClawFormWrite {
        target_host_id: scope.host,
        fields: form.fields,
        user_agent: draft.user_agent,
        declaration: draft.declaration.text(),
        api_key,
        activation,
        revision: draft.revision,
    };
    let result = if create {
        client.openclaw_form_create(&form.id, &request, &token).await
    } else {
        client.openclaw_form_update(&form.id, &request, &token).await
    }
    .map_err(error)?;
    if !result.catalog_saved {
        return Err(if result.live == OpenClawLiveResult::RecoveryRequired {
            "供应商未保存，执行端配置需要恢复。"
        } else {
            "供应商未保存，请重试。"
        }
        .into());
    }
    Ok(result.live)
}

pub(crate) fn finish(
    state: &mut ListState,
    scope: &Scope,
    generation: u64,
    result: &Result<OpenClawLiveResult, String>,
) -> bool {
    if state.scope.as_ref() != Some(scope) || state.generation != generation {
        return false;
    }
    let Some(form) = state.form.as_mut().filter(|form| form.saving && form.openclaw.is_some())
    else {
        return false;
    };
    form.saving = false;
    if result.is_ok() {
        state.form = None;
    }
    true
}

pub(crate) fn membership(state: &ListState, id: &str) -> ProviderMembership {
    state
        .snapshot
        .as_ref()
        .and_then(|snapshot| snapshot.catalog.rows.iter().find(|row| row.provider.id == id))
        .map_or(ProviderMembership::Unknown, |row| row.native_member)
}

pub(crate) fn change_membership(app: &mut App, id: String, add: bool) -> Task<Message> {
    let state = &mut app.app_providers;
    if state.loading || state.form.is_some() || state.editor.is_some() {
        return Task::none();
    }
    let expected = if add { ProviderMembership::Absent } else { ProviderMembership::Present };
    if membership(state, &id) != expected {
        return Task::none();
    }
    let Ok((client, scope)) = list::current(app, AppId::OpenClaw) else {
        return app.show_error_toast("无法连接网关，请刷新列表后重试。");
    };
    let state = &mut app.app_providers;
    if state.scope.as_ref() != Some(&scope) {
        return Task::none();
    }
    state.loading = true;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let token = app.apps_catalog.token.clone();
    let target = OpenClawTarget { target_host_id: scope.host.clone() };
    Task::perform(
        async move {
            if add {
                client.openclaw_provider_add(&id, &target, &token).await
            } else {
                client.openclaw_provider_remove_live(&id, &target, &token).await
            }
            .map(|result| result.live)
            .map_err(error)
        },
        move |result| {
            list::message(ListMessage::OpenClawMemberChanged {
                scope: scope.clone(),
                generation,
                add,
                result,
            })
        },
    )
}

pub(crate) fn membership_changed(
    app: &mut App,
    scope: Scope,
    generation: u64,
    add: bool,
    result: Result<OpenClawLiveResult, String>,
) -> Task<Message> {
    if app.app_providers.scope.as_ref() != Some(&scope)
        || app.app_providers.generation != generation
        || !app.app_providers.loading
        || list::current(app, scope.app).ok().map(|(_, scope)| scope).as_ref() != Some(&scope)
    {
        return Task::none();
    }
    app.app_providers.loading = false;
    let toast = match result {
        Ok(OpenClawLiveResult::Applied) if add => app.show_success_toast("已添加到 OpenClaw。"),
        Ok(OpenClawLiveResult::Applied) => {
            app.show_success_toast("已从 OpenClaw 移除，档案仍在目录中。")
        }
        Ok(OpenClawLiveResult::RecoveryRequired) => app.show_warning_toast("执行端配置需要恢复。"),
        Ok(_) => app.show_error_toast("OpenClaw 成员未变更，请刷新后重试。"),
        Err(error) => return app.show_error_toast(error),
    };
    Task::batch([toast, list::update(app, ListMessage::Refresh)])
}

fn error(error: AgentAppsReadError) -> String {
    match error {
        AgentAppsReadError::Http { status: 400 | 413 | 422, .. } => {
            "执行端未接受 OpenClaw 配置，请检查节点及凭据。"
        }
        AgentAppsReadError::Http { status: 401 | 403, .. } => "无权操作此主机的 OpenClaw 配置。",
        AgentAppsReadError::Http { status: 404, .. } => "供应商不存在或缺少原生节点，请刷新列表。",
        AgentAppsReadError::Http { status: 409, .. } => "配置或事务状态已变更，请重新打开后重试。",
        AgentAppsReadError::Http { status: 501, .. } => "此执行主机不支持 OpenClaw 配置。",
        _ => "OpenClaw 配置请求失败，请检查网关连接后重试。",
    }
    .into()
}
