//! Header-only probe for a provider selected by the diagnostic authorization boundary.

use std::time::Duration;

use reqwest::{Client, Url, header};
use serde_json::Value;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use vw_agent_apps::{
    auth::{copilot::CopilotAuth, flow::AuthAccountScope},
    providers,
};
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_auth::{AuthErrorCode, AuthService},
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource, DiagnosticFailure, DiagnosticFailureCode, DiagnosticOutcome,
        DiagnosticPhase, DiagnosticReport, DiagnosticResult, DiagnosticTiming,
        ReachabilityEvidence,
    },
    id::ProviderId,
};

use super::super::agent_apps_diagnostic_jobs::{DiagnosticJobOwner, DiagnosticProbeError};
use crate::{
    agent_apps::auth_accounts::AuthAccountStore, app_providers::AppProviderStore,
    global_database::GlobalDatabase, model_relay::upstream::upstream_http_client_builder,
};

const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(8);
const SLOW_THRESHOLD_MS: u64 = 6000;

#[derive(Debug, PartialEq, Eq)]
pub enum ReachabilityPrepareError {
    NotFound,
    Unavailable,
    Unsupported,
    Account(AuthErrorCode),
}

pub struct ReachabilityProbe {
    provider_id: ProviderId,
    endpoint: Endpoint,
    user_agent: Option<header::HeaderValue>,
}

enum Endpoint {
    Configured(Url),
    Copilot { owner: DiagnosticJobOwner, account_id: String },
}

impl ReachabilityProbe {
    /// Use the verified owner and provider selection on the execution host.
    /// Call on a blocking thread. Neither the request nor the account can supply a URL override.
    pub fn prepare(
        database: &GlobalDatabase,
        accounts: &AuthAccountStore,
        owner: DiagnosticJobOwner,
        provider_id: ProviderId,
    ) -> Result<Self, ReachabilityPrepareError> {
        use ReachabilityPrepareError::*;
        let row = AppProviderStore::preview_record(
            database,
            owner.scope.project.tenant_scope(),
            owner.scope.app_id,
            provider_id.as_ref(),
        )
        .map_err(|error| match error.as_str() {
            "application provider not found" => NotFound,
            "application provider storage unavailable" => Unavailable,
            _ => Unsupported,
        })?;
        let configured = configured_url(&owner, provider_id.as_ref(), &row);
        let endpoint = if row.pointer("/meta/isFullUrl").and_then(Value::as_bool) != Some(true)
            && is_copilot(&row, configured.as_deref())
        {
            let scope = account_scope(&owner);
            let account_id = match copilot_binding(&row)? {
                Some(id) => id.to_owned(),
                None => accounts
                    .managed_default_account(scope)
                    .map_err(Account)?
                    .ok_or(Account(AuthErrorCode::NotFound))?,
            };
            // Pin the selected account at submission. The account service revalidates it
            // when the worker resolves its current session endpoint.
            accounts.read_metadata(&scope, &account_id).map_err(Account)?;
            Endpoint::Copilot { owner, account_id }
        } else {
            let configured = configured.as_deref().ok_or(Unsupported)?;
            Endpoint::Configured(parse_endpoint(configured).ok_or(Unsupported)?)
        };
        let user_agent = row
            .pointer("/meta/customUserAgent")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .and_then(|value| header::HeaderValue::from_str(value).ok());
        Ok(Self { provider_id, endpoint, user_agent })
    }

    /// Run inside the diagnostic job's execution permit and cancellation scope.
    pub async fn run(
        self,
        accounts: &AuthAccountStore,
        copilot: &CopilotAuth,
        cancel: CancellationToken,
    ) -> Result<DiagnosticResult, DiagnosticProbeError> {
        if cancel.is_cancelled() {
            return Err(DiagnosticProbeError::Cancelled);
        }
        let endpoint = match self.endpoint {
            Endpoint::Configured(url) => url,
            Endpoint::Copilot { owner, account_id } => {
                let session = tokio::select! {
                    biased;
                    () = cancel.cancelled() => return Err(DiagnosticProbeError::Cancelled),
                    result = accounts.copilot_session(copilot, account_scope(&owner), &account_id) => result,
                }
                .map_err(|error| DiagnosticProbeError::Failed(DiagnosticFailure {
                    phase: DiagnosticPhase::Connect,
                    code: match error {
                        AuthErrorCode::NotFound
                        | AuthErrorCode::Unauthenticated
                        | AuthErrorCode::Forbidden
                        | AuthErrorCode::ReauthRequired
                        | AuthErrorCode::AuthorizationDenied
                        | AuthErrorCode::IdentityMismatch => DiagnosticFailureCode::Unauthorized,
                        AuthErrorCode::Unsupported => DiagnosticFailureCode::Unsupported,
                        _ => DiagnosticFailureCode::Transport,
                    },
                }))?;
                session.endpoint
            }
        };
        let client = upstream_http_client_builder()
            .retry(reqwest::retry::never())
            .build()
            .map_err(|_| {
                DiagnosticProbeError::Failed(DiagnosticFailure {
                    phase: DiagnosticPhase::Connect,
                    code: DiagnosticFailureCode::Transport,
                })
            })?;
        probe(
            &client,
            self.provider_id,
            endpoint,
            self.user_agent,
            DiagnosticEvidenceSource::RealService,
            ATTEMPT_TIMEOUT,
            cancel,
        )
        .await
    }
}

