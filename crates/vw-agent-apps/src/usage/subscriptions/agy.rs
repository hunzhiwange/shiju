//! Official Antigravity quota for an explicit managed account or the native CLI login.

use std::{collections::BTreeMap, future::Future, io, process::Stdio, time::Duration};

use chrono::DateTime;
use reqwest::{Client, RequestBuilder, StatusCode};
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
    time::timeout,
};
use vw_api_types::{
    TimestampMs,
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
    apps_usage::{
        UsageCredentialState, UsageNumber, UsageQueryData, UsageQueryError, UsageQueryErrorCode,
        UsageQueryKind, UsageQueryOutcome, UsageQueryResult, UsageQuotaWindow, UsageSource,
        UsageSubscription,
    },
};

use crate::{
    apps::{
        discovery::read::{LocalReadEnvironment, locate_cli},
        launch::{host, request::LaunchMaterial},
    },
    auth::{
        agy_account::{AgyNativeTarget, lock_native_usage, query_access_token},
        agy_refresh::refresh_account,
        flow::AuthAccountScope,
    },
    usage::coding::zhipu::{client_builder, read_json_response, transport_error},
};

const ORIGIN: &str = "https://cloudcode-pa.googleapis.com";
const FALLBACK_CLI_VERSION: &str = "1.4.6";
const CLI_TIMEOUT: Duration = Duration::from_secs(30);
const CLI_OUTPUT_LIMIT: usize = 256 * 1024;

type QueryFailure = (UsageCredentialState, UsageQueryErrorCode, &'static str);

/// Query one managed account from the authorized account scope. For a provider,
/// the caller resolves its saved binding first. No native slot is accessed.
/// The caller authorizes Google egress and may supply the discovered CLI version.
pub async fn query_agy_account(
    scope: AuthAccountScope<'_>,
    account_id: &str,
    cli_version: Option<&semver::Version>,
    now: TimestampMs,
    correlation_id: &str,
) -> Result<UsageQueryResult, AuthErrorCode> {
    query_account_at(
        scope,
        account_id,
        ORIGIN,
        cli_version,
        now,
        correlation_id,
        |scope, account_id, refresh| async move {
            if !refresh && let Some(token) = query_access_token(scope, account_id, now)? {
                return Ok(token);
            }
            refresh_account(scope, account_id).await
        },
    )
    .await
}

/// Query the host's current CLI login. Slot ownership must come from host policy.
/// Keep the native slot locked during the query so managed checkout cannot switch it.
pub async fn query_agy_cli(
    scope: AuthAccountScope<'_>,
    exclusive_owner: Option<AuthAccountScope<'_>>,
    host: &LocalReadEnvironment,
    now: TimestampMs,
    correlation_id: &str,
) -> Result<UsageQueryResult, AuthErrorCode> {
    if host.host_id != scope.host {
        return Err(AuthErrorCode::Forbidden);
    }
    let _lock = lock_native_usage(scope, AgyNativeTarget { home: &host.home, exclusive_owner })?;
    Ok(query_cli_at(host, now, correlation_id).await)
}

pub(super) async fn query_account_at<'a, F>(
    scope: AuthAccountScope<'a>,
    account_id: &'a str,
    origin: &str,
    cli_version: Option<&semver::Version>,
    now: TimestampMs,
    correlation_id: &str,
    mut access_token: impl FnMut(AuthAccountScope<'a>, &'a str, bool) -> F,
) -> Result<UsageQueryResult, AuthErrorCode>
where
    F: Future<Output = Result<String, AuthErrorCode>>,
{
    use UsageQueryErrorCode::UpstreamUnauthorized;

    scope.validate()?;
    if scope.service != AuthService::AgyOauth {
        return Err(AuthErrorCode::Unsupported);
    }
    let version = cli_version.map(ToString::to_string);
    let user_agent = format!(
        "ShiJu/{} (antigravity/cli/{})",
        env!("CARGO_PKG_VERSION"),
        version.as_deref().unwrap_or(FALLBACK_CLI_VERSION),
    );
    let mut queried_at = None;
    let mut token = access_token(scope, account_id, false).await;
    let result = match client_builder().timeout(Duration::from_secs(20)).build() {
        Err(error) => Err(query_error(transport_error(error))),
        Ok(client) => loop {
            let bearer = match token {
                Ok(ref bearer) => bearer,
                Err(AuthErrorCode::NotFound) => {
                    break Err((
                        UsageCredentialState::NotFound,
                        UsageQueryErrorCode::CredentialUnavailable,
                        "Antigravity account is unavailable",
                    ));
                }
                Err(AuthErrorCode::ReauthRequired) => {
                    break Err((
                        UsageCredentialState::Expired,
                        UsageQueryErrorCode::CredentialUnavailable,
                        "Antigravity sign-in expired",
                    ));
                }
                Err(AuthErrorCode::UpstreamFailure) => {
                    break Err((
                        UsageCredentialState::RefreshPending,
                        UsageQueryErrorCode::UpstreamUnavailable,
                        "Antigravity token refresh could not complete",
                    ));
                }
                Err(error) => return Err(error),
            };
            let already_queried = queried_at.replace(now).is_some();
            let result = query_google(&client, origin, &user_agent, bearer).await;
            if !already_queried && matches!(result, Err((_, UpstreamUnauthorized, _))) {
                token = access_token(scope, account_id, true).await;
                continue;
            }
            break result;
        },
    };
    Ok(query_result("agy_oauth", queried_at, result, correlation_id))
}

