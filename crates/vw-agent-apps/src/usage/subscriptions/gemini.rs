//! Official Gemini Code Assist quota for the authorized native CLI login.

use std::{collections::BTreeMap, io::Read, path::Path};

use chrono::DateTime;
use reqwest::{Client, ClientBuilder, RequestBuilder, header::HeaderValue};
use security_framework::passwords::{PasswordOptions, generic_password};
use serde::{Deserialize, de::DeserializeOwned};
use serde_json::json;
use vw_api_types::{
    TimestampMs,
    app_providers::paths::ConfigPathState,
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryError, UsageQueryErrorCode,
        UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
        UsageSubscription,
    },
};

use crate::{
    providers::{paths::open_beneath, validation::header_value},
    usage::coding::zhipu::{client_builder, read_json_response, transport_error},
};

const MAX_CREDENTIAL_BYTES: u64 = 256 * 1024;
const CODE_ASSIST_ORIGIN: &str = "https://cloudcode-pa.googleapis.com";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

pub(super) struct GeminiOAuth<'a> {
    pub token_url: &'a str,
    pub client_credentials: Option<(&'a str, &'a str)>,
}

/// Query the native login for an official Gemini provider. The caller must
/// authorize use of the host's exclusively owned CLI slot, its HOME and
/// egress to cloudcode-pa.googleapis.com and oauth2.googleapis.com.
pub async fn query_gemini(home: &Path, now: TimestampMs, correlation_id: &str) -> UsageQueryResult {
    let keychain_home = std::env::var_os("HOME");
    let client_id = std::env::var("SHIJU_GEMINI_OAUTH_CLIENT_ID").ok();
    let client_secret = std::env::var("SHIJU_GEMINI_OAUTH_CLIENT_SECRET").ok();
    query_at(
        home,
        keychain_home.as_deref().map(Path::new),
        client_builder(),
        CODE_ASSIST_ORIGIN,
        GeminiOAuth {
            token_url: TOKEN_URL,
            client_credentials: client_id.as_deref().zip(client_secret.as_deref()),
        },
        now,
        correlation_id,
    )
    .await
}

pub(super) async fn query_at(
    home: &Path,
    keychain_home: Option<&Path>,
    builder: ClientBuilder,
    quota_origin: &str,
    oauth: GeminiOAuth<'_>,
    now: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    use UsageCredentialState::*;
    use UsageQueryErrorCode::*;

    let (queried_at, state, result) = match NativeGeminiAccount::read(home, keychain_home) {
        Err(state) => (None, state, Err(CredentialUnavailable)),
        Ok(account) => match builder.build().map_err(transport_error) {
            Err(code) => (None, Valid, Err(code)),
            Ok(client) => match account.access_token(&client, oauth, now).await {
                Err((state, code)) => (None, state, Err(code)),
                Ok(bearer) => {
                    let result = quota(&client, quota_origin, &bearer).await;
                    let state =
                        if matches!(result, Err(UpstreamUnauthorized)) { Expired } else { Valid };
                    (Some(now), state, result)
                }
            },
        },
    };
    let credential_message = match state {
        NotFound => Some("Gemini CLI OAuth login is unavailable"),
        ParseError => Some("Gemini CLI OAuth credentials are invalid"),
        Expired => Some("Gemini CLI OAuth authorization has expired or was rejected"),
        RefreshPending if matches!(result, Err(MissingConfiguration)) => Some(
            "Gemini CLI OAuth refresh requires SHIJU_GEMINI_OAUTH_CLIENT_ID and SHIJU_GEMINI_OAUTH_CLIENT_SECRET",
        ),
        RefreshPending => Some("Gemini CLI OAuth refresh could not complete"),
        Valid => None,
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
                    .unwrap_or_else(|| "Gemini quota query failed".into()),
                correlation_id: correlation_id.into(),
            },
        },
    };
    UsageQueryResult {
        source: UsageSource::Official("gemini".into()),
        queried_at,
        credential_state: state,
        credential_message,
        outcome,
    }
}