fn account_scope(owner: &DiagnosticJobOwner) -> AuthAccountScope<'_> {
    AuthAccountScope {
        tenant: owner.scope.project.tenant_scope(),
        principal: &owner.principal,
        host: &owner.scope.target_host_id,
        service: AuthService::GithubCopilot,
    }
}

fn is_copilot(row: &Value, configured: Option<&str>) -> bool {
    row.pointer("/meta/providerType").and_then(Value::as_str) == Some("github_copilot")
        || row.pointer("/meta/authBinding/authProvider").and_then(Value::as_str)
            == Some("github_copilot")
        || configured.and_then(parse_endpoint).is_some_and(|url| {
            url.host_str().is_some_and(|host| {
                host == "githubcopilot.com" || host.ends_with(".githubcopilot.com")
            })
        })
}

fn copilot_binding(row: &Value) -> Result<Option<&str>, ReachabilityPrepareError> {
    let invalid = ReachabilityPrepareError::Account(AuthErrorCode::InvalidInput);
    let account = match row.pointer("/meta/authBinding").filter(|value| !value.is_null()) {
        Some(binding) => {
            if binding["source"] != "managed_account" || binding["authProvider"] != "github_copilot"
            {
                return Err(invalid);
            }
            binding.get("accountId")
        }
        None => row.pointer("/meta/githubAccountId"),
    };
    match account {
        None | Some(Value::Null) => Ok(None),
        Some(id) => id.as_str().map(Some).ok_or(invalid),
    }
}

fn configured_url(owner: &DiagnosticJobOwner, _provider_id: &str, row: &Value) -> Option<String> {
    if row["category"] == "official" {
        return None;
    }
    let settings = &row["settingsConfig"];
    let native = match owner.scope.app_id {
        AppId::Claude | AppId::ClaudeDesktop => settings.pointer("/env/ANTHROPIC_BASE_URL"),
        AppId::Gemini | AppId::Agy => settings.pointer("/env/GOOGLE_GEMINI_BASE_URL"),
        AppId::OpenCode => {
            settings.pointer("/options/baseURL").or_else(|| settings.pointer("/settings/baseURL"))
        }
        AppId::OpenClaw => settings.get("baseUrl"),
        AppId::Pi | AppId::Omp => settings
            .get("baseUrl")
            .filter(|url| url.as_str().is_some_and(|url| !url.is_empty()))
            .or_else(|| {
                settings.get("models")?.as_array()?.iter().find_map(|model| {
                    model
                        .get("baseUrl")
                        .filter(|url| url.as_str().is_some_and(|url| !url.is_empty()))
                })
            }),
        AppId::Hermes => settings.get("base_url").or_else(|| settings.get("baseUrl")),
        #[cfg(target_os = "macos")]
        AppId::Codex if settings.get("config").is_some() => {
            use vw_agent_apps::providers::codex_catalog;
            let profile = codex_catalog::form::editable_profile(_provider_id, row)?;
            return codex_catalog::identity(&owner.scope.target_host_id, _provider_id, &profile)
                .ok()
                .map(|identity| identity.endpoint);
        }
        _ => None,
    };
    native.or_else(|| row.get("baseUrl")).and_then(Value::as_str).map(str::to_owned)
}

fn parse_endpoint(raw: &str) -> Option<Url> {
    let raw = raw.trim();
    providers::validation::endpoint(raw).ok()?;
    let authority = raw.split_once("://")?.1.split('/').next()?;
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    Url::parse(raw).ok()
}

async fn probe(
    client: &Client,
    provider_id: ProviderId,
    endpoint: Url,
    user_agent: Option<header::HeaderValue>,
    source: DiagnosticEvidenceSource,
    attempt_timeout: Duration,
    cancel: CancellationToken,
) -> Result<DiagnosticResult, DiagnosticProbeError> {
    let mut timings = Vec::with_capacity(2);
    let outcome = loop {
        let start = Instant::now();
        let mut request = client
            .get(endpoint.clone())
            .timeout(attempt_timeout)
            .header(header::ACCEPT, "*/*")
            .header(header::ACCEPT_ENCODING, "identity");
        if let Some(user_agent) = &user_agent {
            request = request.header(header::USER_AGENT, user_agent.clone());
        }
        let response = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(DiagnosticProbeError::Cancelled),
            response = request.send() => response,
        };
        let elapsed_ms = start.elapsed().as_millis() as u64;
        let phase = DiagnosticPhase::ResponseHeaders;
        timings.push(DiagnosticTiming { phase, elapsed_ms });
        match response {
            Ok(response) => {
                break DiagnosticOutcome::Succeeded {
                    evidence: ReachabilityEvidence {
                        http_status: response.status().as_u16(),
                        slow: elapsed_ms > SLOW_THRESHOLD_MS,
                    },
                };
            }
            Err(error) if error.is_timeout() && timings.len() == 1 => continue,
            Err(error) => {
                let code = if error.is_timeout() {
                    DiagnosticFailureCode::Timeout
                } else {
                    DiagnosticFailureCode::Transport
                };
                break DiagnosticOutcome::Failed {
                    error: DiagnosticFailure { phase, code },
                    http_status: None,
                };
            }
        }
    };
    Ok(DiagnosticResult::Reachability {
        provider_id,
        report: DiagnosticReport { source, timings, outcome },
    })
}

#[cfg(all(test, unix))]
#[path = "reachability_tests.rs"]
mod tests;
