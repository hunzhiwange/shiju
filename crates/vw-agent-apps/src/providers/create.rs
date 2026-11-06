//! Create without activating. Storage remains owned by the caller's tenant store.
use super::{validate_identity, validate_profile};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderSummary, AppProviderWrite, create::*},
};

/// The callback must atomically insert if absent, returning false on a duplicate.
/// No native path, activation or host access is part of this operation.
pub fn create(
    app: AppId,
    id: &str,
    request: &AppProviderCreate,
    insert: impl FnOnce(&AppProviderWrite) -> Result<bool, String>,
) -> Result<AppProviderSummary, String> {
    validate_identity(id)?;
    if request.add_to_live {
        return Err("live activation is not supported by create-only operation".into());
    }
    let ProviderSecretWrite::Replace { value } = &request.api_key else {
        return Err("new application provider requires explicit credential replacement".into());
    };
    let profile = AppProviderWrite {
        name: request.name.clone(),
        protocol: request.protocol,
        api_key: value.clone(),
        base_url: request.base_url.clone(),
        model: request.model.clone(),
    };
    validate_profile(app, &profile)?;
    if !insert(&profile)? {
        return Err("application provider already exists".into());
    }
    Ok(AppProviderSummary { id: id.into(), app_id: app, name: profile.name, configured: true })
}