#[derive(Deserialize)]
#[serde(untagged)]
enum CredentialFile {
    Keychain { token: NativeGeminiAccount },
    Flat(NativeGeminiAccount),
}

#[derive(Deserialize)]
struct NativeGeminiAccount {
    #[serde(alias = "accessToken")]
    access_token: String,
    #[serde(alias = "refreshToken")]
    refresh_token: Option<String>,
    #[serde(alias = "expiresAt")]
    expiry_date: Option<i64>,
}

#[derive(Deserialize)]
struct RefreshedToken {
    access_token: String,
}

impl NativeGeminiAccount {
    fn read(home: &Path, keychain_home: Option<&Path>) -> Result<Self, UsageCredentialState> {
        use UsageCredentialState::{NotFound, ParseError};

        if let Some(keychain_home) = keychain_home
            && let Ok(keychain_home) = keychain_home.canonicalize()
            && home.canonicalize().is_ok_and(|path| path == keychain_home)
        {
            match generic_password(PasswordOptions::new_generic_password(
                "gemini-cli-oauth",
                "main-account",
            )) {
                Ok(bytes) => return Self::parse(&bytes),
                Err(error) if error.code() == -25300 => {} // errSecItemNotFound
                Err(_) => return Err(ParseError),
            }
        }

        let path = home.join(".gemini/oauth_creds.json");
        let file = match open_beneath(home, &path, false) {
            Ok(file) => file,
            Err(ConfigPathState::Absent) => return Err(NotFound),
            Err(_) => return Err(ParseError),
        };
        let mut bytes = Vec::new();
        file.take(MAX_CREDENTIAL_BYTES + 1).read_to_end(&mut bytes).map_err(|_| ParseError)?;
        Self::parse(&bytes)
    }

    fn parse(bytes: &[u8]) -> Result<Self, UsageCredentialState> {
        use UsageCredentialState::ParseError;

        if bytes.len() as u64 > MAX_CREDENTIAL_BYTES {
            return Err(ParseError);
        }
        let credentials: CredentialFile = serde_json::from_slice(bytes).map_err(|_| ParseError)?;
        let (CredentialFile::Keychain { token: account } | CredentialFile::Flat(account)) =
            credentials;
        if !valid_token(&account.access_token)
            || account.refresh_token.as_deref().is_some_and(|token| !valid_token(token))
        {
            return Err(ParseError);
        }
        Ok(account)
    }

    /// Refresh only for this query. The CLI owns writes to the login slot.
    async fn access_token(
        self,
        client: &Client,
        oauth: GeminiOAuth<'_>,
        now: TimestampMs,
    ) -> Result<HeaderValue, (UsageCredentialState, UsageQueryErrorCode)> {
        use UsageCredentialState::{Expired, ParseError, RefreshPending};
        use UsageQueryErrorCode::*;

        if self.expiry_date.is_none_or(|expiry| expiry > now.0) {
            return bearer(&self.access_token).map_err(|_| (ParseError, CredentialUnavailable));
        }

        let refresh_token = self.refresh_token.ok_or((Expired, CredentialUnavailable))?;
        let (client_id, client_secret) = oauth
            .client_credentials
            .filter(|(id, secret)| valid_token(id) && valid_token(secret))
            .ok_or((RefreshPending, MissingConfiguration))?;
        let request = client.post(oauth.token_url).form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("refresh_token", &refresh_token),
        ]);
        let refreshed: RefreshedToken = post_json(request).await.map_err(|code| {
            let state = match code {
                Timeout | Transport | UpstreamRateLimited | UpstreamUnavailable => RefreshPending,
                _ => Expired,
            };
            (state, code)
        })?;
        bearer(&refreshed.access_token).map_err(|_| (Expired, InvalidResponse))
    }
}

