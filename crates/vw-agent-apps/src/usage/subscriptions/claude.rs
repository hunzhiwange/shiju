//! Claude official quota reads a single authorized login snapshot.

use std::{io::ErrorKind, path::Path};

use chrono::{DateTime, NaiveDateTime};
use reqwest::{
    ClientBuilder,
    header::{ACCEPT, AUTHORIZATION, HeaderValue},
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
    apps_usage::{
        UsageCredentialState, UsageExtraUsage, UsageNumber, UsageQueryData, UsageQueryError,
        UsageQueryErrorCode, UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow,
        UsageSource, UsageSubscription,
    },
};

use crate::{
    auth::{
        claude_keychain::{ClaudeKeychain, MacOsKeychain},
        claude_rotation::{default_login_available, expiry},
        credential_file::AccountCredential,
        credential_lease::CredentialLease,
        flow::AuthAccountScope,
    },
    providers::{
        claude_login::{LoginHost, LoginMaterial},
        live::{
            commit::{ExecutionHostId, ProviderTransactions},
            write_guard::WriteError,
        },
    },
    usage::{
        binding::{UsageAccount, UsageCredentialRef},
        coding::zhipu::{client_builder, read_json_response, transport_error},
    },
};

#[derive(Debug)]
pub struct ClaudeUsage {
    pub quota: UsageQueryResult,
    /// Metadata from the same credential, available even when the quota query fails.
    pub subscription_type: Option<String>,
    pub rate_limit_tier: Option<String>,
}

/// Query the fixed Anthropic OAuth usage endpoint. The caller authorizes use of
/// the resolved account, the host's HOME and access to api.anthropic.com.
/// The managed lookup must lease the scoped account and return its stored identity digest.
/// Snapshot errors are authorization errors. The quota result reports token state for
/// the authorization service to handle. This adapter performs no credential rotation.
pub async fn query_claude(
    credential: &UsageCredentialRef<'_>,
    home: &Path,
    transactions: &ProviderTransactions,
    load_managed: impl FnOnce(
        AuthAccountScope<'_>,
        &str,
    ) -> Result<(CredentialLease, [u8; 32]), AuthErrorCode>,
    now: TimestampMs,
    correlation_id: &str,
) -> Result<ClaudeUsage, AuthErrorCode> {
    query_at(
        LoginSlots { home, transactions, keychain: &MacOsKeychain },
        credential,
        load_managed,
        client_builder(),
        "https://api.anthropic.com",
        now,
        correlation_id,
    )
    .await
}

pub(super) struct LoginSlots<'a> {
    pub home: &'a Path,
    pub transactions: &'a ProviderTransactions,
    pub keychain: &'a dyn ClaudeKeychain,
}

impl LoginSlots<'_> {
    fn read(
        &self,
        credential: &UsageCredentialRef<'_>,
        load_managed: impl FnOnce(
            AuthAccountScope<'_>,
            &str,
        ) -> Result<(CredentialLease, [u8; 32]), AuthErrorCode>,
    ) -> Result<Option<LoginMaterial>, AuthErrorCode> {
        let scope = credential.scope();
        if scope.service != AuthService::ClaudeOfficial {
            return Err(AuthErrorCode::Unsupported);
        }
        let tx = self.transactions;
        if &ExecutionHostId::new(scope.host).map_err(slot_error)? != tx.execution_host() {
            return Err(AuthErrorCode::Forbidden);
        }

        let _directory_lock = tx.lock_claude_directory().map_err(slot_error)?;
        let state = tx.state(AppId::Claude).map_err(slot_error)?;
        let owner = state.claude_managed_login.as_ref();
        let owns_default = owner.is_some_and(|owner| {
            &owner.tenant == scope.tenant
                && owner.principal == scope.principal
                && owner.host == scope.host
        });

        match credential.account() {
            UsageAccount::Managed(id) => {
                let (lease, expected) = load_managed(scope, id)?;
                let AccountCredential::ClaudeDirectory(directory) =
                    lease.read().map_err(|_| AuthErrorCode::StorageUnavailable)?
                else {
                    return Err(AuthErrorCode::ReauthRequired);
                };
                let is_live = owns_default && owner.is_some_and(|owner| owner.account_id == *id);
                let material = if is_live
                    && default_login_available(tx, scope.tenant, &state).map_err(slot_error)?
                {
                    self.read_default()?.ok_or(AuthErrorCode::ReauthRequired)?
                } else {
                    LoginMaterial::locked_account(directory.path(), self.keychain)
                        .map_err(slot_error)?
                        .with_usage_fallback()
                };
                let identity = material
                    .identity()
                    .map_err(|_| AuthErrorCode::IdentityMismatch)?
                    .ok_or(AuthErrorCode::ReauthRequired)?;
                let actual: [u8; 32] = Sha256::digest(identity.as_bytes()).into();
                if actual != expected {
                    return Err(AuthErrorCode::IdentityMismatch);
                }
                Ok(Some(material))
            }
            UsageAccount::NativeLogin => {
                if owner.is_some() && !owns_default {
                    return Err(AuthErrorCode::Forbidden);
                }
                if !default_login_available(tx, scope.tenant, &state).map_err(slot_error)? {
                    return Err(AuthErrorCode::Unsupported);
                }
                self.read_default()
            }
        }
    }

    fn read_default(&self) -> Result<Option<LoginMaterial>, AuthErrorCode> {
        self.keychain.authorize_home(self.home).map_err(|_| AuthErrorCode::Forbidden)?;
        match self.home.join(".claude").symlink_metadata() {
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(_) => return Err(AuthErrorCode::StorageUnavailable),
        }
        let host = LoginHost::open(self.home).map_err(slot_error)?;
        if host.has_journal().map_err(slot_error)? {
            return Err(AuthErrorCode::Conflict);
        }
        host.material(self.keychain)
            .map(|material| Some(material.with_usage_fallback()))
            .map_err(slot_error)
    }
}

