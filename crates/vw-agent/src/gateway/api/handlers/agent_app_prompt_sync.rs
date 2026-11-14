//! Saved-state sync jobs, resolved through the existing host operation endpoints.
use axum::{Extension, Json, Router, extract::DefaultBodyLimit, http::{HeaderMap, StatusCode}, response::{IntoResponse, Response}, routing::post};
use serde::Deserialize;
use std::{collections::HashMap, path::PathBuf, sync::{Arc, Mutex}};
use vw_agent_apps::{apps::lifecycle::operations::{Operation, OperationFailure, Phase}, prompts::sync::{Outcome, TargetResult}};
use vw_api_types::{ProjectId, ProjectScope, TenantScope, agent_apps::AppId};
use crate::{global_database::GlobalDatabase, agent_apps::prompts_store::{PromptScope, PromptsStore}};

#[derive(Clone, Default)]
pub(super) struct Jobs(Arc<Mutex<HashMap<String, Arc<Job>>>>);
pub(super) struct Job {
    pub(super) operation: Operation,
    project: ProjectScope,
    host: String,
    owner: String,
    apps: Vec<AppId>,
    results: Mutex<Vec<TargetResult>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Request {
    project_id: ProjectId,
    target_host_id: String,
    apps: Vec<AppId>,
}
#[derive(Clone)]
struct Policy(Option<PathBuf>);
pub(super) fn router() -> Router {
    Router::new().route("/agent-apps/prompts/sync", post(submit))
        .layer(DefaultBodyLimit::max(4096))
        .layer(Extension(Policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))))
}
impl Job {
    pub(super) fn response(&self, id: String) -> Response {
        let targets: Vec<_> = self.results.lock().unwrap_or_else(|e| e.into_inner()).iter().map(|r| {
            let (status, reason, duplicate) = match &r.outcome {
                Outcome::Published { duplicate_selection } => ("succeeded", None, *duplicate_selection),
                Outcome::Skipped(reason) => ("skipped", Some(match reason {
                    vw_agent_apps::prompts::sync::SkipReason::NativePiSelection => "native_pi_selection",
                    vw_agent_apps::prompts::sync::SkipReason::UnsupportedClaudeDesktop => "unsupported_claude_desktop",
                    vw_agent_apps::prompts::sync::SkipReason::NoSelection => "no_selection",
                }), false),
                Outcome::Failed => ("failed", None, false),
                Outcome::Cancelled => ("not_executed", Some("cancelled"), false),
            };
            serde_json::json!({"appId": r.app, "status": status, "reason": reason, "duplicateSelection": duplicate})
        }).collect();
        ([(axum::http::header::CACHE_CONTROL, "no-store")], Json(serde_json::json!({
            "operation": super::agent_apps::lifecycle::status(id, &self.operation),
            "projectId": self.project.project_id(), "targetHostId": self.host, "targets": targets,
        }))).into_response()
    }
}
impl Jobs {
    pub(super) fn owned(&self, id: &str, tenant: &TenantScope, host: &str, headers: &HeaderMap,
        policy: Option<&std::path::Path>) -> Result<Option<Arc<Job>>, StatusCode> {
        let job = self.0.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.get(id).cloned();
        let Some(job) = job else { return Ok(None); };
        if job.project.tenant_scope() != tenant || job.host != host { return Err(StatusCode::NOT_FOUND); }
        for &app in &job.apps {
            let context = super::agent_apps::authorize_prompts_activation(policy, headers, &job.project, host, app)?;
            if context.2 != job.owner { return Err(StatusCode::FORBIDDEN); }
        }
        Ok(Some(job))
    }
}
async fn submit(Extension(jobs): Extension<Jobs>, Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>, database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap, body: Result<Json<Request>, axum::extract::rejection::JsonRejection>) -> Result<Response, StatusCode> {
    let Json(q) = body.map_err(|_| StatusCode::BAD_REQUEST)?;
    vw_agent_apps::providers::validate_identity(&q.target_host_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    if q.apps.is_empty() || q.apps.len() > 32 || q.apps.iter().enumerate().any(|(i, a)| q.apps[..i].contains(a)) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let project = ProjectScope::new(tenant, q.project_id).map_err(|_| StatusCode::BAD_REQUEST)?;
    let db = database.ok_or(StatusCode::SERVICE_UNAVAILABLE)?.0;
    let p = project.clone();
    let h = headers.clone();
    let apps = q.apps.clone();
    let host = q.target_host_id.clone();
    let path = policy.0.clone();
    let contexts = tokio::task::spawn_blocking(move || {
        let mut contexts = Vec::new();
        for app in apps {
            vw_api_types::agent_apps::PromptResource::Application { app_id: app }.validate_application_access()
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
            contexts.push(super::agent_apps::authorize_prompts_activation(path.as_deref(), &h, &p, &host, app)?);
        }
        Ok::<_, StatusCode>(contexts)
    }).await.map_err(|_| StatusCode::SERVICE_UNAVAILABLE)??;
    let job = Arc::new(Job { operation: Operation::new(), project, host: q.target_host_id,
        owner: contexts[0].2.clone(), apps: q.apps, results: Mutex::new(Vec::new()) });
    if contexts.iter().any(|c| c.2 != job.owner) { return Err(StatusCode::FORBIDDEN); }
    let mut registry = jobs.0.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    if registry.len() >= 1024 { registry.retain(|_, j| j.operation.snapshot().phase != Phase::Terminal); }
    if registry.len() >= 1024 { return Err(StatusCode::TOO_MANY_REQUESTS); }
    let id = uuid::Uuid::new_v4().to_string();
    let worker = job.clone();
    job.operation.start_task(async move {
        tokio::task::spawn_blocking(move || run(worker, policy.0, headers, db, contexts)).await
            .unwrap_or(Err(OperationFailure::Failed))
    }).map_err(|_| StatusCode::CONFLICT)?;
    registry.insert(id.clone(), job.clone());
    let mut response = job.response(id);
    *response.status_mut() = StatusCode::ACCEPTED;
    Ok(response)
}
type Context = (PathBuf, Option<PathBuf>, String, PathBuf);
fn run(job: Arc<Job>, policy: Option<PathBuf>, headers: HeaderMap, db: GlobalDatabase,
    contexts: Vec<Context>) -> Result<(), OperationFailure> {
    use vw_agent_apps::providers::live::{commit::{ProviderTransactions, ExecutionHostId}, write_guard::WriteError};
    let store = PromptsStore::new(db).map_err(|_| OperationFailure::Failed)?;
    let environment = std::env::vars_os().filter_map(|(k,v)| Some((k.into_string().ok()?, v.into_string().ok()?))).collect();
    let report = vw_agent_apps::prompts::sync::sync_all(&job.apps, &job.operation, |app| {
        let context = &contexts[job.apps.iter().position(|a| *a == app).ok_or(())?];
        let tx = ProviderTransactions::open_existing(&context.3, ExecutionHostId::new(&job.host).map_err(|_| ())?)
            .map_err(|_| ())?.ok_or(())?;
        tx.state(app).map_err(|_| ())?.require_project(&job.project).map_err(|_| ())?;
        store.sync_to_live(PromptScope { project: &job.project, host: &job.host, app }, &job.host,
            &context.0, None, &environment, &tx, || {
                let fresh = super::agent_apps::authorize_prompts_activation(policy.as_deref(), &headers, &job.project, &job.host, app)
                    .map_err(|_| WriteError::Conflict)?;
                if &fresh != context { return Err(WriteError::Conflict); }
                Ok(())
            }).map_err(|_| ())
    }, |result| job.results.lock().unwrap_or_else(|e| e.into_inner()).push(result.clone()));
    if report.cancelled { Err(OperationFailure::Cancelled) }
    else if report.targets.iter().any(|r| matches!(r.outcome, Outcome::Failed)) { Err(OperationFailure::Failed) }
    else { Ok(()) }
}
