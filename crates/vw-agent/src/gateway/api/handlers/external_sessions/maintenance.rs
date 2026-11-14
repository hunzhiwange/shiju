//! Authorized owner of host maintenance handles. Disconnects never abort workers.
use super::catalog::Policy;
use axum::{
    Extension, Json,
    http::{HeaderMap, StatusCode},
};
#[cfg(target_os = "macos")]
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
#[cfg(target_os = "macos")]
use vw_agent_apps::sessions::maintenance::{MaintenanceOperation, MaintenancePhase};
use vw_api_types::{TenantScope, external_session::*};
#[cfg(target_os = "macos")]
pub(super) mod host;

#[cfg(target_os = "macos")]
#[derive(Clone, Default)]
pub(super) struct Jobs(Arc<Mutex<HashMap<String, Job>>>);
#[cfg(target_os = "macos")]
struct Job {
    target: HistoryMaintenanceTarget,
    principal: String,
    root: std::path::PathBuf,
    operation: MaintenanceOperation,
    generation: String,
}
#[cfg(not(target_os = "macos"))]
#[derive(Clone, Default)]
pub(super) struct Jobs;

pub(super) async fn submit(
    Extension(policy): Extension<Policy>,
    Extension(jobs): Extension<Jobs>,
    Extension(snapshots): Extension<super::catalog::Snapshots>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<HistoryMaintenanceRequest>,
) -> Result<Json<HistoryMaintenanceProgress>, StatusCode> {
    if request.target.scope.tenant_scope() != &tenant {
        return Err(StatusCode::FORBIDDEN);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (policy, jobs, snapshots, headers);
        Err(StatusCode::NOT_IMPLEMENTED)
    }
    #[cfg(target_os = "macos")]
    {
        let (principal, _, root, _) = host::authorize(&policy, &headers, &request.target)?;
        let operation_id = uuid::Uuid::new_v4().to_string();
        let generation = match request.generation_id.as_deref() {
            Some(id) => uuid::Uuid::parse_str(id).map_err(|_| StatusCode::BAD_REQUEST)?.to_string(),
            None if request.action == HistoryMaintenanceAction::Restore => {
                return Err(StatusCode::BAD_REQUEST);
            }
            None => uuid::Uuid::new_v4().to_string(),
        };
        let operation = MaintenanceOperation::new();
        let job = Job {
            target: request.target.clone(),
            principal: principal.clone(),
            root,
            operation: operation.clone(),
            generation: generation.clone(),
        };
        let mut map = jobs.0.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        // Bound retained handles; persisted terminal records remain queryable.
        map.retain(|id, j| {
            j.operation.snapshot().details.phase != MaintenancePhase::Terminal
                || host::save(&j.target, &j.principal, &j.root, &progress(id, j)).is_err()
        });
        if map.len() >= 64 {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        let response = progress(&operation_id, &job);
        host::save(&job.target, &principal, &job.root, &response)?;
        map.insert(operation_id.clone(), job);
        let worker = operation.clone();
        let worker_id = operation_id.clone();
        let execute_id = operation_id.clone();
        let worker_generation = generation.clone();
        let owner = principal.clone();
        let worker_target = request.target.clone();
        let worker_root =
            map.get(&operation_id).ok_or(StatusCode::SERVICE_UNAVAILABLE)?.root.clone();
        let expected_root = worker_root.clone();
        if operation
            .start(move |context| {
                host::execute(
                    &policy,
                    &headers,
                    &owner,
                    &expected_root,
                    &request,
                    &worker_generation,
                    &execute_id,
                    &snapshots,
                    context,
                )
            })
            .is_err()
        {
            map.remove(&operation_id);
            return Err(StatusCode::CONFLICT);
        }
        // Persist after termination, including queued cancellation/coordinator rejection.
        tokio::spawn(async move {
            worker.wait().await;
            let j = Job {
                target: worker_target,
                principal: principal.clone(),
                root: worker_root,
                operation: worker,
                generation,
            };
            let _ = tokio::task::spawn_blocking(move || {
                host::save(&j.target, &principal, &j.root, &progress(&worker_id, &j))
            })
            .await;
        });
        Ok(Json(response))
    }
}
pub(super) async fn query(
    Extension(policy): Extension<Policy>,
    Extension(jobs): Extension<Jobs>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<HistoryMaintenanceQuery>,
) -> Result<Json<HistoryMaintenanceProgress>, StatusCode> {
    inspect(policy, jobs, tenant, headers, request, false).map(Json)
}
pub(super) async fn cancel(
    Extension(policy): Extension<Policy>,
    Extension(jobs): Extension<Jobs>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    Json(request): Json<HistoryMaintenanceQuery>,
) -> Result<Json<HistoryMaintenanceProgress>, StatusCode> {
    inspect(policy, jobs, tenant, headers, request, true).map(Json)
}
fn inspect(
    policy: Policy,
    jobs: Jobs,
    tenant: TenantScope,
    headers: HeaderMap,
    request: HistoryMaintenanceQuery,
    cancel: bool,
) -> Result<HistoryMaintenanceProgress, StatusCode> {
    if request.target.scope.tenant_scope() != &tenant {
        return Err(StatusCode::FORBIDDEN);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (policy, jobs, headers, cancel);
        Err(StatusCode::NOT_IMPLEMENTED)
    }
    #[cfg(target_os = "macos")]
    {
        let (principal, _, root, _) = host::authorize(&policy, &headers, &request.target)?;
        let map = jobs.0.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        if let Some(job) = map.get(&request.operation_id) {
            if job.target != request.target || job.principal != principal || job.root != root {
                return Err(StatusCode::FORBIDDEN);
            }
            if cancel {
                job.operation.cancel();
            }
            Ok(progress(&request.operation_id, job))
        } else {
            host::load(&request.target, &principal, &root, &request.operation_id)
        }
    }
}
#[cfg(target_os = "macos")]
fn progress(id: &str, job: &Job) -> HistoryMaintenanceProgress {
    let s = job.operation.snapshot();
    let (changed_rollouts, changed_state_rows, skipped_reason) = s
        .details
        .outcome
        .map(|o| (o.changed_rollouts, o.changed_state_rows, o.skipped_reason.map(str::to_owned)))
        .unwrap_or_default();
    let phase = match s.details.phase {
        MaintenancePhase::Preparing => "preparing",
        MaintenancePhase::ThirdParty => "third_party",
        MaintenancePhase::Unifying => "unifying",
        MaintenancePhase::Restoring => "restoring",
        MaintenancePhase::LegacyOfficial => "legacy_official",
        MaintenancePhase::Terminal => "terminal",
    };
    HistoryMaintenanceProgress {
        operation_id: id.into(),
        generation_id: job.generation.clone(),
        status: s.operation.status,
        phase: phase.into(),
        cancellation_requested: s.cancellation_requested,
        completed: s.details.completed,
        total: s.details.total,
        changed_rollouts,
        changed_state_rows,
        skipped_reason,
        interrupted: false,
    }
}

pub(super) mod history_settings;