fn slot_error(error: WriteError) -> AuthErrorCode {
    match error {
        WriteError::Conflict | WriteError::Recovered => AuthErrorCode::Conflict,
        WriteError::InvalidResource => AuthErrorCode::InvalidInput,
        WriteError::Io => AuthErrorCode::StorageUnavailable,
    }
}

pub(super) async fn query_at(
    slots: LoginSlots<'_>,
    credential: &UsageCredentialRef<'_>,
    load_managed: impl FnOnce(
        AuthAccountScope<'_>,
        &str,
    ) -> Result<(CredentialLease, [u8; 32]), AuthErrorCode>,
    builder: ClientBuilder,
    origin: &str,
    now: TimestampMs,
    correlation_id: &str,
) -> Result<ClaudeUsage, AuthErrorCode> {
    use UsageCredentialState::*;
    use UsageQueryErrorCode::*;

    let (authorization, subscription_type, rate_limit_tier) = {
        let material = slots.read(credential, load_managed)?;
        let oauth = material.as_ref().and_then(LoginMaterial::usage_oauth);
        (
            bearer(oauth, now),
            oauth.and_then(|oauth| label(&oauth["subscriptionType"])),
            oauth.and_then(|oauth| label(&oauth["rateLimitTier"])),
        )
    };
    let (queried_at, state, result) = match authorization {
        Ok(bearer) => match builder.build().map_err(transport_error) {
            Ok(client) => {
                let request = client
                    .get(format!("{origin}/api/oauth/usage"))
                    .header(AUTHORIZATION, bearer)
                    .header("anthropic-beta", "oauth-2025-04-20")
                    .header(ACCEPT, "application/json");
                let result = match request.send().await {
                    Ok(response) => read_json_response::<Value>(response).await.and_then(|body| {
                        parse_usage(body, subscription_type.clone(), rate_limit_tier.clone())
                    }),
                    Err(error) => Err(transport_error(error)),
                };
                let state =
                    if matches!(result, Err(UpstreamUnauthorized)) { Expired } else { Valid };
                (Some(now), state, result)
            }
            Err(code) => (None, Valid, Err(code)),
        },
        Err(state) => (None, state, Err(CredentialUnavailable)),
    };
    let credential_message = match state {
        Valid => None,
        Expired => Some("Claude authorization has expired"),
        RefreshPending => Some("Claude authorization is awaiting refresh"),
        NotFound => Some("Claude login credentials are unavailable"),
        ParseError => Some("Claude login credentials are invalid"),
    }
    .map(str::to_owned);
    let outcome = match result {
        Ok(data) => UsageQueryOutcome::Success { data: UsageQueryData::OfficialSubscription(data) },
        Err(code) => UsageQueryOutcome::Failure {
            kind: UsageQueryKind::OfficialSubscription,
            error: UsageQueryError {
                code,
                message: credential_message
                    .clone()
                    .unwrap_or_else(|| "Claude quota query failed".into()),
                correlation_id: correlation_id.into(),
            },
        },
    };
    Ok(ClaudeUsage {
        quota: UsageQueryResult {
            source: UsageSource::Official("claude".into()),
            queried_at,
            credential_state: state,
            credential_message,
            outcome,
        },
        subscription_type,
        rate_limit_tier,
    })
}

