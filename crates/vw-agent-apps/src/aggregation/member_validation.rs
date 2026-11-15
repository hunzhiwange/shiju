use crate::providers::live::commit::{AppState, Mode};
pub use app_aggregation_catalog::member_validation::{MemberValidationError, ProviderRole, validate_provider};
use app_aggregation_catalog::member_validation::require_supported_app;
use vw_api_app_identity::AppId;

/// Validate against the settled state held under the application's transaction lock.
pub fn validate_member_removal(
    app: AppId,
    state: &AppState,
    provider_id: &str,
) -> Result<(), MemberValidationError> {
    require_supported_app(app)?;
    if state.stack.enabled
        && state.mode.as_ref().is_some_and(|mode| {
            mode.mode == Mode::Proxy && mode.proxy_route.as_deref() == Some(provider_id)
        })
    {
        return Err(MemberValidationError::DefaultMemberRequired);
    }
    Ok(())
}
