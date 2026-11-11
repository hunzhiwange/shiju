//! Saved MCP intent reconciliation on the host operation executor.
use super::{
    agent_apps,
    mcp_management_read::{McpHttpError, PolicyFile, access_error, unavailable},
};
use crate::global_database::GlobalDatabase;
use axum::{
    Extension, Json,
    extract::rejection::JsonRejection,
    http::{HeaderMap, StatusCode},
};
use serde::Deserialize;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use vw_agent_apps::apps::lifecycle::operations::Operation;
#[cfg(target_os = "macos")]
use vw_agent_apps::apps::lifecycle::operations::Phase;
use vw_api_types::{
    ProjectScope, TenantScope,
    agent_apps::{AppId, AppInstallationId},
    mcp_management::*,
};

#[derive(Clone, Default)]
pub(super) struct Jobs(Arc<Mutex<HashMap<String, Arc<Job>>>>);
pub(super) struct Job {
    pub(super) operation: Operation,
    tenant: TenantScope,
    scope: McpCatalogScope,
    host: String,
    reports: Mutex<Vec<TargetResult>>,
    outcome: Mutex<Option<McpBatchOutcome>>,
}
type TargetResult = McpResyncTargetResult;
impl Job {
    pub(super) fn response(&self, id: String) -> axum::response::Response {
        use axum::response::IntoResponse;
        let result = (|| {
            let snapshot = McpResyncOperationStatus {
                operation: agent_apps::lifecycle::status(id, &self.operation),
                targets: self.reports.lock().map_err(unavailable)?.clone(),
                outcome: *self.outcome.lock().map_err(unavailable)?,
            };
            Ok::<_, McpHttpError>(Json(snapshot))
        })();
        ([(axum::http::header::CACHE_CONTROL, "no-store")], result).into_response()
    }
}
impl Jobs {
    pub(super) fn owned(
        &self,
        id: &str,
        tenant: &TenantScope,
        host: &str,
        headers: &HeaderMap,
        policy: &PolicyFile,
    ) -> Result<Option<Arc<Job>>, StatusCode> {
        let entry = self.0.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?.get(id).cloned();
        let Some(entry) = entry else {
            return Ok(None);
        };
        if entry.tenant != *tenant || entry.host != host {
            return Err(StatusCode::NOT_FOUND);
        }
        authorize(policy, headers, &entry.scope, tenant, host).map_err(|e| e.0)?;
        Ok(Some(entry))
    }
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ResyncRequest {
    target_host_id: String,
    project_id: Option<String>,
    targets: Vec<Target>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Target {
    app_id: AppId,
    installation_id: AppInstallationId,
    account_id: Option<String>,
}
fn authorize(
    policy: &PolicyFile,
    headers: &HeaderMap,
    scope: &McpCatalogScope,
    tenant: &TenantScope,
    host: &str,
) -> Result<(), McpHttpError> {
    agent_apps::authorize_mcp_catalog_write(policy.0.as_deref(), headers, scope, host)
        .map_err(access_error)?;
    agent_apps::authorize_provider_config_access(policy.0.as_deref(), headers, tenant, host, true)
        .map_err(access_error)
}
pub(super) async fn submit(
    Extension(policy): Extension<PolicyFile>,
    Extension(tenant): Extension<TenantScope>,
    Extension(jobs): Extension<Jobs>,
    headers: HeaderMap,
    database: Option<Extension<GlobalDatabase>>,
    request: Result<Json<ResyncRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<vw_api_types::agent_apps::AppActionExecutionResult>), McpHttpError> {
    let Json(request) = request.map_err(|_| access_error(StatusCode::BAD_REQUEST))?;
    let scope = match request.project_id.as_ref() {
        Some(id) => McpCatalogScope::Project {
            scope: ProjectScope::new(tenant.clone(), id.clone().into())
                .map_err(|_| access_error(StatusCode::BAD_REQUEST))?,
        },
        None => McpCatalogScope::Tenant { scope: tenant.clone() },
    };
    authorize(&policy, &headers, &scope, &tenant, &request.target_host_id)?;
    if request.targets.is_empty() || request.targets.len() > 12 {
        return Err(access_error(StatusCode::BAD_REQUEST));
    }
    let mut seen = Vec::new();
    for target in &request.targets {
        if target.installation_id.0.is_empty()
            || seen.contains(&target.app_id)
            || matches!(target.app_id, AppId::ClaudeDesktop | AppId::OpenClaw)
            || (target.app_id == AppId::Claude) != target.account_id.is_some()
        {
            return Err(access_error(StatusCode::BAD_REQUEST));
        }
        seen.push(target.app_id);
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (database, jobs);
        Err(access_error(StatusCode::NOT_IMPLEMENTED))
    }
    #[cfg(target_os = "macos")]
    {
        let database = super::mcp_management_read::open_database(database)?;
        let id = uuid::Uuid::new_v4().to_string();
        let job = Arc::new(Job {
            operation: Operation::new(),
            tenant: tenant.clone(),
            scope,
            host: request.target_host_id.clone(),
            reports: Mutex::new(Vec::new()),
            outcome: Mutex::new(None),
        });
        let mut registry = jobs.0.lock().map_err(unavailable)?;
        if registry.len() >= 1024 {
            registry.retain(|_, job| job.operation.snapshot().phase != Phase::Terminal);
        }
        if registry.len() >= 1024 {
            return Err(access_error(StatusCode::TOO_MANY_REQUESTS));
        }
        let worker = job.clone();
        job.operation
            .start_task(async move {
                // Discovery and native transactions are blocking. Keep them off HTTP workers.
                let runtime = tokio::runtime::Handle::current();
                tokio::task::spawn_blocking(move || {
                    runtime.block_on(run(worker, policy, headers, request, database))
                })
                .await
                .map_err(|_| vw_agent_apps::apps::lifecycle::operations::OperationFailure::Failed)?
            })
            .map_err(unavailable)?;
        registry.insert(id.clone(), job);
        Ok((
            StatusCode::ACCEPTED,
            Json(vw_api_types::agent_apps::AppActionExecutionResult {
                operation_id: id.into(),
                status: vw_api_types::task::TaskStatus::Queued,
            }),
        ))
    }
}

#[cfg(target_os = "macos")]
async fn run(
    job: Arc<Job>,
    policy: PolicyFile,
    headers: HeaderMap,
    request: ResyncRequest,
    database: GlobalDatabase,
) -> Result<(), vw_agent_apps::apps::lifecycle::operations::OperationFailure> {
    use crate::agent_apps::{
        auth_accounts::{AuthAccountScope, AuthAccountStore},
        mcp::{McpBindingTarget, McpCatalogStore, McpResyncTask},
    };
    use vw_agent_apps::{
        apps::discovery::read,
        apps::lifecycle::operations::OperationFailure,
        mcp::import::NativeMcpSource,
        providers::live::commit::{ExecutionHostId, ProviderTransactions},
    };
    use vw_api_types::agent_apps_auth::AuthService;
    let work = async {
        authorize(&policy, &headers, &job.scope, &job.tenant, &job.host).map_err(|e| e.1.0.code)?;
        let (env, root) = agent_apps::authorize_mcp_native_write(
            policy.0.as_deref(),
            &headers,
            &job.tenant,
            &job.host,
        )
        .map_err(|_| McpFailureCode::PermissionDenied)?;
        let store = McpCatalogStore::new(database.clone())?;
        let accounts = AuthAccountStore::new(database, &root)
            .map_err(|_| McpFailureCode::StorageUnavailable)?;
        let transactions = ProviderTransactions::new(
            &root,
            ExecutionHostId::new(&job.host).map_err(|_| McpFailureCode::TargetUnavailable)?,
        )
        .map_err(|_| McpFailureCode::StorageUnavailable)?;
        let mut authorized = Vec::new();
        for service in [
            AuthService::GithubCopilot,
            AuthService::CodexOauth,
            AuthService::XaiOauth,
            AuthService::KimiOauth,
            AuthService::ClaudeOfficial,
            AuthService::AgyOauth,
            AuthService::DshAccount,
        ] {
            if let Ok(owner) = agent_apps::authorize_auth_binding(
                policy.0.as_deref(),
                &headers,
                &job.tenant,
                &job.host,
                service,
            ) && owner.credential_root == root
            {
                authorized.push((service, owner));
            }
        }
        let mut resolved = Vec::new();
        for target in &request.targets {
            let app_target = McpAppTarget {
                app_id: target.app_id,
                installation_id: target.installation_id.clone(),
                target_host_id: job.host.clone(),
            };
            let installation = read::read(target.app_id, &env)
                .map_err(|_| McpFailureCode::TargetUnavailable)
                .and_then(|read| {
                    read.installations
                        .into_iter()
                        .find(|i| i.id == target.installation_id)
                        .ok_or(McpFailureCode::NotFound)
                });
            let lease = if let Some(id) = &target.account_id {
                (|| {
                    let owner = agent_apps::authorize_auth_account_read(
                        policy.0.as_deref(),
                        &headers,
                        &job.tenant,
                        &job.host,
                        AuthService::ClaudeOfficial,
                        false,
                    )
                    .map_err(|_| McpFailureCode::PermissionDenied)?;
                    if owner.credential_root != root {
                        return Err(McpFailureCode::PermissionDenied);
                    }
                    accounts
                        .managed_official_lease(
                            AuthAccountScope {
                                tenant: &job.tenant,
                                host: &owner.id,
                                principal: &owner.principal,
                                service: AuthService::ClaudeOfficial,
                            },
                            id,
                        )
                        .map(Some)
                        .map_err(|_| McpFailureCode::CredentialUnavailable)
                })()
            } else {
                Ok(None)
            };
            resolved.push((app_target, installation, lease));
        }
        let requested = request
            .targets
            .iter()
            .map(|target| {
                serde_json::to_value(target.app_id)
                    .ok()
                    .and_then(|value| value.as_str().map(str::to_owned))
                    .ok_or(McpFailureCode::InvalidDefinition)
            })
            .collect::<Result<Vec<_>, _>>()?;
        store
            .resync_apps(
                &job.scope,
                &requested,
                |app| {
                    // Revalidate grants immediately before each application's publication.
                    authorize(&policy, &headers, &job.scope, &job.tenant, &job.host)
                        .map_err(|_| McpFailureCode::PermissionDenied)?;
                    let (fresh, fresh_root) = agent_apps::authorize_mcp_native_write(
                        policy.0.as_deref(),
                        &headers,
                        &job.tenant,
                        &job.host,
                    )
                    .map_err(|_| McpFailureCode::PermissionDenied)?;
                    if fresh.home != env.home || fresh_root != root {
                        return Err(McpFailureCode::TargetUnavailable);
                    }
                    let (target, installation, lease) = resolved
                        .iter()
                        .find(|(target, _, _)| target.app_id == app)
                        .ok_or(McpFailureCode::NotFound)?;
                    let installation = installation.as_ref().map_err(|code| *code)?;
                    let lease = lease.as_ref().map_err(|code| *code)?;
                    Ok(McpBindingTarget {
                        target,
                        installation,
                        source: lease.as_ref().map_or(
                            NativeMcpSource::HostHome(&env.home),
                            NativeMcpSource::ClaudeAccount,
                        ),
                        transactions: &transactions,
                    })
                },
                &accounts,
                |_, _, reference| {
                    authorized
                        .iter()
                        .find(|(service, owner)| {
                            *service == reference.service
                                && agent_apps::authorize_auth_binding(
                                    policy.0.as_deref(),
                                    &headers,
                                    &job.tenant,
                                    &job.host,
                                    *service,
                                )
                                .is_ok_and(|fresh| {
                                    fresh.id == owner.id
                                        && fresh.principal == owner.principal
                                        && fresh.credential_root == root
                                })
                        })
                        .map(|(service, owner)| AuthAccountScope {
                            tenant: &job.tenant,
                            host: &owner.id,
                            principal: &owner.principal,
                            service: *service,
                        })
                },
                McpResyncTask {
                    operation: &job.operation,
                    on_target: |target: &crate::agent_apps::mcp::McpResyncTargetResult, _, _| {
                        if let Ok(mut reports) = job.reports.lock() {
                            reports.push(match &target.distribution {
                                Ok(distribution) => TargetResult {
                                    app_id: target.app_id,
                                    distribution: Some(distribution.clone()),
                                    code: None,
                                },
                                Err(code) => TargetResult {
                                    app_id: target.app_id,
                                    distribution: None,
                                    code: Some(*code),
                                },
                            });
                        }
                    },
                },
            )
            .await
    }
    .await;
    let (outcome, reports) = match work {
        Ok(result) => (
            result.outcome,
            result
                .targets
                .into_iter()
                .map(|target| match target.distribution {
                    Ok(distribution) => TargetResult {
                        app_id: target.app_id,
                        distribution: Some(distribution),
                        code: None,
                    },
                    Err(code) => {
                        TargetResult { app_id: target.app_id, distribution: None, code: Some(code) }
                    }
                })
                .collect(),
        ),
        Err(code) => (
            McpBatchOutcome::Failed,
            request
                .targets
                .iter()
                .map(|target| TargetResult {
                    app_id: target.app_id,
                    distribution: None,
                    code: Some(code),
                })
                .collect(),
        ),
    };
    *job.reports.lock().map_err(|_| OperationFailure::Failed)? = reports;
    *job.outcome.lock().map_err(|_| OperationFailure::Failed)? = Some(outcome);
    match outcome {
        McpBatchOutcome::Succeeded => Ok(()),
        McpBatchOutcome::Cancelled => Err(OperationFailure::Cancelled),
        _ => Err(OperationFailure::Failed),
    }
}
