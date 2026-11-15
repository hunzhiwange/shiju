use crate::{Scope,state::{edit::{EditForm,Mode,ProviderEditState},list::Snapshot}};
use super::provider_update::{State,Effect as Message};
use iced::Task;
use gateway_http::AgentAppsReadError;
use vw_gateway_client::GatewayClient;
use vw_api_app_provider_edit::catalog::ProviderMembership;
use vw_api_app_provider_base::create::ProviderSecretWrite;
use vw_api_app_provider_extended::opencode::*;

use iced::widget::text_editor;
#[derive(Clone)]
pub enum Field {
    Declaration(text_editor::Action),
    V2(bool),
    Apply(bool),
}

pub fn change(form: &mut EditForm, field: Field) {
    let Some(draft) = form.opencode.as_mut() else { return };
    match field {
        Field::Declaration(action) => draft.declaration.perform(action),
        Field::V2(value) if matches!(form.mode, Mode::Create) => {
            draft.format = if value { OpenCodeFormat::V2 } else { OpenCodeFormat::V1 };
            form.clear_key = false;
            form.replace_key = false;
            form.key.clear();
        }
        Field::V2(_) => {}
        Field::Apply(value) => draft.apply = value,
    }
}

pub async fn load(
    client: GatewayClient,
    scope: Scope,
    id: &str,
    token: &str,
) -> Result<OpenCodeFormView, String> {
    gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone())
        .opencode_form_read(id, &OpenCodeTarget { target_host_id: scope.host }, token)
        .await
        .map_err(error)
}

pub async fn save(
    client: GatewayClient,
    scope: Scope,
    form: EditForm,
    token: String,
) -> Result<OpenCodeLiveResult, String> {
    let client = gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone());
    let draft = form.opencode.ok_or("此档案的 OpenCode 节点不可编辑。")?;
    let api_key = if form.clear_key {
        ProviderSecretWrite::Clear
    } else if form.replace_key || (!draft.has_saved_key && !form.key.is_empty()) {
        ProviderSecretWrite::Replace { value: form.key }
    } else {
        ProviderSecretWrite::Keep
    };
    let create = matches!(form.mode, Mode::Create);
    let activation = match (draft.apply, create) {
        (false, _) => OpenCodeActivation::CatalogOnly,
        (true, true) => OpenCodeActivation::Add,
        (true, false) => OpenCodeActivation::IfMember,
    };
    let request = OpenCodeFormWrite {
        target_host_id: scope.host,
        fields: form.fields,
        format: draft.format,
        declaration: draft.declaration.text(),
        api_key,
        activation,
        revision: draft.revision,
    };
    let result = if create {
        client.opencode_form_create(&form.id, &request, &token).await
    } else {
        client.opencode_form_update(&form.id, &request, &token).await
    }
    .map_err(error)?;
    if !result.catalog_saved {
        return Err(if result.live == OpenCodeLiveResult::RecoveryRequired {
            "供应商未保存，执行端配置需要恢复。"
        } else {
            "供应商未保存，请重试。"
        }
        .into());
    }
    Ok(result.live)
}

pub fn finish(
    state: &mut ProviderEditState<'_>,
    scope: &Scope,
    generation: u64,
    result: &Result<OpenCodeLiveResult, String>,
) -> bool {
    if state.scope != Some(scope) || state.generation != generation {
        return false;
    }
    let Some(form) = state.form.as_mut().filter(|form| form.saving && form.opencode.is_some())
    else {
        return false;
    };
    form.saving = false;
    if result.is_ok() {
        *state.form = None;
    }
    true
}

pub fn membership(snapshot: Option<&Snapshot>, id: &str) -> ProviderMembership {
    snapshot
        .and_then(|snapshot| snapshot.catalog.rows.iter().find(|row| row.provider.id == id))
        .map_or(ProviderMembership::Unknown, |row| row.native_member)
}

pub fn change_membership(state: &mut State, current: Option<(GatewayClient, Scope)>, token: &str, snapshot: Option<&Snapshot>, id: String, add: bool) -> Task<Message> {
    if state.loading || state.form.is_some() || state.editor.is_some() {
        return Task::none();
    }
    let expected = if add { ProviderMembership::Absent } else { ProviderMembership::Present };
    if membership(snapshot, &id) != expected {
        return Task::none();
    }
    let Ok((client, scope)) = current.clone().ok_or(()) else {
        return super::provider_update::error("无法连接网关，请刷新列表后重试。");
    };
    if state.scope.as_ref() != Some(&scope) {
        return Task::none();
    }
    state.loading = true;
    state.generation = state.generation.wrapping_add(1);
    let generation = state.generation;
    let token = token.to_owned();
    let client = gateway_app_provider::GatewayAppProviderClient::new(client.transport().clone());
    let target = OpenCodeTarget { target_host_id: scope.host.clone() };
    Task::perform(
        async move {
            if add {
                client.opencode_provider_add(&id, &target, &token).await
            } else {
                client.opencode_provider_remove_live(&id, &target, &token).await
            }
            .map(|result| result.live)
            .map_err(error)
        },
        move |result| {
            Message::OpenCodeMemberChanged {
                scope: scope.clone(),
                generation,
                add,
                result,
            }
        },
    )
}

pub fn membership_changed(
    state: &mut State, current: Option<(GatewayClient,Scope)>,
    scope: Scope,
    generation: u64,
    add: bool,
    result: Result<OpenCodeLiveResult, String>,
) -> Task<Message> {
    if state.scope.as_ref() != Some(&scope)
        || state.generation != generation
        || !state.loading
        || current.clone().map(|(_, scope)| scope).as_ref() != Some(&scope)
    {
        return Task::none();
    }
    state.loading = false;
    let toast = match result {
        Ok(OpenCodeLiveResult::Applied) if add => super::provider_update::success("已添加到 OpenCode。"),
        Ok(OpenCodeLiveResult::Applied) => {
            super::provider_update::success("已从 OpenCode 移除，档案仍在目录中。")
        }
        Ok(OpenCodeLiveResult::RecoveryRequired) => super::provider_update::warning("执行端配置需要恢复。"),
        Ok(_) => super::provider_update::error("OpenCode 成员未变更，请刷新后重试。"),
        Err(error) => return super::provider_update::error(error),
    };
    Task::batch([toast, super::provider_update::refresh(state)])
}

fn error(error: AgentAppsReadError) -> String {
    match error {
        AgentAppsReadError::Http { status: 400 | 413 | 422, .. } => {
            "执行端未接受 OpenCode 配置，请检查节点、格式及凭据。"
        }
        AgentAppsReadError::Http { status: 401 | 403, .. } => "无权操作此主机的 OpenCode 配置。",
        AgentAppsReadError::Http { status: 404, .. } => "供应商不存在或缺少原生节点，请刷新列表。",
        AgentAppsReadError::Http { status: 409, .. } => "配置或事务状态已变更，请重新打开后重试。",
        AgentAppsReadError::Http { status: 501, .. } => "此执行主机不支持 OpenCode 配置。",
        _ => "OpenCode 配置请求失败，请检查网关连接后重试。",
    }
    .into()
}
