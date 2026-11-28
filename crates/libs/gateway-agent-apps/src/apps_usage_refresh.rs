//! Manual usage refreshes use the same query service as scheduled refreshes.

use std::{
    collections::HashMap,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, rejection::JsonRejection},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use parking_lot::Mutex;
use serde::Serialize;
use uuid::Uuid;
use vw_agent_apps::{
    apps::lifecycle::operations::{Operation, OperationFailure, Phase},
    providers::validate_identity,
    usage::cache::UsageCacheResult,
};
use {
    vw_api_app_operations::AppActionExecutionResult,
    vw_api_app_identity::AppId,
    vw_api_app_operations::AppOperationStatus,
    vw_api_app_auth::AuthErrorCode,
    vw_api_apps_usage::apps_usage::UsageQueryKind,
    vw_api_apps_usage::apps_usage::UsageQueryOutcome,
    vw_api_apps_usage::apps_usage::UsageSnapshotQuery,
    vw_api_transport::id::ProjectId,
    vw_api_transport::id::ProjectScope,
    vw_api_transport::id::TaskId,
    vw_api_transport::id::TenantScope,
    vw_api_task::TaskStatus,
};

use super::{
    access::{self, UsageRefreshTarget},
    lifecycle,
    usage_read::ReadError,
    usage_refresh::{Query, RefreshError, UsageRefresh},
};

const MAX_OPERATIONS: usize = 128;
const RETENTION: Duration = Duration::from_secs(10 * 60);

#[derive(Clone)]
struct Policy(Option<PathBuf>);

pub fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/apps/usage/refresh", post(single))
        .layer(DefaultBodyLimit::max(8192))
        .layer(Extension(Policy(policy)))
}

async fn single(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    service: Option<Extension<Arc<UsageRefresh>>>,
    input: Result<Json<UsageSnapshotQuery>, JsonRejection>,
) -> Result<impl IntoResponse, ReadError> {
    let Json(query) = input.map_err(|_| AuthErrorCode::InvalidInput)?;
    validate_identity(&query.provider_id).map_err(|_| AuthErrorCode::InvalidInput)?;
    if let Some(account) = &query.account_id {
        validate_identity(account).map_err(|_| AuthErrorCode::InvalidInput)?;
    }
    validate_identity(&query.target_host_id).map_err(|_| AuthErrorCode::InvalidInput)?;
    let project =
        ProjectScope::new(tenant, query.project_id.clone()).map_err(|_| AuthErrorCode::InvalidInput)?;
    let Extension(service) = service.ok_or(AuthErrorCode::StorageUnavailable)?;
    let policy = policy.0.unwrap_or_else(|| access::tenant_policy_path(project.tenant_scope()));
    let host = query.target_host_id.clone();
    let (principal, target, prepared) = tokio::task::spawn_blocking({
        let project = project.clone();
        let service = service.clone();
        let policy = policy.clone();
        move || {
            let auth = access::authorize_usage_refresh(
                &policy,
                &headers,
                &project,
                &query.target_host_id,
            )
            .map_err(ReadError::access)?;
            let kind = if query.account_id.is_some() {
                if query.app_id != AppId::Codex
                    || query.kind.is_some_and(|kind| kind != UsageQueryKind::OfficialSubscription)
                {
                    return Err(AuthErrorCode::Unsupported.into());
                }
                UsageQueryKind::OfficialSubscription
            } else {
                let kind = query.kind.unwrap_or(UsageQueryKind::CustomScript);
                if kind == UsageQueryKind::OfficialSubscription {
                    return Err(AuthErrorCode::Unsupported.into());
                }
                kind
            };
            let targets = access::usage_targets(&policy).map_err(ReadError::access)?;
            let mut selected: Option<(UsageRefreshTarget, Result<Query, RefreshError>)> = None;
            for target in targets.into_iter().filter(|target| {
                target.project == project
                    && target.host == query.target_host_id
                    && target.principal == auth.principal
                    && target.app == query.app_id
                    && target.provider == query.provider_id
                    && target.codex.as_ref().map(|codex| &codex.account) == query.account_id.as_ref()
                    && target.kind == kind
            }) {
                let prepared = match service.prepare(&target) {
                    Ok(Some(prepared)) => Ok(prepared),
                    Ok(None) => return Err(AuthErrorCode::NotFound.into()),
                    Err(error) => Err(error),
                };
                // An overlapping grant may authorize the current provider origin.
                if selected.as_ref().is_none_or(|(_, old)| old.is_err() && prepared.is_ok()) {
                    selected = Some((target, prepared));
                }
            }
            let (target, prepared) = selected.ok_or(AuthErrorCode::Forbidden)?;
            Ok::<_, ReadError>((auth.principal, target, prepared.map_err(error_code)?))
        }
    })
    .await
    .map_err(|_| AuthErrorCode::StorageUnavailable)??;

    let operation = Arc::new(RefreshOperation {
        id: Uuid::new_v4(),
        operation: Operation::new(),
        project,
        host,
        principal,
        policy,
        target,
        progress: Mutex::default(),
    });
    let response = AppActionExecutionResult {
        operation_id: TaskId::from(operation.id.to_string()),
        status: TaskStatus::Queued,
    };
    service.operations.admit(operation, service.clone(), prepared)?;
    Ok((StatusCode::ACCEPTED, [(header::CACHE_CONTROL, "no-store")], Json(response)))
}