fn valid_token(token: &str) -> bool {
    !token.is_empty() && token.len() <= 16_384 && token.bytes().all(|byte| byte.is_ascii_graphic())
}

fn bearer(token: &str) -> Result<HeaderValue, ()> {
    if !valid_token(token) {
        return Err(());
    }
    let mut value = header_value(&format!("Bearer {token}")).map_err(|_| ())?;
    value.set_sensitive(true);
    Ok(value)
}

async fn post_json<T: DeserializeOwned>(request: RequestBuilder) -> Result<T, UsageQueryErrorCode> {
    let response = request.send().await.map_err(transport_error)?;
    read_json_response(response).await
}

#[derive(Deserialize)]
struct ProjectResponse {
    #[serde(rename = "cloudaicompanionProject")]
    project: Option<Project>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum Project {
    Name(String),
    Object {
        #[serde(alias = "projectId")]
        id: String,
    },
}

#[derive(Deserialize)]
struct QuotaResponse {
    buckets: Vec<Bucket>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bucket {
    model_id: Option<String>,
    remaining_fraction: Option<f64>,
    reset_time: Option<String>,
}

async fn quota(
    client: &Client,
    origin: &str,
    bearer: &HeaderValue,
) -> Result<UsageSubscription, UsageQueryErrorCode> {
    let request = |method| {
        client
            .post(format!("{origin}/v1internal:{method}"))
            .header(reqwest::header::AUTHORIZATION, bearer.clone())
    };
    let project: ProjectResponse = post_json(request("loadCodeAssist").json(&json!({
        "metadata": {"ideType": "GEMINI_CLI", "pluginType": "GEMINI"}
    })))
    .await?;
    let body = match project.project {
        Some(Project::Name(id) | Project::Object { id }) => json!({"project": id}),
        None => json!({}),
    };
    let response: QuotaResponse = post_json(request("retrieveUserQuota").json(&body)).await?;
    Ok(UsageSubscription {
        subscription_type: None,
        rate_limit_tier: None,
        windows: quota_windows(response.buckets)?,
        extra_usage: None,
        credits_balance: None,
        reset_credits: None,
    })
}

fn quota_windows(buckets: Vec<Bucket>) -> Result<Vec<UsageQuotaWindow>, UsageQueryErrorCode> {
    use UsageQueryErrorCode::InvalidResponse;

    let mut tiers = BTreeMap::<_, (Option<f64>, Option<String>)>::new();
    for bucket in buckets {
        let model = bucket.model_id.as_deref().unwrap_or("unknown");
        if model.is_empty()
            || model.len() > 128
            || !model.bytes().all(|byte| byte.is_ascii_alphanumeric() || b"-_./".contains(&byte))
        {
            return Err(InvalidResponse);
        }
        let key = if model.contains("flash-lite") {
            (2, "gemini_flash_lite".to_owned())
        } else if model.contains("flash") {
            (1, "gemini_flash".to_owned())
        } else if model.contains("pro") {
            (0, "gemini_pro".to_owned())
        } else {
            (3, model.to_owned())
        };
        let used =
            bucket.remaining_fraction.map(|remaining| (1.0 - remaining.clamp(0.0, 1.0)) * 100.0);
        let reset = bucket.reset_time;
        if reset.as_deref().is_some_and(|time| DateTime::parse_from_rfc3339(time).is_err()) {
            return Err(InvalidResponse);
        }
        let entry = tiers.entry(key).or_insert_with(|| (used, reset.clone()));
        if used > entry.0 || (used == entry.0 && entry.1.is_none()) {
            *entry = (used, reset);
        }
    }
    Ok(tiers
        .into_iter()
        .map(|((_, name), (used, resets_at))| UsageQuotaWindow {
            name,
            plan_label: None,
            utilization: used.and_then(|value| UsageNumber::try_from(value).ok()),
            used: None,
            total: None,
            remaining: None,
            unit: None,
            resets_at,
        })
        .collect())
}
