//! Single, bounded model generation probe using an authorized saved provider.

use std::time::Duration;

use reqwest::{StatusCode, header};
use tokio::time::{Instant, timeout};
use tokio_util::sync::CancellationToken;
use vw_agent_apps::auth::copilot::CopilotAuth;
use vw_api_types::{
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource, DiagnosticFailure, DiagnosticFailureCode, DiagnosticOutcome,
        DiagnosticPhase, DiagnosticReport, DiagnosticResult, DiagnosticTiming, ModelEvidence,
    },
    id::{ModelId, ProviderId},
};

use super::super::agent_apps_diagnostic_jobs::{DiagnosticJobOwner, DiagnosticProbeError};
use crate::{
    agent_apps::auth_accounts::AuthAccountStore,
    global_database::GlobalDatabase,
    model_relay::upstream::{self, upstream_http_client_builder},
};

mod response;
mod target;

const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

pub use target::ModelPrepareError;

pub struct ModelProbe {
    target: target::ModelTarget,
}

impl ModelProbe {
    /// Use an owner already authorized for diagnostics on this execution host.
    /// Call on a blocking thread. Only provider and model identities are accepted.
    pub fn prepare(
        database: &GlobalDatabase,
        accounts: &AuthAccountStore,
        owner: DiagnosticJobOwner,
        provider_id: ProviderId,
        model_id: ModelId,
    ) -> Result<Self, ModelPrepareError> {
        let target =
            target::ModelTarget::prepare(database, accounts, owner, provider_id, model_id)?;
        Ok(Self { target })
    }

    /// Run within the diagnostic job's permit and cancellation scope.
    pub async fn run(
        self,
        accounts: &AuthAccountStore,
        copilot: &CopilotAuth,
        cancel: CancellationToken,
    ) -> Result<DiagnosticResult, DiagnosticProbeError> {
        self.run_with_source(accounts, copilot, cancel, DiagnosticEvidenceSource::RealService).await
    }

    async fn run_with_source(
        self,
        accounts: &AuthAccountStore,
        copilot: &CopilotAuth,
        cancel: CancellationToken,
        source: DiagnosticEvidenceSource,
    ) -> Result<DiagnosticResult, DiagnosticProbeError> {
        let target = self.target;
        let started = Instant::now();
        let mut phase = DiagnosticPhase::Connect;
        let mut status = None;
        let result = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(DiagnosticProbeError::Cancelled),
            result = timeout(TIMEOUT, async {
                target.check_current(accounts)?;
                let (url, auth) = target.destination(accounts, copilot).await?;
                target.check_current(accounts)?;
                let client = upstream_http_client_builder().retry(reqwest::retry::never());
                let client = if matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")) {
                    client.no_proxy()
                } else {
                    client
                };
                let client = client
                    .build()
                    .map_err(|_| DiagnosticFailureCode::Transport)?;
                let request = client
                    .post(url.clone())
                    .headers(auth)
                    .header(header::ACCEPT, "application/json")
                    .json(&target.protocol.prompt(target.model_id.as_ref()))
                    .build()
                    .map_err(|_| DiagnosticFailureCode::Unsupported)?;
                phase = DiagnosticPhase::ResponseHeaders;
                let mut response = upstream::execute(&client, &url, request, &cancel)
                    .await
                    .map_err(|_| DiagnosticFailureCode::Transport)?;
                status = Some(response.status().as_u16());
                match response.status() {
                    StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => {
                        return Err(DiagnosticFailureCode::Unauthorized);
                    }
                    http if !http.is_success() => {
                        return Err(DiagnosticFailureCode::UnexpectedHttpStatus);
                    }
                    _ => {}
                }
                phase = DiagnosticPhase::ModelResponse;
                let actual_model_id = target.protocol.actual_model(&read_response(&mut response).await?)?;
                target.check_current(accounts)?;
                Ok(ModelEvidence { http_status: response.status().as_u16(), actual_model_id })
            }) => result,
        };
        let failed = |code| DiagnosticOutcome::Failed {
            error: DiagnosticFailure { phase, code },
            http_status: status,
        };
        let outcome = match result {
            Ok(Ok(evidence)) => DiagnosticOutcome::Succeeded { evidence },
            Ok(Err(code)) => failed(code),
            Err(_) => failed(DiagnosticFailureCode::Timeout),
        };
        Ok(DiagnosticResult::Model {
            provider_id: target.provider_id,
            model_id: target.model_id,
            may_incur_cost: true,
            report: DiagnosticReport {
                source,
                timings: vec![DiagnosticTiming {
                    phase,
                    elapsed_ms: started.elapsed().as_millis() as u64,
                }],
                outcome,
            },
        })
    }
}

async fn read_response(
    response: &mut reqwest::Response,
) -> Result<serde_json::Value, DiagnosticFailureCode> {
    let invalid = DiagnosticFailureCode::InvalidResponse;
    let json = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"));
    if !json || response.content_length().is_some_and(|length| length > MAX_RESPONSE_BYTES as u64) {
        return Err(invalid);
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| DiagnosticFailureCode::Transport)? {
        if chunk.len() > MAX_RESPONSE_BYTES - bytes.len() {
            return Err(invalid);
        }
        bytes.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&bytes).map_err(|_| invalid)
}

#[cfg(all(test, target_os = "macos"))]
#[path = "model_check_tests.rs"]
mod tests;
