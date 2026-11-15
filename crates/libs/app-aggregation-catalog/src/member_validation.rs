//! Eligibility rules for aggregation members and default providers.

use vw_api_app_auth::AuthService;
use vw_api_app_identity::{AppId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProviderRole {
    Member,
    Default,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemberValidationError {
    UnsupportedApp,
    InvalidMember,
    DefaultMemberRequired,
}

/// The executor classifies the saved provider and its binding before calling this rule.
/// A Codex official default still requires the account owner to verify login and recovery.
pub fn validate_provider(
    app: AppId,
    official_service: Option<AuthService>,
    role: ProviderRole,
) -> Result<(), MemberValidationError> {
    require_supported_app(app)?;
    match (app, official_service, role) {
        (_, None, _) | (AppId::Codex, Some(AuthService::CodexOauth), ProviderRole::Default) => {
            Ok(())
        }
        _ => Err(MemberValidationError::InvalidMember),
    }
}

pub fn require_supported_app(app: AppId) -> Result<(), MemberValidationError> {
    match app {
        AppId::Claude | AppId::Codex | AppId::ClaudeDesktop => Ok(()),
        _ => Err(MemberValidationError::UnsupportedApp),
    }
}
