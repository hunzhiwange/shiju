use crate::app::state::app_providers::{
    edit::{EditForm, Mode},
    list::{ListState, Scope},
};
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::app_providers::{agy::*, create::ProviderSecretWrite},
};
#[derive(Clone)]
pub enum Field {
    Model(String),
    BaseUrl(String),
}
pub(crate) fn change(form: &mut EditForm, field: Field) {
    let Some(draft) = form.agy.as_mut() else { return };
    match field {
        Field::Model(v) => draft.model = v,
        Field::BaseUrl(v) => draft.base_url = v,
    }
}
pub(crate) async fn load(
    client: GatewayClient,
    scope: Scope,
    id: &str,
    token: &str,
) -> Result<AgyFormView, String> {
    client.agy_form_read(id, &AgyTarget { target_host_id: scope.host }, token).await.map_err(error)
}
pub(crate) async fn save(
    client: GatewayClient,
    scope: Scope,
    form: EditForm,
    token: String,
) -> Result<(), String> {
    let draft = form.agy.ok_or("此 Agy 档案不可编辑。")?;
    let api_key = if form.clear_key {
        ProviderSecretWrite::Clear
    } else if form.replace_key || (!draft.has_saved_key && !form.key.is_empty()) {
        ProviderSecretWrite::Replace { value: form.key }
    } else {
        ProviderSecretWrite::Keep
    };
    let optional = |s: String| (!s.is_empty()).then_some(s);
    let body = AgyFormWrite {
        target_host_id: scope.host,
        fields: form.fields,
        model: optional(draft.model),
        base_url: optional(draft.base_url),
        api_key,
        revision: draft.revision,
    };
    let result = if matches!(form.mode, Mode::Create) {
        client.agy_form_create(&form.id, &body, &token).await
    } else {
        client.agy_form_update(&form.id, &body, &token).await
    }
    .map_err(error)?;
    if !result.catalog_saved {
        return Err("供应商未保存，请重试。".into());
    }
    Ok(())
}
pub(crate) fn finish(
    state: &mut ListState,
    scope: &Scope,
    generation: u64,
    result: &Result<(), String>,
) -> bool {
    if state.scope.as_ref() != Some(scope) || state.generation != generation {
        return false;
    }
    let Some(form) = state.form.as_mut().filter(|f| f.saving && f.agy.is_some()) else {
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
        AgentAppsReadError::Http { status: 400 | 413 | 422, .. } => {
            "执行端未接受 Agy 配置，请检查模型、地址及凭据。"
        }
        AgentAppsReadError::Http { status: 401 | 403, .. } => "无权操作此主机的 Agy 档案。",
        AgentAppsReadError::Http { status: 404, .. } => "供应商不存在，请刷新列表。",
        AgentAppsReadError::Http { status: 409, .. } => "档案或运行状态已变更，请重新打开后重试。",
        AgentAppsReadError::Http { status: 501, .. } => "此执行主机不支持 Agy 配置。",
        _ => "Agy 请求失败，请检查网关连接后重试。",
    }
    .into()
}