#[derive(Default)]
pub struct Operations(Mutex<HashMap<Uuid, Arc<RefreshOperation>>>);

impl Operations {
    fn admit(
        &self,
        entry: Arc<RefreshOperation>,
        service: Arc<UsageRefresh>,
        query: Query,
    ) -> Result<(), ReadError> {
        let mut entries = self.0.lock();
        entries.retain(|_, entry| !entry.expired());
        if entries.len() >= MAX_OPERATIONS {
            entries.retain(|_, entry| entry.operation.snapshot().phase != Phase::Terminal);
        }
        if entries.len() >= MAX_OPERATIONS {
            return Err(AuthErrorCode::StorageUnavailable.into());
        }
        let worker = entry.clone();
        entry
            .operation
            .start_task(async move { worker.run(service, query).await })
            .map_err(|_| AuthErrorCode::Conflict)?;
        entries.insert(entry.id, entry);
        Ok(())
    }

    pub fn get(&self, id: &str) -> Option<Arc<RefreshOperation>> {
        let id = Uuid::parse_str(id).ok()?;
        self.0.lock().get(&id).filter(|entry| !entry.expired()).cloned()
    }
}

pub struct RefreshOperation {
    id: Uuid,
    pub operation: Operation,
    project: ProjectScope,
    host: String,
    principal: String,
    policy: PathBuf,
    target: UsageRefreshTarget,
    progress: Mutex<Progress>,
}

#[derive(Default)]
struct Progress {
    finished: Option<Instant>,
    item: Option<Item>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    app_id: vw_api_app_identity::AppId,
    provider_id: String,
    account_id: Option<String>,
    kind: UsageQueryKind,
    status: TaskStatus,
    snapshot_version: Option<String>,
    error_code: Option<AuthErrorCode>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressResponse {
    #[serde(flatten)]
    operation: AppOperationStatus,
    project_id: ProjectId,
    target_host_id: String,
    correlation_id: String,
    cancel_requested: bool,
    items: Vec<Item>,
}

impl RefreshOperation {
    fn expired(&self) -> bool {
        self.progress.lock().finished.is_some_and(|time| time.elapsed() >= RETENTION)
    }

