use crate::app::state::app_providers::{
    edit::{EditForm, Mode},
    list::{ListState, Scope},
};
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::app_providers::claude::{ClaudeFormView, ClaudeLiveResult, ClaudeTarget},
};

#[derive(Clone)]
pub enum Field {
    ThirdParty(bool),
    BaseUrl(String),
    ApiKeyHeader(bool),
    Model(String),
    Apply(bool),
}

pub(crate) fn change(form: &mut EditForm, field: Field) {
    use crate::app::state::app_providers::claude::Kind;
    let Some(draft) = form.claude.as_mut().filter(|draft| draft.kind != Kind::Unsupported) else {
        return;
    };
    match field {
        Field::ThirdParty(true) => draft.kind = Kind::ThirdParty,
        Field::ThirdParty(false) => draft.kind = Kind::Official,
        Field::BaseUrl(value) => draft.base_url = value,
        Field::ApiKeyHeader(value) => draft.api_key_header = value,
        Field::Model(value) => draft.model = value,
        Field::Apply(value) => draft.apply = value,
    }
}

pub(crate) async fn load(
    client: GatewayClient,
    scope: Scope,
    id: &str,
    token: &str,
) -> Result<ClaudeFormView, String> {
    client
        .claude_form_read(id, &ClaudeTarget { target_host_id: scope.host }, token)
        .await
        .map_err(error)
}

pub(crate) async fn save(
    client: GatewayClient,
    scope: Scope,
    form: EditForm,
    token: String,
) -> Result<ClaudeLiveResult, String> {
    let request = form.claude_request(scope.host).ok_or("当前供应商配置不可通过此表单保存")?;
    let result = if matches!(form.mode, Mode::Create) {
        client.claude_form_create(&form.id, &request, &token).await
    } else {
        client.claude_form_update(&form.id, &request, &token).await
    }
    .map_err(error)?;
    if !result.catalog_saved {
        return Err("供应商未保存，请重试".into());
    }
    Ok(result.live)
}

pub(crate) fn finish(
    state: &mut ListState,
    scope: &Scope,
    generation: u64,
    result: &Result<ClaudeLiveResult, String>,
) -> bool {
    if state.scope.as_ref() != Some(scope) || state.generation != generation {
        return false;
    }
    let Some(form) = state.form.as_mut().filter(|form| form.saving && form.claude_editable())
    else {
        return false;
    };
    form.saving = false;
    if result.is_ok() {
        state.form = None;
    }
    true
}

fn error(error: AgentAppsReadError) -> String {
    match error {
        AgentAppsReadError::Http { status: 400 | 422, .. } => {
            "执行端未接受 Claude 配置，请检查名称、API 地址、密钥和模型。"
        }
        AgentAppsReadError::Http { status: 401 | 403, .. } => {
            "无权读取或保存此主机的 Claude 配置。"
        }
        AgentAppsReadError::Http { status: 404, .. } => "供应商不存在，请刷新列表。",
        AgentAppsReadError::Http { status: 409, .. } => "配置或事务状态已变更，请刷新后重试。",
        AgentAppsReadError::Http { status: 501, .. } => "此执行主机不支持 Claude 配置。",
        _ => "Claude 配置请求失败，请检查网关连接后重试。",
    }
    .into()
}
