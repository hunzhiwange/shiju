use crate::app::state::app_providers::{
    edit::{EditForm, Mode},
    list::{ListState, Scope},
};
use iced::widget::text_editor;
use vw_gateway_client::{
    AgentAppsReadError, GatewayClient,
    vw_api_types::app_providers::{
        codex_catalog::{CodexFormView, CodexLiveResult, CodexModelMapping, CodexTarget},
        create::ProviderSecretWrite,
    },
};

#[derive(Clone)]
pub enum Field {
    Toml(text_editor::Action),
    AddModel,
    RemoveModel(usize),
    ModelId(usize, String),
    ModelName(usize, String),
    Apply(bool),
}

pub(crate) fn change(form: &mut EditForm, field: Field) {
    let Some(draft) = form.codex.as_mut() else { return };
    match field {
        Field::Toml(action) => draft.toml.perform(action),
        Field::Apply(value) => draft.apply = value,
        Field::AddModel => {
            if let Some(models) = &mut draft.models {
                models.push(CodexModelMapping { slug: String::new(), display_name: String::new() });
            }
        }
        Field::RemoveModel(i) => {
            if let Some(models) = draft.models.as_mut().filter(|rows| i < rows.len()) {
                models.remove(i);
            }
        }
        Field::ModelId(i, value) => {
            if let Some(model) = draft.models.as_mut().and_then(|rows| rows.get_mut(i)) {
                model.slug = value;
            }
        }
        Field::ModelName(i, value) => {
            if let Some(model) = draft.models.as_mut().and_then(|rows| rows.get_mut(i)) {
                model.display_name = value;
            }
        }
    }
}

pub(crate) async fn load(
    client: GatewayClient,
    scope: Scope,
    id: &str,
    token: &str,
) -> Result<CodexFormView, String> {
    client
        .codex_form_read(id, &CodexTarget { target_host_id: scope.host }, token)
        .await
        .map_err(error)
}

pub(crate) async fn save(
    client: GatewayClient,
    scope: Scope,
    form: EditForm,
    token: String,
) -> Result<CodexLiveResult, String> {
    let draft = form.codex.as_ref().ok_or("当前档案不支持 Codex TOML 编辑。")?;
    let key = if form.clear_key {
        ProviderSecretWrite::Clear
    } else if form.replace_key || (!draft.has_saved_key && !form.key.is_empty()) {
        ProviderSecretWrite::Replace { value: form.key.clone() }
    } else {
        ProviderSecretWrite::Keep
    };
    let request = draft.request(scope.host, form.fields, key);
    let result = if matches!(form.mode, Mode::Create) {
        client.codex_form_create(&form.id, &request, &token).await
    } else {
        client.codex_form_update(&form.id, &request, &token).await
    }
    .map_err(error)?;
    if !result.catalog_saved {
        return Err(if result.live == CodexLiveResult::RecoveryRequired {
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
    result: &Result<CodexLiveResult, String>,
) -> bool {
    if state.scope.as_ref() != Some(scope) || state.generation != generation {
        return false;
    }
    let Some(form) = state.form.as_mut().filter(|form| form.saving && form.codex.is_some()) else {
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
            "执行端未接受 Codex 配置，请检查 TOML、模型映射和凭据。"
        }
        AgentAppsReadError::Http { status: 401 | 403, .. } => "无权读取或保存此主机的 Codex 配置。",
        AgentAppsReadError::Http { status: 404, .. } => "供应商不存在，请刷新列表。",
        AgentAppsReadError::Http { status: 409, .. } => "配置或事务状态已变更，请刷新后重试。",
        AgentAppsReadError::Http { status: 501, .. } => "此执行主机不支持 Codex 配置。",
        _ => "Codex 配置请求失败，请检查网关连接后重试。",
    }
    .into()
}