async fn query_google(
    client: &Client,
    origin: &str,
    user_agent: &str,
    bearer: &str,
) -> Result<Vec<UsageQuotaWindow>, QueryFailure> {
    let request = |method| {
        client
            .post(format!("{origin}/v1internal:{method}"))
            .bearer_auth(bearer)
            .header(reqwest::header::USER_AGENT, user_agent)
    };
    let response = request("retrieveUserQuotaSummary")
        .json(&json!({}))
        .send()
        .await
        .map_err(|error| query_error(transport_error(error)))?;
    if response.status() == StatusCode::FORBIDDEN {
        let metadata = json!({"metadata": {"ideType": "ANTIGRAVITY"}});
        let ineligible = read_eligibility(request("loadCodeAssist").json(&metadata)).await;
        return Err((
            UsageCredentialState::Valid,
            UsageQueryErrorCode::UpstreamRejected,
            if ineligible {
                "Antigravity account is not eligible"
            } else {
                "Google denied Antigravity quota access (403)"
            },
        ));
    }
    let groups: QuotaGroups = read_json_response(response).await.map_err(query_error)?;
    windows(groups).map_err(query_error)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Eligibility {
    current_tier: Option<Value>,
    #[serde(default)]
    ineligible_tiers: Vec<Value>,
}

async fn read_eligibility(request: RequestBuilder) -> bool {
    let Ok(response) = request.send().await else { return false };
    read_json_response::<Eligibility>(response)
        .await
        .is_ok_and(|result| result.current_tier.is_none() && !result.ineligible_tiers.is_empty())
}

pub(super) async fn query_cli_at(
    env: &LocalReadEnvironment,
    now: TimestampMs,
    correlation_id: &str,
) -> UsageQueryResult {
    use UsageQueryErrorCode::{
        CredentialUnavailable, InvalidResponse, Timeout, Transport, UpstreamRejected,
    };

    let mut queried_at = None;
    let result = async {
        let program = locate_cli(AppId::Agy, env).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                (
                    UsageCredentialState::NotFound,
                    CredentialUnavailable,
                    "Antigravity CLI is unavailable",
                )
            } else {
                query_error(Transport)
            }
        })?;
        let home = env.home.canonicalize().map_err(|_| query_error(Transport))?;
        let material = LaunchMaterial {
            program,
            cwd: home.clone(),
            args: ["--print", "/usage", "--output-format", "json"].map(str::to_owned).into(),
            skip_permissions: false,
        };
        let command = host::command(material, &home, &env.search_paths)
            .map_err(|_| query_error(Transport))?;
        queried_at = Some(now);
        let bytes = timeout(CLI_TIMEOUT, cli_output(command))
            .await
            .map_err(|_| query_error(Timeout))?
            .map_err(query_error)?;
        let response: CliUsage =
            serde_json::from_slice(&bytes).map_err(|_| query_error(InvalidResponse))?;
        if response.status.as_deref() != Some("SUCCESS") {
            return Err(query_error(UpstreamRejected));
        }
        let groups = response
            .command
            .and_then(|command| command.data)
            .ok_or_else(|| query_error(InvalidResponse))?;
        windows(groups).map_err(query_error)
    }
    .await;
    query_result("agy", queried_at, result, correlation_id)
}

struct CliChild(Child);

impl Drop for CliChild {
    fn drop(&mut self) {
        if let Some(pid) = self.0.id() {
            // SAFETY: this child owns the process group created by process_group(0).
            unsafe { libc::kill(-(pid as i32), libc::SIGKILL) };
        }
    }
}

