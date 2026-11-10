//! Header latency for candidates resolved by the diagnostic authorization boundary.

use std::time::Duration;

use reqwest::{Client, Url, header};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use vw_agent_apps::{diagnostics::endpoints::EndpointCandidate, providers::validation};
use vw_api_types::agent_apps_diagnostics::{
    DiagnosticEndpointId, DiagnosticEvidenceSource, DiagnosticFailure, DiagnosticFailureCode,
    DiagnosticJobIdentity, DiagnosticOutcome, DiagnosticPhase, DiagnosticProbeKind,
    DiagnosticReport, DiagnosticResult, DiagnosticTiming, SpeedEvidence,
};

use super::super::agent_apps_diagnostic_jobs::{
    DiagnosticJobError, DiagnosticJobOwner, DiagnosticJobs, DiagnosticProbeError,
};
use crate::model_relay::upstream::upstream_http_client_builder;

const DEFAULT_TIMEOUT_SECS: u64 = 8;
const MIN_TIMEOUT_SECS: u64 = 2;
const MAX_TIMEOUT_SECS: u64 = 30;

/// Submit endpoints in request order. The caller must resolve each ID to the exact candidate
/// authorized for the same owner and provider on this execution host. Call on a blocking
/// thread within the host runtime, as required by the job registry.
pub fn start(
    jobs: &DiagnosticJobs,
    owner: DiagnosticJobOwner,
    targets: Vec<(DiagnosticEndpointId, EndpointCandidate)>,
    timeout_secs: Option<u64>,
) -> Result<DiagnosticJobIdentity, DiagnosticJobError> {
    start_with_source(jobs, owner, targets, timeout_secs, DiagnosticEvidenceSource::RealService)
}

fn start_with_source(
    jobs: &DiagnosticJobs,
    owner: DiagnosticJobOwner,
    targets: Vec<(DiagnosticEndpointId, EndpointCandidate)>,
    timeout_secs: Option<u64>,
    source: DiagnosticEvidenceSource,
) -> Result<DiagnosticJobIdentity, DiagnosticJobError> {
    let client = upstream_http_client_builder()
        .retry(reqwest::retry::never())
        .build()
        .map_err(|_| DiagnosticJobError::Unavailable)?;
    let timeout = attempt_timeout(timeout_secs);
    let probes = targets
        .into_iter()
        .map(|(endpoint_id, candidate)| {
            let client = client.clone();
            move |cancel| probe(client, endpoint_id, candidate.url, timeout, source, cancel)
        })
        .collect();
    jobs.start(owner, DiagnosticProbeKind::Speed, probes)
}

fn attempt_timeout(timeout_secs: Option<u64>) -> Duration {
    Duration::from_secs(
        timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS).clamp(MIN_TIMEOUT_SECS, MAX_TIMEOUT_SECS),
    )
}

async fn probe(
    client: Client,
    endpoint_id: DiagnosticEndpointId,
    raw_url: String,
    timeout: Duration,
    source: DiagnosticEvidenceSource,
    cancel: CancellationToken,
) -> Result<DiagnosticResult, DiagnosticProbeError> {
    let Some(endpoint) = parse_endpoint(&raw_url) else {
        return Ok(DiagnosticResult::Speed {
            endpoint_id,
            report: DiagnosticReport {
                source,
                timings: Vec::new(),
                outcome: failure(DiagnosticPhase::Connect, DiagnosticFailureCode::Unsupported),
            },
        });
    };
    let get = |url| {
        client
            .get(url)
            .timeout(timeout)
            .header(header::ACCEPT, "*/*")
            .header(header::ACCEPT_ENCODING, "identity")
            .send()
    };
    let started = Instant::now();
    tokio::select! {
        biased;
        () = cancel.cancelled() => return Err(DiagnosticProbeError::Cancelled),
        _ = get(endpoint.clone()) => {}
    }
    let warmup_ms = started.elapsed().as_millis() as u64;
    let started = Instant::now();
    let response = tokio::select! {
        biased;
        () = cancel.cancelled() => return Err(DiagnosticProbeError::Cancelled),
        response = get(endpoint) => response,
    };
    let latency_ms = started.elapsed().as_millis() as u64;
    let phase = DiagnosticPhase::ResponseHeaders;
    let outcome = match response {
        Ok(response) => DiagnosticOutcome::Succeeded {
            evidence: SpeedEvidence { http_status: response.status().as_u16(), latency_ms },
        },
        Err(error) if error.is_timeout() => failure(phase, DiagnosticFailureCode::Timeout),
        Err(_) => failure(phase, DiagnosticFailureCode::Transport),
    };
    Ok(DiagnosticResult::Speed {
        endpoint_id,
        report: DiagnosticReport {
            source,
            timings: vec![
                DiagnosticTiming { phase: DiagnosticPhase::Warmup, elapsed_ms: warmup_ms },
                DiagnosticTiming { phase, elapsed_ms: latency_ms },
            ],
            outcome,
        },
    })
}

fn parse_endpoint(raw: &str) -> Option<Url> {
    validation::endpoint(raw).ok()?;
    let authority = raw.split_once("://")?.1.split('/').next()?;
    // URL parsing normalizes away empty userinfo and extra slashes before the host.
    if authority.is_empty() || authority.contains('@') {
        return None;
    }
    Url::parse(raw).ok()
}

fn failure(
    phase: DiagnosticPhase,
    code: DiagnosticFailureCode,
) -> DiagnosticOutcome<SpeedEvidence> {
    DiagnosticOutcome::Failed { error: DiagnosticFailure { phase, code }, http_status: None }
}

#[cfg(test)]
#[path = "speedtest_tests.rs"]
mod tests;
