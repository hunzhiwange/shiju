//! ACP 可选模型来自与原生执行相同的租户和项目模型目录。

use std::collections::HashMap;

use agent_client_protocol::{ModelInfo, SessionModelState};
use sha2::{Digest, Sha256};
use vw_acp::server::BackendError;

use crate::provider::provider;
use crate::session::runtime::SessionScope;

fn selection_key(scope: &SessionScope, session_id: &str) -> String {
    let identity = serde_json::json!([
        scope.tenant().platform_id().as_ref(),
        scope.tenant().company_id().as_ref(),
        scope.directory(),
        session_id,
    ]);
    hex::encode(Sha256::digest(identity.to_string().as_bytes()))
}

pub(super) async fn load_selection(
    scope: &SessionScope,
    session_id: &str,
) -> Result<String, BackendError> {
    match crate::storage::read(&["acp_session_model", &selection_key(scope, session_id)]).await {
        Ok(model) => Ok(model),
        Err(crate::storage::Error::NotFound(_)) => Ok("auto".into()),
        Err(error) => Err(BackendError::Internal(error.to_string())),
    }
}

pub(super) async fn save_selection(
    scope: &SessionScope,
    session_id: &str,
    model_id: &str,
) -> Result<(), BackendError> {
    crate::storage::write_atomic(
        &["acp_session_model", &selection_key(scope, session_id)],
        &model_id,
    )
    .await
    .map_err(|error| BackendError::Internal(error.to_string()))
}

fn model_state(providers: HashMap<String, provider::Info>, current: String) -> SessionModelState {
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

pub(super) async fn session_models(
    scope: SessionScope,
    current: String,
) -> Result<SessionModelState, BackendError> {
    crate::project::instance::provide(scope.directory().to_path_buf(), None, move || {
        Box::pin(async move {
            crate::session::ui_store::with_tenant_scope(scope.tenant().clone(), async move {
                let providers = provider::list()
                    .await
                    .map_err(|error| BackendError::Internal(error.to_string()))?;
                let models = model_state(providers, current);
                if !models
                    .available_models
                    .iter()
                    .any(|model| model.model_id == models.current_model_id)
                {
                    return Err(BackendError::InvalidRequest("unknown or disabled model".into()));
                }
                Ok(models)
            })
            .await
        })
    })
    .await
    .map_err(|error| BackendError::Internal(error.to_string()))?
}

#[cfg(test)]
#[path = "model_tests.rs"]
mod tests;