async fn cli_output(command: std::process::Command) -> Result<Vec<u8>, UsageQueryErrorCode> {
    use UsageQueryErrorCode::{InvalidResponse, Transport, UpstreamRejected};

    let mut child = CliChild(
        Command::from(command)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .stdout(Stdio::piped())
            .process_group(0)
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| Transport)?,
    );
    let stdout = child.0.stdout.take().ok_or(Transport)?;
    let mut bytes = Vec::new();
    stdout
        .take((CLI_OUTPUT_LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|_| Transport)?;
    if bytes.len() > CLI_OUTPUT_LIMIT {
        return Err(InvalidResponse);
    }
    if !child.0.wait().await.map_err(|_| Transport)?.success() {
        return Err(UpstreamRejected);
    }
    Ok(bytes)
}

#[derive(Deserialize)]
struct CliUsage {
    status: Option<String>,
    command: Option<CliCommand>,
}

#[derive(Deserialize)]
struct CliCommand {
    data: Option<QuotaGroups>,
}

#[derive(Deserialize)]
struct QuotaGroups {
    #[serde(default)]
    groups: Vec<QuotaGroup>,
}

#[derive(Deserialize)]
struct QuotaGroup {
    #[serde(default)]
    buckets: Vec<QuotaBucket>,
}

#[derive(Deserialize)]
struct QuotaBucket {
    #[serde(alias = "bucketId")]
    id: Option<String>,
    #[serde(alias = "remainingFraction")]
    remaining_fraction: Option<f64>,
    #[serde(alias = "resetTime")]
    reset_time: Option<String>,
}

fn windows(groups: QuotaGroups) -> Result<Vec<UsageQuotaWindow>, UsageQueryErrorCode> {
    use UsageQueryErrorCode::InvalidResponse;

    let mut windows = BTreeMap::<_, (f64, Option<String>)>::new();
    for bucket in groups.groups.into_iter().flat_map(|group| group.buckets) {
        let key = match bucket.id.as_deref() {
            Some("gemini-5h") => (0, "agy_gemini_five_hour"),
            Some("gemini-weekly") => (1, "agy_gemini_weekly"),
            Some("3p-5h") => (2, "agy_third_party_five_hour"),
            Some("3p-weekly") => (3, "agy_third_party_weekly"),
            _ => continue,
        };
        let Some(remaining) = bucket.remaining_fraction else { continue };
        let utilization = (1.0 - remaining.clamp(0.0, 1.0)) * 100.0;
        let reset = bucket.reset_time;
        if reset.as_deref().is_some_and(|time| DateTime::parse_from_rfc3339(time).is_err()) {
            return Err(InvalidResponse);
        }
        let entry = windows.entry(key).or_insert_with(|| (utilization, reset.clone()));
        if utilization > entry.0 || (utilization == entry.0 && entry.1.is_none()) {
            *entry = (utilization, reset);
        }
    }
    if windows.is_empty() {
        return Err(InvalidResponse);
    }
    Ok(windows
        .into_iter()
        .map(|((_, name), (utilization, resets_at))| UsageQuotaWindow {
            name: name.into(),
            plan_label: None,
            utilization: UsageNumber::try_from(utilization).ok(),
            used: None,
            total: None,
            remaining: None,
            unit: None,
            resets_at,
        })
        .collect())
}

fn query_error(code: UsageQueryErrorCode) -> QueryFailure {
    use UsageQueryErrorCode::*;

    match code {
        UpstreamUnauthorized => {
            (UsageCredentialState::Expired, code, "Google rejected Antigravity sign-in (401)")
        }
        UpstreamRejected => {
            (UsageCredentialState::Valid, code, "Antigravity quota query was rejected")
        }
        InvalidResponse => {
            (UsageCredentialState::Valid, code, "Invalid Antigravity quota response")
        }
        _ => (UsageCredentialState::Valid, code, "Antigravity quota query failed"),
    }
}

fn query_result(
    source: &str,
    queried_at: Option<TimestampMs>,
    result: Result<Vec<UsageQuotaWindow>, QueryFailure>,
    correlation_id: &str,
) -> UsageQueryResult {
    use UsageCredentialState::Valid;

    let (credential_state, credential_message, outcome) = match result {
        Ok(windows) => (
            Valid,
            None,
            UsageQueryOutcome::Success {
                data: UsageQueryData::OfficialSubscription(UsageSubscription {
                    subscription_type: None,
                    rate_limit_tier: None,
                    windows,
                    extra_usage: None,
                    credits_balance: None,
                    reset_credits: None,
                }),
            },
        ),
        Err((state, code, message)) => (
            state,
            (state != Valid).then(|| message.to_owned()),
            UsageQueryOutcome::Failure {
                kind: UsageQueryKind::OfficialSubscription,
                error: UsageQueryError {
                    code,
                    message: message.into(),
                    correlation_id: correlation_id.into(),
                },
            },
        ),
    };
    UsageQueryResult {
        source: UsageSource::Official(source.into()),
        queried_at,
        credential_state,
        credential_message,
        outcome,
    }
}
