//! Single, bounded model generation probe using an authorized saved provider.

use std::time::Duration;

use reqwest::{StatusCode, header};
use tokio::time::{Instant, timeout};
use tokio_util::sync::CancellationToken;
use vw_agent_apps::auth::copilot::CopilotAuth;
use vw_api_types::{
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource, DiagnosticFailure, DiagnosticFailureCode,
        DiagnosticModelProtocol, DiagnosticOutcome, DiagnosticPhase, DiagnosticReport,
        DiagnosticResult, DiagnosticTiming, ModelEvidence, ModelProtocolAttempt,
        ModelProtocolEvidence,
    },
    id::{ModelId, ProviderId},
};

use super::super::agent_apps_diagnostic_jobs::{DiagnosticJobOwner, DiagnosticProbeError};
use crate::{
    agent_apps::auth_accounts::AuthAccountStore,
    global_database::GlobalDatabase,
    model_relay::upstream::{self, upstream_http_client_builder},
};

use crate::model_relay::generation::GenerationProtocol;
use sha2::{Digest, Sha256};

pub(super) mod response;
pub(super) mod target;

const TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RESPONSE_BYTES: usize = 64 * 1024;

pub use target::ModelPrepareError;

pub struct ModelProbe {
    target: target::ModelTarget,
    source: GenerationProtocol,
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
        target.select_protocol(target.protocol)?;
        let source = target.protocol;
        Ok(Self { target, source })
    }

    /// An explicit client protocol is checked against the registered saved target.
    pub fn prepare_protocol(
        database: &GlobalDatabase,
        accounts: &AuthAccountStore,
        owner: DiagnosticJobOwner,
        provider_id: ProviderId,
        model_id: ModelId,
        protocol: Option<DiagnosticModelProtocol>,
    ) -> Result<Self, ModelPrepareError> {
        let mut probe = Self::prepare(database, accounts, owner, provider_id, model_id)?;
        if let Some(protocol) = protocol {
            let source = response::internal_protocol(protocol);
            probe.target.select_protocol(source)?;
            probe.source = source;
        }
        Ok(probe)
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
        let source_protocol = self.source;
        let started = Instant::now();
        let started_at = vw_api_types::TimestampMs(chrono::Utc::now().timestamp_millis());
        let mut endpoint_fingerprint = None;
        let mut phase = DiagnosticPhase::Connect;
        let mut status = None;
        let result = tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(DiagnosticProbeError::Cancelled),
            result = timeout(TIMEOUT, async {
                target.check_current(accounts)?;
                let (url, auth) = target.destination(accounts, copilot).await?;
                let fingerprint = hex::encode(Sha256::digest(url.as_str().as_bytes()));
                endpoint_fingerprint = Some(fingerprint.clone());
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
                    .json(&response::request(source_protocol, target.protocol, target.model_id.as_ref())?)
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
                    StatusCode::TOO_MANY_REQUESTS => {
                        return Err(DiagnosticFailureCode::RateLimited);
                    }
                    StatusCode::UNSUPPORTED_MEDIA_TYPE => {
                        return Err(DiagnosticFailureCode::Unsupported);
                    }
                    http if !http.is_success() => {
                        return Err(DiagnosticFailureCode::UnexpectedHttpStatus);
                    }
                    _ => {}
                }
                phase = DiagnosticPhase::ModelResponse;
                let actual_model_id = response::actual_model(source_protocol, target.protocol, &read_response(&mut response).await?)?;
                target.check_current(accounts)?;
                Ok(ModelEvidence {
                    http_status: response.status().as_u16(),
                    actual_model_id,
                    route: Some(ModelProtocolEvidence {
                        source_protocol: response::public_protocol(source_protocol),
                        upstream_protocol: response::public_protocol(target.protocol),
                        endpoint_fingerprint: fingerprint,
                        checked_at: vw_api_types::TimestampMs(chrono::Utc::now().timestamp_millis()),
                        verified_fields: vec!["text".into(), "terminal_status".into(), "model".into()],
                    }),
                })
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
            attempt: Some(ModelProtocolAttempt {
                source_protocol: response::public_protocol(source_protocol),
                upstream_protocol: response::public_protocol(target.protocol),
                started_at,
                endpoint_fingerprint,
            }),
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
