//! Universal synchronization selection and identity rules.
use app_provider_validation::validate_identity;
use vw_api_app_identity::AppId;
use vw_api_app_provider_edit::universal_sync::{UniversalSyncError, UniversalSyncRequest};
pub fn validate_request(request: &UniversalSyncRequest) -> Result<(), String> {
    if request.apps.is_empty() || request.apps.len() > 14 {
        return Err("invalid universal sync selection".into());
    }
    for (i, app) in request.apps.iter().enumerate() {
        if request.apps[..i].contains(app) {
            return Err("invalid universal sync selection".into());
        }
    }
    Ok(())
}
pub fn child_id(app: AppId, id: &str) -> Result<String, UniversalSyncError> {
    let app = match app {
        AppId::Claude => "claude",
        AppId::Codex => "codex",
        AppId::Gemini => "gemini",
        _ => return Err(UniversalSyncError::UnsupportedApp),
    };
    let child = format!("universal-{app}-{id}");
    validate_identity(&child).map_err(|_| UniversalSyncError::InvalidTargetIdentity)?;
    Ok(child)
}
