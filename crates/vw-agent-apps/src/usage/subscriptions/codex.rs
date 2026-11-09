//! Official Codex quota for the authorized CLI slot or a managed ChatGPT account.

use std::{io::Read, path::Path, time::Duration};

use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::{
    Client, ClientBuilder, RequestBuilder,
    header::{ACCEPT, AUTHORIZATION, HeaderValue, USER_AGENT},
};
use security_framework::passwords::{PasswordOptions, generic_password};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use vw_api_types::{
    TimestampMs,
    agent_apps_auth::{AuthErrorCode, AuthService},
    app_providers::paths::ConfigPathState,
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryError, UsageQueryErrorCode,
        UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
        UsageSubscription,
    },
};

use crate::{
    auth::credential_file::{AccountCredential, CredentialFileStore},
    providers::{paths::open_beneath, validate_identity, validation::header_value},
    usage::{
        binding::{UsageAccount, UsageCredentialRef},
        coding::zhipu::{client_builder, read_json_response, transport_error},
    },
};

pub const CODEX_USAGE_ORIGIN: &str = "https://chatgpt.com";

/// The managed gateway query consumes a previously checked credential snapshot.
pub struct ManagedCodexUsage(Client);

impl ManagedCodexUsage {
    pub fn new(builder: ClientBuilder) -> Result<Self, AuthErrorCode> {
        builder
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .no_proxy()
            .timeout(Duration::from_secs(15))
            .build()
            .map(Self)
            .map_err(|_| AuthErrorCode::ExecutionHostUnavailable)
    }

    pub async fn query(
        &self,
        auth: &Value,
        now: TimestampMs,
        correlation_id: &str,
    ) -> UsageQueryResult {
        query_headers(
            "codex_oauth",
            headers(auth, true),
            Ok(self.0.clone()),
            CODEX_USAGE_ORIGIN,
            now,
            correlation_id,
        )
        .await
    }
}

const MAX_NATIVE_FILE_BYTES: u64 = 256 * 1024;

/// The caller must authorize the resolved account, the execution host's HOME
/// and access to chatgpt.com. Both requests use one credential snapshot.
pub async fn query_codex(
    credential: &UsageCredentialRef<'_>,
    home: &Path,
    managed_store: &CredentialFileStore,
    now: TimestampMs,
    correlation_id: &str,
) -> Result<UsageQueryResult, AuthErrorCode> {
    query_at(
        credential,
        home,
        managed_store,
        client_builder(),
        CODEX_USAGE_ORIGIN,
        now,
        correlation_id,
    )
    .await
}

pub(super) async fn query_at(
    credential: &UsageCredentialRef<'_>,
    home: &Path,
    managed_store: &CredentialFileStore,
    builder: ClientBuilder,
    origin: &str,
    now: TimestampMs,
    correlation_id: &str,
) -> Result<UsageQueryResult, AuthErrorCode> {
    use UsageCredentialState::{NotFound, ParseError};

    let scope = credential.scope();
    if scope.service != AuthService::CodexOauth {
        return Err(AuthErrorCode::Unsupported);
    }

    let (source, headers) = match credential.account() {
        UsageAccount::NativeLogin => {
            ("codex", read_native(home).and_then(|auth| headers(&auth, false)))
        }
        UsageAccount::Managed(id) => {
            let headers = match managed_store.read_managed_account(scope, id)? {
                Some(AccountCredential::CodexAuth(auth)) => headers(&auth, true),
                Some(_) => Err(ParseError),
                None => Err(NotFound),
            };
            ("codex_oauth", headers)
        }
    };

    Ok(query_headers(
        source,
        headers,
        builder.build().map_err(transport_error),
        origin,
        now,
        correlation_id,
    )
    .await)
}

type CodexHeaders = (HeaderValue, Option<HeaderValue>);

