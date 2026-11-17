//! 租户和项目目录上下文由宿主绑定。

use agent_client_protocol::SessionModelState;
use acp_server::server::BackendError;
use crate::provider::provider;
use crate::session::runtime::SessionScope;

#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use agent_acp::models::{load_selection, save_selection};

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
                let models = agent_acp::models::model_state(providers, current);
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