    async fn run(
        &self,
        service: Arc<UsageRefresh>,
        query: Query,
    ) -> Result<(), OperationFailure> {
        let target = self.target.clone();
        let selector = target.selector();
        let mut item = Item {
            app_id: target.app,
            provider_id: selector.provider_id,
            account_id: selector.account_id,
            kind: target.kind,
            status: TaskStatus::Failed,
            snapshot_version: None,
            error_code: None,
        };
        let result = if self.operation.is_cancelled() {
            None
        } else {
            service.refresh(target, query, None, Some(&self.operation)).await
        };
        match result.as_deref() {
            None => item.status = TaskStatus::Cancelled,
            Some(Err(error)) => item.error_code = Some(error_code(*error)),
            Some(Ok(publication)) => {
                item.snapshot_version = Some(publication.version.to_string());
                if matches!(&publication.snapshot, UsageCacheResult::Current(result)
                    if matches!(result.outcome, UsageQueryOutcome::Success { .. }))
                {
                    item.status = TaskStatus::Completed;
                } else {
                    item.error_code = Some(AuthErrorCode::UpstreamFailure);
                }
            }
        }
        let result = match item.status {
            TaskStatus::Cancelled => Err(OperationFailure::Cancelled),
            TaskStatus::Failed => Err(OperationFailure::Failed),
            _ => Ok(()),
        };
        let mut progress = self.progress.lock();
        progress.item = Some(item);
        progress.finished = Some(Instant::now());
        result
    }

    pub async fn authorize(
        self: &Arc<Self>,
        tenant: TenantScope,
        host: &str,
        headers: HeaderMap,
        service: Arc<UsageRefresh>,
        cancel: bool,
    ) -> Result<(), ReadError> {
        if self.project.tenant_scope() != &tenant || self.host != host {
            return Err(AuthErrorCode::NotFound.into());
        }
        let entry = self.clone();
        tokio::task::spawn_blocking(move || {
            let auth = access::authorize_usage_refresh(
                &entry.policy,
                &headers,
                &entry.project,
                &entry.host,
            )
            .map_err(|_| AuthErrorCode::NotFound)?;
            if auth.principal != entry.principal {
                return Err(AuthErrorCode::NotFound.into());
            }
            let targets = access::usage_targets(&entry.policy).map_err(ReadError::access)?;
            if service
                .projects
                .get(&entry.project)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?
                .is_none()
            {
                return Err(AuthErrorCode::NotFound.into());
            }
            entry.authorize_target(&headers, &service, &targets, &entry.target, cancel)
        })
        .await
        .map_err(|_| AuthErrorCode::StorageUnavailable)?
    }

    fn authorize_target(
        &self,
        headers: &HeaderMap,
        service: &UsageRefresh,
        targets: &[UsageRefreshTarget],
        target: &UsageRefreshTarget,
        cancel: bool,
    ) -> Result<(), ReadError> {
        if !targets.contains(target) {
            return Err(AuthErrorCode::Forbidden.into());
        }
        let query = target.selector();
        let tenant = self.project.tenant_scope();
        let auth = access::authorize_usage_read(Some(&self.policy), headers, tenant, &query)
            .map_err(ReadError::access)?;
        if auth.principal != self.principal {
            return Err(AuthErrorCode::NotFound.into());
        }
        if cancel {
            let auth = access::authorize_usage_cancel(&self.policy, headers, tenant, &query)
                .map_err(ReadError::access)?;
            if auth.principal != self.principal {
                return Err(AuthErrorCode::NotFound.into());
            }
        }
        if let Some(scope) = service.codex_scope(target) {
            scope.require_binding(service)?;
        } else {
            app_provider_services::AppProviderStore::usage_record(
                &service.database,
                tenant,
                target.app,
                &target.provider,
            )?;
        }
        Ok(())
    }

    pub fn response(&self) -> Response {
        let operation = lifecycle::status(self.id.to_string(), &self.operation);
        let cancel_requested =
            self.operation.is_cancelled() && self.operation.snapshot().phase != Phase::Terminal;
        let items = self.progress.lock().item.clone().into_iter().collect();
        (
            [(header::CACHE_CONTROL, "no-store")],
            Json(ProgressResponse {
                operation,
                project_id: self.project.project_id().clone(),
                target_host_id: self.host.clone(),
                correlation_id: self.id.to_string(),
                cancel_requested,
                items,
            }),
        )
            .into_response()
    }
}

fn error_code(error: RefreshError) -> AuthErrorCode {
    match error {
        RefreshError::Revoked => AuthErrorCode::Forbidden,
        RefreshError::Unavailable => AuthErrorCode::StorageUnavailable,
        RefreshError::InvalidConfiguration => AuthErrorCode::Conflict,
        RefreshError::NotFound => AuthErrorCode::NotFound,
        RefreshError::Account(code) => code,
    }
}
