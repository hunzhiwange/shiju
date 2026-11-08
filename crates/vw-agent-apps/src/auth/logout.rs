//! Host-owned logout and remote revocation.

use super::{
    accounts::{remove_managed_claude, remove_managed_oauth_file},
    copilot::read_json_limited,
    credential_file::CredentialFileStore,
    dsh_account::DshAccount,
    dsh_login::{PLATFORM_ORIGIN, platform_client, platform_identity},
    flow::{AuthAccountScope, AuthFlows},
    kimi_account::KimiNativeTarget,
};
use reqwest::Request;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthRegion, AuthService},
};

const DSH_LOGOUT_PATH: &str = "/auth-api/v0/users/logout";
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

/// Resolve a logout target within the authorized service and owner scope.
#[derive(Clone, Copy)]
pub enum LogoutSelection<'a> {
    Account(&'a str),
    Default,
    All,
}

/// Only the DSH protocol provides confirmed upstream revocation.
pub fn check_revocation_support(scope: AuthAccountScope<'_>) -> Result<(), AuthErrorCode> {
    scope.validate()?;
    if scope.service != AuthService::DshAccount {
        return Err(AuthErrorCode::Unsupported);
    }
    Ok(())
}

/// Invalidate private material while holding the existing account lease.
/// The host must have already verified the metadata for this scope and account.
pub fn clear_managed_credentials(
    credentials: &CredentialFileStore,
    scope: AuthAccountScope<'_>,
    account_id: &str,
    identity_sha256: [u8; 32],
    kimi: Option<(&KimiNativeTarget<'_>, AuthRegion)>,
    clear_default: impl FnOnce() -> Result<(), AuthErrorCode>,
) -> Result<(), AuthErrorCode> {
    scope.validate()?;
    let lease = match scope.service {
        AuthService::GithubCopilot => credentials.lease_copilot(scope, account_id, false),
        AuthService::XaiOauth => credentials.lease_xai(scope, account_id, false),
        AuthService::KimiOauth => credentials.lease_kimi(scope, account_id, false),
        AuthService::CodexOauth | AuthService::ClaudeOfficial => {
            let app =
                if scope.service == AuthService::CodexOauth { AppId::Codex } else { AppId::Claude };
            credentials
                .for_owner(scope.principal)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?
                .lease(scope.tenant, scope.host, app, account_id, false)
        }
        _ => return Err(AuthErrorCode::Unsupported),
    };
    let lease = match lease {
        Ok(lease) => lease,
        Err(error) if error == "official account not found" => return clear_default(),
        Err(error) if error == "official account is in use" => {
            return Err(AuthErrorCode::AccountInUse);
        }
        Err(_) => return Err(AuthErrorCode::StorageUnavailable),
    };
    if let Some((target, region)) = kimi {
        target.clear_matching_grant(scope, account_id, region, &lease)?;
    }
    match scope.service {
        AuthService::ClaudeOfficial => remove_managed_claude(&lease, identity_sha256)?,
        _ => remove_managed_oauth_file(&lease)?,
    }
    clear_default()
}

impl DshAccount<'_> {
    /// Clear the owned local grant and cancel the selected login attempts.
    pub fn logout(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        selection: LogoutSelection<'_>,
    ) -> Result<(), AuthErrorCode> {
        self.check_logout_scope(scope)?;
        match selection {
            LogoutSelection::Account(account_id) => self.check_account_id(account_id)?,
            LogoutSelection::Default => return Err(AuthErrorCode::Unsupported),
            LogoutSelection::All => {}
        }
        // The singleton native grant is the target of every DSH login attempt.
        flows.cancel_for_logout(scope, None)?;
        self.sign_out()
    }

    /// Return success only after the local grant has been cleared and the
    /// platform confirms revocation. Run on the authorized execution host.
    pub async fn revoke(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        locale: &str,
        timezone_offset_seconds: i32,
    ) -> Result<(), AuthErrorCode> {
        self.revoke_with(flows, scope, account_id, locale, timezone_offset_seconds, |_| {}).await
    }

    pub(super) async fn revoke_with(
        &self,
        flows: &AuthFlows,
        scope: AuthAccountScope<'_>,
        account_id: &str,
        locale: &str,
        timezone_offset_seconds: i32,
        route: impl FnOnce(&mut Request),
    ) -> Result<(), AuthErrorCode> {
        check_revocation_support(scope)?;
        self.check_logout_scope(scope)?;
        self.check_account_id(account_id)?;
        let revision = self.revocable_revision()?;
        // Every browser attempt for the singleton grant targets the same slot.
        flows.cancel_for_logout(scope, None)?;
        let token = self.take_revocable_grant(revision)?;

        let unconfirmed = |_| AuthErrorCode::RemoteRevocationUnconfirmed;
        let client = platform_client().map_err(unconfirmed)?;
        let mut request = platform_identity(
            client.post(format!("{PLATFORM_ORIGIN}{DSH_LOGOUT_PATH}")),
            locale,
            timezone_offset_seconds,
        )
        .header("x-dsh-auth-token", token)
        .build()
        .map_err(|_| AuthErrorCode::RemoteRevocationUnconfirmed)?;
        route(&mut request);
        let response = client
            .execute(request)
            .await
            .map_err(|_| AuthErrorCode::RemoteRevocationUnconfirmed)?;
        if !response.status().is_success() {
            return Err(AuthErrorCode::RemoteRevocationUnconfirmed);
        }
        let body: serde_json::Value =
            read_json_limited(response, MAX_RESPONSE_BYTES).await.map_err(unconfirmed)?;
        if body["code"].as_i64() != Some(0) {
            return Err(AuthErrorCode::RemoteRevocationUnconfirmed);
        }
        Ok(())
    }
}
