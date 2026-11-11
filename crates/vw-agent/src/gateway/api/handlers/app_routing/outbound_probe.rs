//! Test only the proxy selected by the execution host's published egress configuration.

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
use std::time::Duration;
use tokio::time::{Instant, timeout_at};
use tokio_util::sync::CancellationToken;
use vw_api_types::{
    agent_apps::AppId,
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource, DiagnosticFailure, DiagnosticFailureCode, DiagnosticJobIdentity,
        DiagnosticJobSnapshot, DiagnosticOutcome, DiagnosticPhase, DiagnosticProbeKind,
        DiagnosticReport, DiagnosticResult, DiagnosticScope, DiagnosticTiming,
    },
    app_routing::{RoutingOutboundProbeEvidence, RoutingOutboundProbeRequest},
    id::{ProjectId, ProjectScope, TaskId, TenantScope},
};
use vw_shared::update::desktop_release::SHIJU_MANIFEST;

const PROBE_TIMEOUT: Duration = Duration::from_secs(10);
const KIND: DiagnosticProbeKind = DiagnosticProbeKind::OutboundProxy;
type ProbePath = Result<Path<(String, ProjectId, AppId)>, PathRejection>;
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

pub(crate) async fn start_outbound_probe(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    path: ProbePath,
    headers: HeaderMap,
    body: ConfigBody,
) -> Result<(StatusCode, Json<DiagnosticJobIdentity>), ControlError> {
    let Path(path) = path.map_err(|_| ControlError::InvalidConfig)?;
    let (control, owner) = authorize(control, tenant, headers, path).await?;
    let _: RoutingOutboundProbeRequest = decode(body)?;
    let jobs = control.diagnostic_jobs.ok_or(ControlError::Unavailable)?;
    let egress = control
        .routing
        .configured_egress(owner.scope.project.tenant_scope(), &owner.scope.target_host_id)
        .ok_or(ControlError::Unavailable)?;
    let target = reqwest::Url::parse(SHIJU_MANIFEST).expect("ShiJu manifest URL is valid");
    let deadline = Instant::now() + PROBE_TIMEOUT;
    let job = tokio::task::spawn_blocking(move || {
        jobs.start(
            owner,
            KIND,
            vec![move |cancel| async move {
                if Instant::now() >= deadline {
                    return Err(probe_failure(DiagnosticFailureCode::Timeout));
                }
                let listener = control.routing.bound_address();
                let client = egress
                    .probe_client(&target, listener)
                    .map_err(|_| probe_failure(DiagnosticFailureCode::Transport))?
                    .ok_or_else(|| probe_failure(DiagnosticFailureCode::Unsupported))?;
                probe(client, target, deadline, cancel).await
            }],
        )
        .map_err(job_error)
    })
    .await
    .map_err(|_| ControlError::Unavailable)??;
    Ok((StatusCode::ACCEPTED, Json(job)))
}

pub(crate) async fn outbound_probe_status(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    path: JobPath,
    headers: HeaderMap,
) -> Result<Json<DiagnosticJobSnapshot>, ControlError> {
    outbound_probe_job(control, tenant, path, headers, false).await.map(Json)
}

pub(crate) async fn cancel_outbound_probe(
    control: Option<Extension<RoutingControl>>,
    Extension(tenant): Extension<TenantScope>,
    path: JobPath,
    headers: HeaderMap,
) -> Result<Json<DiagnosticJobSnapshot>, ControlError> {
    outbound_probe_job(control, tenant, path, headers, true).await.map(Json)
}

async fn outbound_probe_job(
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

fn probe_failure(code: DiagnosticFailureCode) -> DiagnosticProbeError {
    DiagnosticProbeError::Failed(DiagnosticFailure {
        phase: DiagnosticPhase::ResponseHeaders,
        code,
    })
}

pub(super) async fn probe(
    client: reqwest::Client,
    target: reqwest::Url,
    deadline: Instant,
    cancel: CancellationToken,
) -> Result<DiagnosticResult, DiagnosticProbeError> {
    let started = Instant::now();
    let response = tokio::select! {
        biased;
        () = cancel.cancelled() => return Err(DiagnosticProbeError::Cancelled),
        response = timeout_at(deadline, client.head(target).send()) => response,
    };
    let response = match response {
        Ok(Ok(response)) => response,
        Err(_) => return Err(probe_failure(DiagnosticFailureCode::Timeout)),
        Ok(Err(error)) if error.is_timeout() => {
            return Err(probe_failure(DiagnosticFailureCode::Timeout));
        }
        Ok(Err(_)) => return Err(probe_failure(DiagnosticFailureCode::Transport)),
    };
    Ok(DiagnosticResult::OutboundProxy {
        report: DiagnosticReport {
            source: DiagnosticEvidenceSource::RealService,
            timings: vec![DiagnosticTiming {
                phase: DiagnosticPhase::ResponseHeaders,
                elapsed_ms: started.elapsed().as_millis() as u64,
            }],
            outcome: DiagnosticOutcome::Succeeded {
                evidence: RoutingOutboundProbeEvidence { http_status: response.status().as_u16() },
            },
        },
    })
}
