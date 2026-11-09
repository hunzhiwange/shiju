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
use futures_util::{StreamExt, stream};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use vw_agent_apps::{
    apps::lifecycle::operations::{Operation, OperationFailure, Phase},
    providers::validate_identity,
    usage::cache::UsageCacheResult,
};
use vw_api_types::{
    agent_apps::{AppActionExecutionResult, AppId, AppOperationStatus},
    agent_apps_auth::AuthErrorCode,
    apps_usage::{UsageQueryKind, UsageQueryOutcome, UsageSnapshotQuery},
    id::{ProjectId, ProjectScope, TaskId, TenantScope},
    task::TaskStatus,
};

use super::{
    access::{self, UsageRefreshTarget},
    lifecycle,
    usage_read::ReadError,
    usage_refresh::{CONCURRENCY, Query, RefreshError, UsageRefresh},
};

const MAX_BATCH: usize = 32;
const MAX_OPERATIONS: usize = 128;
const RETENTION: Duration = Duration::from_secs(10 * 60);

#[derive(Clone)]
struct Policy(Option<PathBuf>);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Batch {
    project_id: ProjectId,
    target_host_id: String,
}

pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/apps/usage/refresh", post(single))
        .route("/apps/usage/refresh-all", post(batch))
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
    let scope = Batch {
        project_id: query.project_id.clone(),
        target_host_id: query.target_host_id.clone(),
    };
    submit(policy, tenant, headers, service, scope, Some(query)).await
}

async fn batch(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    headers: HeaderMap,
    service: Option<Extension<Arc<UsageRefresh>>>,
    input: Result<Json<Batch>, JsonRejection>,
) -> Result<impl IntoResponse, ReadError> {
    let Json(scope) = input.map_err(|_| AuthErrorCode::InvalidInput)?;
    submit(policy, tenant, headers, service, scope, None).await
}

async fn submit(
    policy: Policy,
    tenant: TenantScope,
    headers: HeaderMap,
    service: Option<Extension<Arc<UsageRefresh>>>,
    scope: Batch,
    query: Option<UsageSnapshotQuery>,
) -> Result<impl IntoResponse, ReadError> {
    let project =
        ProjectScope::new(tenant, scope.project_id).map_err(|_| AuthErrorCode::InvalidInput)?;
    validate_identity(&scope.target_host_id).map_err(|_| AuthErrorCode::InvalidInput)?;
    let Extension(service) = service.ok_or(AuthErrorCode::StorageUnavailable)?;
    let policy = policy.0.unwrap_or_else(|| access::tenant_policy_path(project.tenant_scope()));
    let (principal, items) = tokio::task::spawn_blocking({
        let project = project.clone();
        let service = service.clone();
        let host = scope.target_host_id.clone();
        let policy = policy.clone();
        move || {
            let batch = query.is_none();
            let auth = access::authorize_usage_refresh(&policy, &headers, &project, &host, batch)
                .map_err(ReadError::access)?;
            if let Some(query) = &query {
                let supported = if query.account_id.is_some() {
                    query.app_id == AppId::Codex
                        && query
                            .kind
                            .is_none_or(|kind| kind == UsageQueryKind::OfficialSubscription)
                } else {
                    query.kind.is_none_or(|kind| kind == UsageQueryKind::CustomScript)
                };
                if !supported {
                    return Err(AuthErrorCode::Unsupported.into());
                }
            }
            let targets = access::usage_targets(&policy, batch).map_err(ReadError::access)?;
            let mut items: PreparedItems = Vec::new();
            for target in targets.into_iter().filter(|target| {
                target.project == project
                    && target.host == host
                    && target.principal == auth.principal
                    && query.as_ref().is_none_or(|query| {
                        target.app == query.app_id
                            && target.provider == query.provider_id
                            && target.codex.as_ref().map(|codex| &codex.account)
                                == query.account_id.as_ref()
                    })
            }) {
                let prepared = match service.prepare(&target) {
                    Ok(Some(prepared)) => Ok(prepared),
                    Ok(None)
                    | Err(
                        RefreshError::NotFound
                        | RefreshError::Account(AuthErrorCode::NotFound | AuthErrorCode::Forbidden),
                    ) if batch => continue,
                    Ok(None) => return Err(AuthErrorCode::NotFound.into()),
                    Err(error) => Err(error),
                };
                // Accept an authorized origin that matches the current profile.
                // Retain at most one result per selector across overlapping grants.
                if let Some(existing) = items.iter_mut().find(|(existing, _)| {
                    existing.app == target.app
                        && existing.provider == target.provider
                        && existing.codex == target.codex
                }) {
                    if existing.1.is_err() && prepared.is_ok() {
                        *existing = (target, prepared);
                    }
                    continue;
                }
                items.push((target, prepared));
                if items.len() > MAX_BATCH {
                    return Err(AuthErrorCode::Conflict.into());
                }
            }
            if items.is_empty() {
                return Err(
                    if batch { AuthErrorCode::NotFound } else { AuthErrorCode::Forbidden }.into()
                );
            }
            if !batch && let Err(error) = &items[0].1 {
                return Err(error_code(*error).into());
            }
            Ok::<_, ReadError>((auth.principal, items))
        }
    })
    .await
    .map_err(|_| AuthErrorCode::StorageUnavailable)??;

    let operation = Arc::new(RefreshOperation {
        id: Uuid::new_v4(),
        operation: Operation::new(),
        project,
        host: scope.target_host_id,
        principal,
        policy,
        targets: items.iter().map(|(target, _)| target.clone()).collect(),
        progress: Mutex::default(),
    });
    let response = AppActionExecutionResult {
        operation_id: TaskId::from(operation.id.to_string()),
        status: TaskStatus::Queued,
    };
    service.operations.admit(operation, service.clone(), items)?;
    Ok((StatusCode::ACCEPTED, [(header::CACHE_CONTROL, "no-store")], Json(response)))
}

