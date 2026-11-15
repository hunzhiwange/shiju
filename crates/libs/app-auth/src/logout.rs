//! Upstream revocation capability policy.

use app_auth_core::flow::AuthAccountScope;
use vw_api_app_auth::{AuthErrorCode, AuthService};

/// Only the DSH protocol provides confirmed upstream revocation.
pub fn check_revocation_support(scope: AuthAccountScope<'_>) -> Result<(), AuthErrorCode> {
    scope.validate()?;
    if scope.service != AuthService::DshAccount {
        return Err(AuthErrorCode::Unsupported);
    }
    Ok(())
}
