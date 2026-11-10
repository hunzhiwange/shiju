//! Eligibility rules for aggregation members and default providers.

use crate::providers::live::commit::{AppState, Mode};
use vw_api_types::{agent_apps::AppId, agent_apps_auth::AuthService};

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

fn require_supported_app(app: AppId) -> Result<(), MemberValidationError> {
    match app {
        AppId::Claude | AppId::Codex | AppId::ClaudeDesktop => Ok(()),
        _ => Err(MemberValidationError::UnsupportedApp),
    }
}
