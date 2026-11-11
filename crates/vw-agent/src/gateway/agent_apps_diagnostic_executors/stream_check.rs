//! Authorized, bounded streaming probe using the production protocol observer.

use std::time::Duration;

use reqwest::{StatusCode, header};
use tokio::time::{Instant, timeout};
use tokio_util::sync::CancellationToken;
use vw_agent_apps::auth::copilot::CopilotAuth;
use vw_api_types::{
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource, DiagnosticFailure, DiagnosticFailureCode, DiagnosticOutcome,
        DiagnosticPhase, DiagnosticReport, DiagnosticResult, DiagnosticTiming, StreamingEvidence,
    },
    id::{ModelId, ProviderId},
};

use super::super::agent_apps_diagnostic_jobs::{DiagnosticJobOwner, DiagnosticProbeError};
use crate::{
    agent_apps::auth_accounts::AuthAccountStore,
    global_database::GlobalDatabase,
    model_relay::upstream::{self, upstream_http_client_builder},
};

use super::model_check::{response::GenerationEvidence, target};

const TIMEOUT: Duration = Duration::from_secs(30);

pub use target::ModelPrepareError;

pub struct StreamProbe {
    target: target::ModelTarget,
}

impl StreamProbe {
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
                let (mut url, auth) = target.destination(accounts, copilot).await?;
                if target.protocol == crate::model_relay::generation::GenerationProtocol::Gemini {
                    let path = url.path().strip_suffix(":generateContent")
                        .ok_or(DiagnosticFailureCode::Unsupported)?;
                    url.set_path(&format!("{path}:streamGenerateContent"));
                    url.query_pairs_mut().append_pair("alt", "sse");
                }
                let mut prompt = target.protocol.prompt(target.model_id.as_ref());
                if target.protocol != crate::model_relay::generation::GenerationProtocol::Gemini {
                    prompt["stream"] = true.into();
                }
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
                    .header(header::ACCEPT, "text/event-stream")
                    .json(&prompt)
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
                phase = DiagnosticPhase::FirstEvent;
                let evidence = read_stream(&mut response, target.protocol, started, &mut phase).await?;
                target.check_current(accounts)?;
                Ok(evidence)
            }) => result,
        };
        let failed = |code| DiagnosticOutcome::Failed {
            error: DiagnosticFailure { phase, code },
            http_status: status,
        };
        let outcome = if started.elapsed() >= TIMEOUT {
            failed(DiagnosticFailureCode::Timeout)
        } else {
            match result {
                Ok(Ok(evidence)) => DiagnosticOutcome::Succeeded { evidence },
                Ok(Err(code)) => failed(code),
                Err(_) => failed(DiagnosticFailureCode::Timeout),
            }
        };
        Ok(DiagnosticResult::Streaming {
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

async fn read_stream(
    response: &mut reqwest::Response,
    protocol: crate::model_relay::generation::GenerationProtocol,
    started: Instant,
    phase: &mut DiagnosticPhase,
) -> Result<StreamingEvidence, DiagnosticFailureCode> {
    use crate::model_relay::stream_observer::StreamObserver;
    use tokio::time::timeout_at;

    const EVENT_TIMEOUT: Duration = Duration::from_secs(10);
    const MAX_BYTES: usize = 1024 * 1024;
    let invalid = DiagnosticFailureCode::InvalidResponse;
    let content_type = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next());
    if !content_type.is_some_and(|value| value.trim().eq_ignore_ascii_case("text/event-stream"))
        || response.content_length().is_some_and(|length| length > MAX_BYTES as u64)
    {
        return Err(invalid);
    }
    let mut observer = StreamObserver::new(protocol);
    let mut received = 0;
    let mut first = None;
    let mut last = started;
    let mut max_gap = None;
    loop {
        // Keepalives and metadata do not extend either event deadline.
        let deadline = last + EVENT_TIMEOUT;
        if Instant::now() >= deadline {
            return Err(DiagnosticFailureCode::Timeout);
        }
        let next = timeout_at(deadline, response.chunk())
            .await
            .map_err(|_| DiagnosticFailureCode::Timeout)?
            .map_err(|_| DiagnosticFailureCode::Transport)?;
        if Instant::now() >= deadline {
            return Err(DiagnosticFailureCode::Timeout);
        }
        let Some(bytes) = next else {
            return Err(invalid);
        };
        if bytes.len() > MAX_BYTES - received {
            return Err(invalid);
        }
        received += bytes.len();
        let update = observer.push(&bytes).map_err(|_| invalid)?;
        let now = Instant::now();
        if update.text_events > 0 || update.completed && first.is_some() {
            if first.is_none() {
                first = Some(now);
                *phase = DiagnosticPhase::Stream;
            } else {
                let gap = now.duration_since(last).as_millis() as u64;
                max_gap = Some(max_gap.map_or(gap, |previous: u64| previous.max(gap)));
            }
            last = now;
        }
        if update.text_events > 1 || update.text_events > 0 && update.completed {
            max_gap = Some(max_gap.unwrap_or(0));
        }
        if update.completed {
            observer.finish().map_err(|_| invalid)?;
            let first = first.ok_or(invalid)?;
            let model = observer.actual_model().ok_or(invalid)?;
            let actual_model_id = ModelId::from(model);
            vw_agent_apps::models::edits::validate_model_id(&actual_model_id)
                .map_err(|_| invalid)?;
            return Ok(StreamingEvidence {
                http_status: response.status().as_u16(),
                actual_model_id,
                first_event_ms: first.duration_since(started).as_millis() as u64,
                max_event_gap_ms: max_gap,
            });
        }
    }
}

#[cfg(all(test, target_os = "macos"))]
#[path = "stream_check_tests.rs"]
mod tests;