async fn query_headers(
    source: &str,
    headers: Result<CodexHeaders, UsageCredentialState>,
    client: Result<Client, UsageQueryErrorCode>,
    origin: &str,
    now: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    use UsageCredentialState::*;
    use UsageQueryErrorCode::*;
    let (queried_at, credential_state, result) = match headers {
        Ok((bearer, workspace)) => match client {
            Ok(client) => {
                let request = |path| {
                    let request = client
                        .get(format!("{origin}/backend-api/wham/{path}"))
                        .header(AUTHORIZATION, bearer.clone())
                        .header(USER_AGENT, "codex-cli")
                        .header(ACCEPT, "application/json");
                    match &workspace {
                        Some(workspace) => request.header("ChatGPT-Account-Id", workspace.clone()),
                        None => request,
                    }
                };
                let (usage, resets) = tokio::join!(
                    get_json::<UsageResponse>(request("usage")),
                    get_json::<ResetCreditsResponse>(
                        request("rate-limit-reset-credits")
                            .header("OpenAI-Beta", "codex-1")
                            .timeout(Duration::from_secs(8)),
                    ),
                );
                let state =
                    if matches!(usage, Err(UpstreamUnauthorized)) { Expired } else { Valid };
                (Some(now), state, usage.map(|body| subscription(body, resets.ok(), now)))
            }
            Err(code) => (None, Valid, Err(code)),
        },
        Err(state) => (None, state, Err(CredentialUnavailable)),
    };
    let credential_message = match credential_state {
        Expired => Some("Codex authorization has expired or was rejected"),
        NotFound => Some("Codex ChatGPT login credentials are unavailable"),
        ParseError => Some("Codex login credentials are invalid"),
        _ => None,
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
                    .unwrap_or_else(|| "Codex quota query failed".into()),
                correlation_id: correlation_id.into(),
            },
        },
    };
    UsageQueryResult {
        source: UsageSource::Official(source.into()),
        queried_at,
        credential_state,
        credential_message,
        outcome,
    }
}

fn read_native(home: &Path) -> Result<Value, UsageCredentialState> {
    use UsageCredentialState::{NotFound, ParseError};

    let directory = home.join(".codex");
    let config = read_file(home, &directory.join("config.toml"))?.unwrap_or_default();
    let config = std::str::from_utf8(&config).map_err(|_| ParseError)?;
    let config: toml::Table = config.parse().map_err(|_| ParseError)?;
    let bytes = match config.get("cli_auth_credentials_store").map(toml::Value::as_str) {
        None | Some(Some("file")) => read_file(home, &directory.join("auth.json"))?,
        Some(Some("keyring")) => read_keychain(home, &directory)?,
        Some(Some("auto")) => match read_keychain(home, &directory)? {
            Some(bytes) => Some(bytes),
            None => read_file(home, &directory.join("auth.json"))?,
        },
        Some(Some("ephemeral")) => return Err(NotFound),
        _ => return Err(ParseError),
    };
    serde_json::from_slice(&bytes.ok_or(NotFound)?).map_err(|_| ParseError)
}

fn read_file(home: &Path, path: &Path) -> Result<Option<Vec<u8>>, UsageCredentialState> {
    let file = match open_beneath(home, path, false) {
        Ok(file) => file,
        Err(ConfigPathState::Absent) => return Ok(None),
        Err(_) => return Err(UsageCredentialState::ParseError),
    };
    let mut bytes = Vec::new();
    file.take(MAX_NATIVE_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| UsageCredentialState::ParseError)?;
    if bytes.len() as u64 > MAX_NATIVE_FILE_BYTES {
        return Err(UsageCredentialState::ParseError);
    }
    Ok(Some(bytes))
}

fn read_keychain(home: &Path, directory: &Path) -> Result<Option<Vec<u8>>, UsageCredentialState> {
    use UsageCredentialState::ParseError;

    let actual_home = std::env::var_os("HOME").ok_or(ParseError)?;
    if home.canonicalize().map_err(|_| ParseError)?
        != Path::new(&actual_home).canonicalize().map_err(|_| ParseError)?
    {
        return Err(ParseError);
    }
    let directory = directory.canonicalize().unwrap_or_else(|_| directory.to_path_buf());
    let digest = Sha256::digest(directory.to_string_lossy().as_bytes());
    let prefix: String = digest[..8].iter().map(|byte| format!("{byte:02x}")).collect();
    let account = format!("cli|{prefix}");
    match generic_password(PasswordOptions::new_generic_password("Codex Auth", &account)) {
        Ok(bytes) => Ok(Some(bytes)),
        Err(error) if error.code() == -25300 => Ok(None), // errSecItemNotFound
        Err(_) => Err(ParseError),
    }
}

