//! Official Copilot premium quota for the resolved managed account.

use chrono::{DateTime, NaiveDate};
use reqwest::{Request, StatusCode};
use serde::Deserialize;
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthService},
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryError, UsageQueryErrorCode,
        UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
        UsageSubscription,
    },
};

use crate::{
    auth::{
        copilot::{CopilotAuth, valid_token},
        credential_file::{AccountCredential, CredentialFileStore},
    },
    usage::{
        binding::{UsageAccount, UsageCredentialRef},
        coding::zhipu::{read_json_response, transport_error},
    },
};

/// Query the host-resolved explicit, bound or default account. The host must
/// authorize egress and supply the authority from that account's login record.
pub async fn query_copilot(
    auth: &CopilotAuth,
    credentials: &CredentialFileStore,
    account: &UsageCredentialRef<'_>,
    authority: &str,
    now: TimestampMs,
    correlation_id: &str,
) -> Result<UsageQueryResult, AuthErrorCode> {
    query_at(auth, credentials, account, authority, now, correlation_id, |_| {}).await
}

pub(super) async fn query_at(
    auth: &CopilotAuth,
    credentials: &CredentialFileStore,
    account: &UsageCredentialRef<'_>,
    authority: &str,
    now: TimestampMs,
    correlation_id: &str,
    route: impl FnOnce(&mut Request),
) -> Result<UsageQueryResult, AuthErrorCode> {
    use UsageCredentialState::{Expired, NotFound, ParseError, Valid};
    use UsageQueryErrorCode::{CredentialUnavailable, UpstreamUnauthorized};

    let scope = account.scope();
    let UsageAccount::Managed(id) = account.account() else {
        return Err(AuthErrorCode::Unsupported);
    };
    if scope.service != AuthService::GithubCopilot {
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
    let token = match credentials.read_managed_account(scope, id)? {
        Some(AccountCredential::CopilotToken(token)) if valid_token(&token) => token,
        None => {
            return Ok(failure(
                None,
                NotFound,
                CredentialUnavailable,
                "Copilot sign-in unavailable",
            ));
        }
        Some(_) => {
            return Ok(failure(
                None,
                ParseError,
                CredentialUnavailable,
                "Invalid Copilot credential",
            ));
        }
    };
    let mut request = auth.copilot_user_request(authority, &token)?;
    route(&mut request);
    let queried_at = Some(now);
    let outcome = match fetch(auth, request, &token).await {
        Ok(subscription) => {
            UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(subscription) }
        }
        Err(UpstreamUnauthorized) => {
            return Ok(failure(
                queried_at,
                Expired,
                UpstreamUnauthorized,
                "Copilot sign-in expired",
            ));
        }
        Err(code) => {
            return Ok(failure(queried_at, Valid, code, "Copilot quota query failed"));
        }
    };
    Ok(result(queried_at, Valid, outcome))
}

async fn fetch(
    auth: &CopilotAuth,
    request: Request,
    token: &str,
) -> Result<UsageSubscription, UsageQueryErrorCode> {
    let response = auth.client.execute(request).await.map_err(transport_error)?;
    if response.status() == StatusCode::FORBIDDEN {
        return Err(UsageQueryErrorCode::UpstreamRejected);
    }
    let body: CopilotUsage = read_json_response(response).await?;
    body.subscription(token)
}

#[derive(Deserialize)]
struct CopilotUsage {
    copilot_plan: String,
    quota_reset_date: String,
    quota_snapshots: QuotaSnapshots,
}

#[derive(Deserialize)]
struct QuotaSnapshots {
    premium_interactions: PremiumQuota,
}

#[derive(Deserialize)]
struct PremiumQuota {
    entitlement: i64,
    remaining: i64,
}

impl CopilotUsage {
    fn subscription(self, token: &str) -> Result<UsageSubscription, UsageQueryErrorCode> {
        let Self { copilot_plan: plan, quota_reset_date: reset, quota_snapshots } = self;
        if plan.is_empty()
            || plan.len() > 64
            || !plan.bytes().all(|byte| byte.is_ascii_alphanumeric() || b" _-+".contains(&byte))
            || plan.contains(token)
            || reset.contains(token)
            || (NaiveDate::parse_from_str(&reset, "%Y-%m-%d").is_err()
                && DateTime::parse_from_rfc3339(&reset).is_err())
        {
            return Err(UsageQueryErrorCode::InvalidResponse);
        }

        let PremiumQuota { entitlement, remaining } = quota_snapshots.premium_interactions;
        let used = (i128::from(entitlement) - i128::from(remaining)) as f64;
        let utilization = if entitlement > 0 { used / entitlement as f64 * 100.0 } else { 0.0 };
        Ok(UsageSubscription {
            subscription_type: Some(plan),
            rate_limit_tier: None,
            windows: vec![UsageQuotaWindow {
                name: "premium".into(),
                plan_label: None,
                utilization: UsageNumber::try_from(utilization).ok(),
                used: UsageNumber::try_from(used).ok(),
                total: UsageNumber::try_from(entitlement as f64).ok(),
                remaining: UsageNumber::try_from(remaining as f64).ok(),
                unit: Some("requests".into()),
                resets_at: Some(reset),
            }],
            extra_usage: None,
            credits_balance: None,
            reset_credits: None,
        })
    }
}

fn result(
    queried_at: Option<TimestampMs>,
    credential_state: UsageCredentialState,
    outcome: UsageQueryOutcome,
) -> UsageQueryResult {
    UsageQueryResult {
        source: UsageSource::Official("github_copilot".into()),
        queried_at,
        credential_state,
        credential_message: None,
        outcome,
    }
}
