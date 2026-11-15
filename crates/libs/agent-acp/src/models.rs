//! ACP 可选模型来自与原生执行相同的租户和项目模型目录。

use std::collections::HashMap;

use agent_client_protocol::{ModelInfo, SessionModelState};
use sha2::{Digest, Sha256};
use acp_server::server::BackendError;

use agent_provider::provider;
use session_port::SessionScope;

fn selection_key(scope: &SessionScope, session_id: &str) -> String {
    let identity = serde_json::json!([
        scope.tenant().platform_id().as_ref(),
        scope.tenant().company_id().as_ref(),
        scope.directory(),
        session_id,
    ]);
    hex::encode(Sha256::digest(identity.to_string().as_bytes()))
}

pub async fn load_selection(
    scope: &SessionScope,
    session_id: &str,
) -> Result<String, BackendError> {
    match agent_storage::read(&["acp_session_model", &selection_key(scope, session_id)]).await {
        Ok(model) => Ok(model),
        Err(agent_storage::Error::NotFound(_)) => Ok("auto".into()),
        Err(error) => Err(BackendError::Internal(error.to_string())),
    }
}

pub async fn save_selection(
    scope: &SessionScope,
    session_id: &str,
    model_id: &str,
) -> Result<(), BackendError> {
    agent_storage::write_atomic(
        &["acp_session_model", &selection_key(scope, session_id)],
        &model_id,
    )
    .await
    .map_err(|error| BackendError::Internal(error.to_string()))
}

pub fn model_state(providers: HashMap<String, provider::Info>, current: String) -> SessionModelState {
    let mut models = providers
        .into_values()
        .flat_map(|provider| {
            provider.models.into_values().map(move |model| {
                ModelInfo::new(
                    format!("{}/{}", provider.id, model.id),
                    format!("{} · {}", provider.name, model.name),
                )
            })
        })
        .collect::<Vec<_>>();
    models.sort_by(|left, right| left.model_id.0.cmp(&right.model_id.0));
    models.insert(
        0,
        ModelInfo::new("auto", "Auto").description("Use automatic model selection".to_string()),
    );
    SessionModelState::new(current, models)
}