fn headers(
    auth: &Value,
    managed: bool,
) -> Result<(HeaderValue, Option<HeaderValue>), UsageCredentialState> {
    use UsageCredentialState::{NotFound, ParseError};

    let object = auth.as_object().ok_or(ParseError)?;
    match object.get("auth_mode").and_then(Value::as_str) {
        Some("chatgpt") => {}
        None if managed => {} // Legacy managed bundles omitted auth_mode.
        _ => return Err(NotFound),
    }
    let tokens = auth["tokens"].as_object().ok_or(ParseError)?;
    let token = tokens.get("access_token").and_then(Value::as_str).ok_or(ParseError)?;
    if token.is_empty() || token.len() > 16384 || !token.bytes().all(|byte| byte.is_ascii_graphic())
    {
        return Err(ParseError);
    }
    let mut bearer = header_value(&format!("Bearer {token}")).map_err(|_| ParseError)?;
    bearer.set_sensitive(true);
    let workspace = match tokens.get("account_id") {
        None | Some(Value::Null) if !managed => None,
        Some(Value::String(id)) => {
            validate_identity(id).map_err(|_| ParseError)?;
            let mut header = header_value(id).map_err(|_| ParseError)?;
            header.set_sensitive(true);
            Some(header)
        }
        _ => return Err(ParseError),
    };
    Ok((bearer, workspace))
}

async fn get_json<T: DeserializeOwned>(request: RequestBuilder) -> Result<T, UsageQueryErrorCode> {
    let response = request.send().await.map_err(transport_error)?;
    read_json_response(response).await
}

#[derive(Deserialize)]
struct UsageResponse {
    rate_limit: Option<RateLimit>,
    credits: Option<Value>,
}

#[derive(Deserialize)]
struct RateLimit {
    primary_window: Option<RateLimitWindow>,
    secondary_window: Option<RateLimitWindow>,
}

#[derive(Deserialize)]
struct RateLimitWindow {
    used_percent: Option<UsageNumber>,
    limit_window_seconds: Option<i64>,
    reset_at: Option<i64>,
}

#[derive(Deserialize)]
struct ResetCreditsResponse {
    #[serde(default)]
    credits: Vec<ResetCredit>,
}

#[derive(Deserialize)]
struct ResetCredit {
    status: Option<String>,
    expires_at: Option<String>,
}

fn subscription(
    body: UsageResponse,
    resets: Option<ResetCreditsResponse>,
    now: TimestampMs,
) -> UsageSubscription {
    let windows = body
        .rate_limit
        .into_iter()
        .flat_map(|limit| [limit.primary_window, limit.secondary_window])
        .flatten()
        .filter_map(|window| {
            let used = window.used_percent?;
            let resets_at = if f64::from(used) == 0.0 {
                None
            } else {
                window.reset_at.and_then(|seconds| DateTime::from_timestamp(seconds, 0)).map(iso)
            };
            Some(UsageQuotaWindow {
                name: window_name(window.limit_window_seconds),
                plan_label: None,
                utilization: Some(used),
                resets_at,
                used: None,
                total: None,
                remaining: None,
                unit: None,
            })
        })
        .collect();
    UsageSubscription {
        subscription_type: None,
        rate_limit_tier: None,
        windows,
        extra_usage: None,
        credits_balance: body.credits.as_ref().and_then(credits_balance),
        reset_credits: resets.map(|resets| reset_expiries(resets, now)),
    }
}

fn window_name(seconds: Option<i64>) -> String {
    match seconds {
        Some(18_000) => "five_hour".into(),
        Some(604_800) => "seven_day".into(),
        Some(2_592_000) => "30_day".into(),
        Some(seconds) if seconds >= 86_400 => format!("{}_day", seconds / 86_400),
        Some(seconds) if seconds > 0 => format!("{}_hour", seconds / 3_600),
        _ => "unknown".into(),
    }
}

fn credits_balance(credits: &Value) -> Option<UsageNumber> {
    if credits["has_credits"].as_bool() != Some(true)
        || credits["unlimited"].as_bool() == Some(true)
    {
        return None;
    }
    let balance = match &credits["balance"] {
        Value::String(value) => value.trim().parse::<f64>().ok()?,
        value => value.as_f64()?,
    };
    UsageNumber::try_from(balance).ok().filter(|_| balance > 0.0)
}

fn reset_expiries(resets: ResetCreditsResponse, now: TimestampMs) -> Vec<Option<String>> {
    let mut expiries: Vec<_> = resets
        .credits
        .into_iter()
        .filter(|credit| credit.status.as_deref() == Some("available"))
        .map(|credit| {
            credit
                .expires_at
                .as_deref()
                .and_then(|raw| DateTime::parse_from_rfc3339(raw).ok())
                .map(|at| at.to_utc())
        })
        .filter(|at| at.is_none_or(|at| at.timestamp_millis() > now.0))
        .collect();
    expiries.sort_unstable_by_key(|at| (at.is_none(), *at));
    expiries.into_iter().map(|at| at.map(iso)).collect()
}

fn iso(at: DateTime<Utc>) -> String {
    at.to_rfc3339_opts(SecondsFormat::AutoSi, true)
}
