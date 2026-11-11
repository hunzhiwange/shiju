//! Scan only the fixed source candidates on the authorized execution host.

use super::{
    config_api::{ConfigBody, decode},
    lifecycle_api::{ControlError, RoutingControl},
};
use crate::gateway::agent_apps_diagnostic_jobs::{
    DiagnosticJobError, DiagnosticJobOwner, DiagnosticProbeError,
};
use axum::{
    Extension, Json,
    extract::{Path, rejection::PathRejection},
    http::{HeaderMap, StatusCode},
};
use std::{
    net::{Ipv4Addr, SocketAddrV4},
    time::Duration,
};
use tokio::{
    net::TcpStream,
    time::{Instant, timeout_at},
};
use tokio_util::sync::CancellationToken;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource, DiagnosticFailure, DiagnosticFailureCode, DiagnosticJobIdentity,
        DiagnosticJobSnapshot, DiagnosticOutcome, DiagnosticPhase, DiagnosticProbeKind,
        DiagnosticReport, DiagnosticResult, DiagnosticScope, DiagnosticTiming,
    },
    app_routing::{RoutingOutboundScanEvidence, RoutingOutboundScanRequest},
    id::{ProjectId, ProjectScope, TaskId, TenantScope},
};

const SCAN_TIMEOUT: Duration = Duration::from_secs(2);
const PORT_TIMEOUT: Duration = Duration::from_millis(100);

// CC Switch fixed candidates; no caller-controlled addresses or ports.
pub(super) const CANDIDATES: &[(u16, &[&str])] = &[
    (7890, &["http", "socks5"]),
    (7891, &["socks5"]),
    (1080, &["socks5"]),
    (8080, &["http"]),
    (8888, &["http"]),
    (3128, &["http"]),
    (10808, &["socks5"]),
    (10809, &["http"]),
];
const KIND: DiagnosticProbeKind = DiagnosticProbeKind::OutboundScan;
type ScanPath = Result<Path<(String, ProjectId, AppId)>, PathRejection>;
type JobPath = Result<Path<(String, ProjectId, AppId, TaskId)>, PathRejection>;

async fn authorize(
    control: Option<Extension<RoutingControl>>,
    tenant: TenantScope,
    headers: HeaderMap,
    (host, project, app): (String, ProjectId, AppId),
) -> Result<(RoutingControl, DiagnosticJobOwner), ControlError> {
    let (control, _, principal) = RoutingControl::authorize_member(
        control,
        headers,
        tenant.clone(),
        host.clone(),
        Some((project.clone(), app)),
        true,
    )
    .await?;
    if !vw_agent_apps::routing::supports_app(app) {
        return Err(ControlError::UnsupportedApp);
    }
    let project = ProjectScope::new(tenant, project).map_err(|_| ControlError::InvalidConfig)?;
    let scope = DiagnosticScope { project, target_host_id: host, app_id: app };
    Ok((control, DiagnosticJobOwner { scope, principal }))
}

