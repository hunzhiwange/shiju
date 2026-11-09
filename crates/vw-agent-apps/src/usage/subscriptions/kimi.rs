//! Official Kimi quota for the account selected by the authorized execution host.

use reqwest::Request;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthService},
    apps_usage::{
        UsageCredentialState, UsageQueryData, UsageQueryError, UsageQueryErrorCode, UsageQueryKind,
        UsageQueryOutcome, UsageQueryResult, UsageSource, UsageSubscription,
    },
};

use crate::{
    auth::{
        credential_file::{AccountCredential, CredentialFileStore},
        kimi::KimiDeviceAuth,
        kimi_account::{KimiAccountTarget, KimiNativeTarget},
    },
    usage::coding::kimi::query_windows,
};

/// Query a verified managed account. The caller must authorize regional egress
/// and may supply the account's authorized native profile for renewal.
pub async fn query_kimi_account(
    auth: &KimiDeviceAuth,
    credentials: &CredentialFileStore,
    account: KimiAccountTarget<'_>,
    native: Option<KimiNativeTarget<'_>>,
    now: TimestampMs,
    correlation_id: &str,
) -> Result<UsageQueryResult, AuthErrorCode> {
    query_at(auth, credentials, account, native, now, correlation_id, |_| {}).await
}

pub(super) async fn query_at(
    auth: &KimiDeviceAuth,
    credentials: &CredentialFileStore,
    account: KimiAccountTarget<'_>,
    native: Option<KimiNativeTarget<'_>>,
    now: TimestampMs,
    correlation_id: &str,
    mut route: impl FnMut(&mut Request),
) -> Result<UsageQueryResult, AuthErrorCode> {
    use UsageCredentialState::{Expired, NotFound, RefreshPending, Valid};
    use UsageQueryErrorCode::{CredentialUnavailable, UpstreamUnauthorized, UpstreamUnavailable};

    let KimiAccountTarget { scope, account_id, region, .. } = account;
    if scope.service != AuthService::KimiOauth {
        return Err(AuthErrorCode::Unsupported);
    }
    let failure = |queried_at, state, code, message: &str| {
        result(
            queried_at,
            state,
            UsageQueryOutcome::Failure {
                kind: UsageQueryKind::OfficialSubscription,
                error: UsageQueryError {
                    code,
                    message: message.into(),
                    correlation_id: correlation_id.into(),
                },
            },
        )
    };
    let Some(AccountCredential::KimiOauth(saved)) =
        credentials.read_managed_account(scope, account_id)?
    else {
        return Ok(failure(None, NotFound, CredentialUnavailable, "Kimi sign-in unavailable"));
    };
    if saved.region != region {
        return Err(AuthErrorCode::IdentityMismatch);
    }

    let access = if saved.expires_at.0 > now.0 {
        saved.access_token
    } else {
        let Some(native) = native else {
            return Ok(failure(
                None,
                RefreshPending,
                CredentialUnavailable,
                "Kimi sign-in needs renewal",
            ));
        };
        match auth.refresh_usage_token(credentials, account, native, now, &mut route).await {
            Ok(access) => access,
            Err(AuthErrorCode::AccountInUse) => {
                return Ok(failure(
                    None,
                    RefreshPending,
                    CredentialUnavailable,
                    "Kimi sign-in needs renewal",
                ));
            }
            Err(AuthErrorCode::ReauthRequired) => {
                return Ok(failure(None, Expired, CredentialUnavailable, "Kimi sign-in expired"));
            }
            Err(AuthErrorCode::UpstreamFailure) => {
                return Ok(failure(
                    Some(now),
                    RefreshPending,
                    UpstreamUnavailable,
                    "Kimi token renewal failed",
                ));
            }
            Err(error) => return Err(error),
        }
    };
    let outcome = match query_windows(region, &access, route).await {
        Ok(windows) => UsageQueryOutcome::Success {
            data: UsageQueryData::OfficialSubscription(UsageSubscription {
                subscription_type: None,
                rate_limit_tier: None,
                windows,
                extra_usage: None,
                credits_balance: None,
                reset_credits: None,
            }),
        },
        Err(UpstreamUnauthorized) => {
            return Ok(failure(
                Some(now),
                Expired,
                UpstreamUnauthorized,
                "Kimi rejected the credential",
            ));
        }
        Err(code) => return Ok(failure(Some(now), Valid, code, "Kimi quota query failed")),
    };
    Ok(result(Some(now), Valid, outcome))
}

fn result(
    queried_at: Option<TimestampMs>,
    credential_state: UsageCredentialState,
    outcome: UsageQueryOutcome,
) -> UsageQueryResult {
    UsageQueryResult {
        source: UsageSource::Official("kimi_oauth".into()),
        queried_at,
        credential_state,
        credential_message: None,
        outcome,
    }
}