type PreparedItems = Vec<(UsageRefreshTarget, Result<Query, RefreshError>)>;

#[derive(Default)]
pub(super) struct Operations(Mutex<HashMap<Uuid, Arc<RefreshOperation>>>);

impl Operations {
    fn admit(
        &self,
        entry: Arc<RefreshOperation>,
        service: Arc<UsageRefresh>,
        items: PreparedItems,
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
            .start_task(async move { worker.run(service, items).await })
            .map_err(|_| AuthErrorCode::Conflict)?;
        entries.insert(entry.id, entry);
        Ok(())
    }

    pub(super) fn get(&self, id: &str) -> Option<Arc<RefreshOperation>> {
        let id = Uuid::parse_str(id).ok()?;
        self.0.lock().get(&id).filter(|entry| !entry.expired()).cloned()
    }
}

pub(super) struct RefreshOperation {
    id: Uuid,
    pub(super) operation: Operation,
    project: ProjectScope,
    host: String,
    principal: String,
    policy: PathBuf,
    targets: Vec<UsageRefreshTarget>,
    progress: Mutex<Progress>,
}

#[derive(Default)]
struct Progress {
    finished: Option<Instant>,
    items: Vec<Item>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Item {
    app_id: vw_api_types::agent_apps::AppId,
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
        items: PreparedItems,
    ) -> Result<(), OperationFailure> {
        let mut updates = stream::iter(items)
            .map(|(target, prepared)| {
                let service = &service;
                async move {
                    let selector = target.selector();
                    let mut item = Item {
                        app_id: target.app,
                        provider_id: selector.provider_id,
                        account_id: selector.account_id,
                        kind: selector.kind.expect("refresh target always selects one query kind"),
                        status: TaskStatus::Failed,
                        snapshot_version: None,
                        error_code: None,
                    };
                    if self.operation.is_cancelled() {
                        item.status = TaskStatus::Cancelled;
                        return item;
                    }
                    let result = match prepared {
                        Ok(query) => {
                            service.refresh(target, query, None, Some(&self.operation)).await
                        }
                        Err(error) => {
                            item.error_code = Some(error_code(error));
                            return item;
                        }
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
                    item
                }
            })
            .buffer_unordered(CONCURRENCY);
        let mut cancelled = false;
        let mut failed = false;
        while let Some(item) = updates.next().await {
            cancelled |= item.status == TaskStatus::Cancelled;
            failed |= item.status == TaskStatus::Failed;
            self.progress.lock().items.push(item);
        }
        self.progress.lock().finished = Some(Instant::now());
        if cancelled {
            Err(OperationFailure::Cancelled)
        } else if failed {
            Err(OperationFailure::Failed)
        } else {
            Ok(())
        }
    }

    pub(super) async fn authorize(
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
            let batch = entry.targets.first().is_some_and(|target| target.batch);
            let auth = access::authorize_usage_refresh(
                &entry.policy,
                &headers,
                &entry.project,
                &entry.host,
                batch,
            )
            .map_err(|_| AuthErrorCode::NotFound)?;
            if auth.principal != entry.principal {
                return Err(AuthErrorCode::NotFound.into());
            }
            let targets = access::usage_targets(&entry.policy, batch).map_err(ReadError::access)?;
            if service
                .projects
                .get(&entry.project)
                .map_err(|_| AuthErrorCode::StorageUnavailable)?
                .is_none()
            {
                return Err(AuthErrorCode::NotFound.into());
            }
            for target in &entry.targets {
                entry.authorize_target(&headers, &service, &targets, target, cancel)?;
            }
            Ok(())
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
            crate::app_providers::AppProviderStore::usage_record(
                &service.database,
                tenant,
                target.app,
                &target.provider,
            )?;
        }
        Ok(())
    }

    pub(super) fn response(&self) -> Response {
        let operation = lifecycle::status(self.id.to_string(), &self.operation);
        let cancel_requested =
            self.operation.is_cancelled() && self.operation.snapshot().phase != Phase::Terminal;
        let items = self.progress.lock().items.clone();
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