pub(crate) async fn start_outbound_scan(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    path: ScanPath,
    headers: HeaderMap,
    body: ConfigBody,
) -> Result<(StatusCode, Json<DiagnosticJobIdentity>), ControlError> {
    let Path(path) = path.map_err(|_| ControlError::InvalidConfig)?;
    let (control, owner) = authorize(control, tenant, headers, path).await?;
    let _: RoutingOutboundScanRequest = decode(body)?;
    let jobs = control.diagnostic_jobs.ok_or(ControlError::Unavailable)?;
    let deadline = Instant::now() + SCAN_TIMEOUT;
    let job = tokio::task::spawn_blocking(move || {
        jobs
            .start(
                owner,
                KIND,
                CANDIDATES
                    .iter()
                    .map(|&(port, protocols)| {
                        move |cancel| async move {
                            scan_candidate(port, protocols, deadline, cancel).await
                        }
                    })
                    .collect(),
            )
            .map_err(job_error)
    })
    .await
    .map_err(|_| ControlError::Unavailable)??;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

pub(crate) async fn outbound_scan_status(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    path: JobPath,
    headers: HeaderMap,
) -> Result<Json<DiagnosticJobSnapshot>, ControlError> {
    outbound_scan_job(control, tenant, path, headers, false).await.map(Json)
}

pub(crate) async fn cancel_outbound_scan(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    path: JobPath,
    headers: HeaderMap,
) -> Result<Json<DiagnosticJobSnapshot>, ControlError> {
    outbound_scan_job(control, tenant, path, headers, true).await.map(Json)
}

async fn outbound_scan_job(
    control: Option<Extension<RoutingControl>>,
    tenant: TenantScope,
    path: JobPath,
    headers: HeaderMap,
    cancel: bool,
) -> Result<DiagnosticJobSnapshot, ControlError> {
    let Path((host, project, app, task)) = path.map_err(|_| ControlError::InvalidConfig)?;
    let (control, owner) = authorize(control, tenant, headers, (host, project, app)).await?;
    let jobs = control.diagnostic_jobs.ok_or(ControlError::Unavailable)?;
    tokio::task::spawn_blocking(move || {
        let snapshot = jobs.snapshot(&owner, &task).map_err(job_error)?;
        if snapshot.job.kind != KIND {
            return Err(ControlError::DiagnosticNotFound);
        }
        if cancel { jobs.cancel(&owner, &task).map_err(job_error) } else { Ok(snapshot) }
    })
    .await
    .map_err(|_| ControlError::Unavailable)?
}

fn job_error(error: DiagnosticJobError) -> ControlError {
    match error {
        DiagnosticJobError::InvalidInput => ControlError::InvalidConfig,
        DiagnosticJobError::CapacityExceeded => ControlError::DiagnosticCapacity,
        DiagnosticJobError::Unavailable => ControlError::Unavailable,
        DiagnosticJobError::NotFound => ControlError::DiagnosticNotFound,
    }
}

fn scan_failure(code: DiagnosticFailureCode) -> DiagnosticProbeError {
    DiagnosticProbeError::Failed(DiagnosticFailure { phase: DiagnosticPhase::Connect, code })
}

async fn scan_candidate(
    port: u16,
    protocols: &'static [&'static str],
    deadline: Instant,
    cancel: CancellationToken,
) -> Result<DiagnosticResult, DiagnosticProbeError> {
    scan_connection(
        TcpStream::connect(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port)),
        port,
        protocols,
        deadline,
        cancel,
    )
    .await
}

// The future is supplied separately so deadline and cancellation tests never need
// occupied fixed ports. Production always constructs the address above.
pub(super) async fn scan_connection<F>(
    connection: F,
    port: u16,
    protocols: &[&str],
    deadline: Instant,
    cancel: CancellationToken,
) -> Result<DiagnosticResult, DiagnosticProbeError>
where
    F: std::future::Future<Output = std::io::Result<TcpStream>>,
{
    let started = Instant::now();
    if started >= deadline {
        return Err(scan_failure(DiagnosticFailureCode::Timeout));
    }
    let connected = tokio::select! {
        biased;
        () = cancel.cancelled() => return Err(DiagnosticProbeError::Cancelled),
        result = timeout_at(deadline.min(started + PORT_TIMEOUT), connection) => {
            // Drop the socket before publishing completion; send no application bytes.
            match result {
                Ok(Ok(socket)) => {
                    drop(socket);
                    true
                }
                Ok(Err(_)) | Err(_) => false,
            }
        },
    };
    if Instant::now() >= deadline {
        return Err(scan_failure(DiagnosticFailureCode::Timeout));
    }
    Ok(DiagnosticResult::OutboundScan {
        report: DiagnosticReport {
            source: DiagnosticEvidenceSource::RealService,
            timings: vec![DiagnosticTiming {
                phase: DiagnosticPhase::Connect,
                elapsed_ms: started.elapsed().as_millis() as u64,
            }],
            outcome: DiagnosticOutcome::Succeeded {
                evidence: RoutingOutboundScanEvidence {
                    port,
                    tcp_open: connected,
                    protocol_candidates: if connected {
                        protocols.iter().map(|protocol| (*protocol).to_owned()).collect()
                    } else {
                        Vec::new()
                    },
                },
            },
        },
    })
}