fn bearer(oauth: Option<&Value>, now: TimestampMs) -> Result<HeaderValue, UsageCredentialState> {
    use UsageCredentialState::*;

    let oauth = oauth.ok_or(NotFound)?;
    let token = oauth["accessToken"]
        .as_str()
        .filter(|token| {
            !token.is_empty()
                && token.len() <= 16384
                && !token.chars().any(|c| c.is_whitespace() || c.is_control())
        })
        .ok_or(ParseError)?;
    if token_expiry(oauth.get("expiresAt"))?.is_some_and(|expiry| expiry <= now) {
        let refreshable = oauth["refreshToken"].as_str().is_some_and(|token| !token.is_empty())
            && token_expiry(oauth.get("refreshTokenExpiresAt"))?.is_none_or(|expiry| expiry > now);
        return Err(if refreshable { RefreshPending } else { Expired });
    }
    let mut bearer = crate::providers::validation::header_value(&format!("Bearer {token}"))
        .map_err(|_| ParseError)?;
    bearer.set_sensitive(true);
    Ok(bearer)
}

fn token_expiry(value: Option<&Value>) -> Result<Option<TimestampMs>, UsageCredentialState> {
    let value = value.filter(|value| !value.is_null());
    if let Some(text) = value.and_then(Value::as_str) {
        return DateTime::parse_from_rfc3339(text)
            .map(|time| time.timestamp_millis())
            .or_else(|_| {
                NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f")
                    .map(|time| time.and_utc().timestamp_millis())
            })
            .map(|time| Some(TimestampMs(time)))
            .map_err(|_| UsageCredentialState::ParseError);
    }
    expiry(value).map_err(|_| UsageCredentialState::ParseError)
}

const KNOWN_WINDOWS: [&str; 5] =
    ["five_hour", "seven_day", "seven_day_fable", "seven_day_opus", "seven_day_sonnet"];

fn parse_usage(
    body: Value,
    subscription_type: Option<String>,
    rate_limit_tier: Option<String>,
) -> Result<UsageSubscription, UsageQueryErrorCode> {
    let object = body.as_object().ok_or(UsageQueryErrorCode::InvalidResponse)?;
    let mut windows = Vec::new();
    for (name, data) in object {
        if matches!(name.as_str(), "extra_usage" | "limits") {
            continue;
        }
        if let Some(window) = quota_window(name, &data["utilization"], &data["resets_at"]) {
            windows.push(window);
        }
    }

    // The newer model-wide weekly limits override legacy entries with the same name.
    let mut scoped_names = std::collections::HashSet::new();
    for limits in body["limits"].as_array().into_iter().flatten() {
        if limits["kind"] != "weekly_scoped"
            || limits["group"] != "weekly"
            || !limits["scope"]["surface"].is_null()
        {
            continue;
        }
        let model = limits["scope"]["model"]["display_name"].as_str().unwrap_or("");
        let name = match model.trim().to_ascii_lowercase().as_str() {
            "fable" => "seven_day_fable",
            "opus" => "seven_day_opus",
            "sonnet" => "seven_day_sonnet",
            _ => continue,
        };
        if let Some(window) = quota_window(name, &limits["percent"], &limits["resets_at"])
            && scoped_names.insert(name)
        {
            if let Some(existing) = windows.iter_mut().find(|window| window.name == name) {
                *existing = window;
            } else {
                windows.push(window);
            }
        }
    }
    windows.sort_by_key(|window| {
        KNOWN_WINDOWS.iter().position(|name| *name == window.name).unwrap_or(KNOWN_WINDOWS.len())
    });

    let extra = &body["extra_usage"];
    let extra_usage = extra.is_object().then(|| UsageExtraUsage {
        is_enabled: extra["is_enabled"].as_bool().unwrap_or(false),
        monthly_limit: number(&extra["monthly_limit"]),
        used_credits: number(&extra["used_credits"]),
        utilization: number(&extra["utilization"]),
        currency: label(&extra["currency"]),
    });
    Ok(UsageSubscription {
        subscription_type,
        rate_limit_tier,
        windows,
        extra_usage,
        credits_balance: None,
        reset_credits: None,
    })
}

fn quota_window(name: &str, percent: &Value, reset: &Value) -> Option<UsageQuotaWindow> {
    let utilization = number(percent).filter(|n| f64::from(*n) >= 0.0)?;
    Some(UsageQuotaWindow {
        name: name.into(),
        plan_label: None,
        utilization: Some(utilization),
        used: None,
        total: None,
        remaining: None,
        unit: None,
        resets_at: reset
            .as_str()
            .filter(|s| DateTime::parse_from_rfc3339(s).is_ok())
            .map(str::to_owned),
    })
}

fn number(value: &Value) -> Option<UsageNumber> {
    UsageNumber::try_from(value.as_f64()?).ok()
}

fn label(value: &Value) -> Option<String> {
    value
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty() && s.len() <= 128 && !s.chars().any(char::is_control))
        .map(str::to_owned)
}
