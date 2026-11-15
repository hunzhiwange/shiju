//! Scoped managed account defaults.

use app_auth_core::flow::AuthAccountScope;
use vw_api_app_auth::{AuthErrorCode, AuthService};

/// Tenant platform and company are supplied separately to the settings store.
pub fn managed_default_key(scope: AuthAccountScope<'_>) -> Result<String, AuthErrorCode> {
    scope.validate()?;
    match scope.service {
        AuthService::GithubCopilot | AuthService::CodexOauth | AuthService::XaiOauth => {
            serde_json::to_string(&(scope.principal, scope.host, scope.service))
                .map_err(|_| AuthErrorCode::InvalidInput)
        }
        _ => Err(AuthErrorCode::Unsupported),
    }
}
