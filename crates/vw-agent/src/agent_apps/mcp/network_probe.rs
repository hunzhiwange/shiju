//! Authorized MCP handshakes and revision-bound connection checks.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use axum::http::{HeaderMap, StatusCode};
use tokio::time::{Instant, timeout_at};
use tokio_util::sync::CancellationToken;
use vw_agent_apps::mcp::validation::validate_new_name;
use vw_api_types::{
    agent_apps_diagnostics::{
        DiagnosticEvidenceSource, DiagnosticFailure, DiagnosticFailureCode, DiagnosticJobIdentity,
        DiagnosticOutcome, DiagnosticPhase, DiagnosticProbeKind, DiagnosticReport,
        DiagnosticResult, DiagnosticScope, DiagnosticTiming, McpHandshakeEvidence,
    },
    mcp_management::{
        McpCatalogScope, McpConfigValue, McpConnectionCheckItem, McpConnectionDefinition,
        McpConnectionOutcome, McpFailureCode, McpRecordId, McpTransport,
    },
};

use super::{
    McpCatalogStore, McpExecutionConnection,
    connectivity::{CheckRevision, ConnectionCheckHost},
    runtime::runtime_server,
};
use crate::{
    agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore},
    gateway::{
        agent_apps_diagnostic_jobs::{
            DiagnosticJobError, DiagnosticJobOwner, DiagnosticJobs, DiagnosticProbeError,
            MAX_PROBES_PER_JOB,
        },
        api::handlers::agent_apps::{AuthorizedMcpProbe, McpProbeAccess},
    },
    global_database::{GlobalDatabase, run_sync_database},
    tools::mcp_common::{ConnectionCheckCleanup, open_connection_check_peer},
};

const CHECK_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, PartialEq, Eq)]
pub enum McpHandshakeStartError {
    Access(StatusCode),
    Record(McpFailureCode),
    Job(DiagnosticJobError),
}

/// Submit handshake-only probes to the host registry. Call from a blocking thread in
/// the gateway runtime with the authenticated tenant in `scope` and the host database.
/// Every target must pass authorization and revision checks before any probe is submitted.
pub fn start_handshakes(
    jobs: &DiagnosticJobs,
    database: GlobalDatabase,
    policy: Option<PathBuf>,
    headers: &HeaderMap,
    scope: DiagnosticScope,
    ids: Vec<McpRecordId>,
) -> Result<DiagnosticJobIdentity, McpHandshakeStartError> {
    start_with_source(
        jobs,
        database,
        policy,
        headers,
        scope,
        ids,
        DiagnosticEvidenceSource::RealService,
    )
}

pub(super) fn start_with_source(
    jobs: &DiagnosticJobs,
    database: GlobalDatabase,
    policy: Option<PathBuf>,
    headers: &HeaderMap,
    scope: DiagnosticScope,
    ids: Vec<McpRecordId>,
    source: DiagnosticEvidenceSource,
) -> Result<DiagnosticJobIdentity, McpHandshakeStartError> {
    start_probes(jobs, database, policy, headers, scope, ids, ProbeReport::Handshake(source))
}

/// Managed checks retain the authorized connection revision and caller generation.
pub fn start_connection_checks(
    jobs: &DiagnosticJobs,
    database: GlobalDatabase,
    policy: Option<PathBuf>,
    headers: &HeaderMap,
    scope: DiagnosticScope,
    ids: Vec<McpRecordId>,
    generation: u64,
) -> Result<DiagnosticJobIdentity, McpHandshakeStartError> {
    start_probes(
        jobs,
        database,
        policy,
        headers,
        scope,
        ids,
        ProbeReport::ConnectionCheck { generation },
    )
}

#[derive(Clone, Copy)]
enum ProbeReport {
    Handshake(DiagnosticEvidenceSource),
    ConnectionCheck { generation: u64 },
}

fn start_probes(
    jobs: &DiagnosticJobs,
    database: GlobalDatabase,
    policy: Option<PathBuf>,
    headers: &HeaderMap,
    scope: DiagnosticScope,
    ids: Vec<McpRecordId>,
    report: ProbeReport,
) -> Result<DiagnosticJobIdentity, McpHandshakeStartError> {
    use McpHandshakeStartError::{Access, Job, Record};

    let mut unique = std::collections::HashSet::new();
    if ids.is_empty()
        || ids.len() > MAX_PROBES_PER_JOB
        || !ids.iter().all(|id| {
            validate_new_name(&id.0, &[]).is_ok_and(|valid| valid == *id) && unique.insert(id)
        })
    {
        return Err(Job(DiagnosticJobError::InvalidInput));
    }
    let access = McpProbeAccess::new(policy, headers).map_err(Access)?;
    let grants = ids
        .into_iter()
        .map(|id| {
            let grant = access.authorize(&scope, &id)?;
            Ok((id, grant))
        })
        .collect::<Result<Vec<_>, StatusCode>>()
        .map_err(Access)?;
    let principal =
        grants.first().ok_or(Job(DiagnosticJobError::InvalidInput))?.1.principal.clone();
    if grants.iter().any(|(_, grant)| grant.principal != principal) {
        return Err(Access(StatusCode::FORBIDDEN));
    }
    let catalog = run_sync_database(|| Ok(McpCatalogStore::new(database.clone())))
        .map_err(|_| Record(McpFailureCode::StorageUnavailable))?
        .map_err(Record)?;
    let context = Arc::new(ProbeContext { access, scope, catalog, database });
    let probes = grants
        .into_iter()
        .map(|(id, grant)| {
            let probe = McpHandshakeProbe { context: context.clone(), id, grant };
            probe.authorize_record().map_err(Record)?;
            Ok(move |cancel| async move {
                match report {
                    ProbeReport::Handshake(source) => probe.run(cancel, source).await,
                    ProbeReport::ConnectionCheck { generation } => {
                        probe.run_connection_check(cancel, generation).await
                    }
                }
            })
        })
        .collect::<Result<Vec<_>, McpHandshakeStartError>>()?;
    let owner = DiagnosticJobOwner { scope: context.scope.clone(), principal };
    jobs.start(owner, DiagnosticProbeKind::Mcp, probes).map_err(Job)
}

struct ProbeContext {
    access: McpProbeAccess,
    scope: DiagnosticScope,
    catalog: McpCatalogStore,
    database: GlobalDatabase,
}

struct McpHandshakeProbe {
    context: Arc<ProbeContext>,
    id: McpRecordId,
    grant: AuthorizedMcpProbe,
}

impl McpHandshakeProbe {
    fn catalog_scope(&self) -> McpCatalogScope {
        self.grant.permit.catalog_scope(&self.context.scope)
    }

    fn authorize_record(&self) -> Result<(McpTransport, bool), McpFailureCode> {
        if !self
            .context
            .access
            .authorize(&self.context.scope, &self.id)
            .is_ok_and(|current| current == self.grant)
        {
            return Err(McpFailureCode::PermissionDenied);
        }
        let stored = run_sync_database(|| {
            Ok(self.context.catalog.active_record(&self.catalog_scope(), &self.id))
        })
        .map_err(|_| McpFailureCode::StorageUnavailable)??;
        if stored.record.connection_revision != self.grant.permit.connection_revision {
            return Err(McpFailureCode::StaleRevision);
        }
        if let McpConnectionDefinition::Stdio { env, .. } = &stored.connection {
            if !self.grant.permit.permits_stdio(&stored.connection) {
                return Err(McpFailureCode::PermissionDenied);
            }
            let mut needs_credentials = false;
            for value in env.values() {
                if let McpConfigValue::Credential { reference } = value {
                    if self.grant.credential_root.is_none()
                        || !self.grant.permit.permits_credential(&stored.connection, reference)
                    {
                        return Err(McpFailureCode::PermissionDenied);
                    }
                    needs_credentials = true;
                }
            }
            return Ok((stored.record.transport, needs_credentials));
        }
        let (url, headers) = match &stored.connection {
            McpConnectionDefinition::StreamableHttp { url, headers }
            | McpConnectionDefinition::Sse { url, headers } => (url, headers),
            _ => return Err(McpFailureCode::InvalidDefinition),
        };
        if !self.grant.permit.permits_endpoint(url) {
            return Err(McpFailureCode::PermissionDenied);
        }
        let mut needs_credentials = false;
        for (name, value) in headers {
            if name.eq_ignore_ascii_case("host") || name.eq_ignore_ascii_case("proxy-authorization")
            {
                return Err(McpFailureCode::InvalidDefinition);
            }
            if let McpConfigValue::Credential { reference } = value {
                if self.grant.credential_root.is_none()
                    || !self.grant.permit.permits_credential(&stored.connection, reference)
                {
                    return Err(McpFailureCode::PermissionDenied);
                }
                needs_credentials = true;
            }
        }
        Ok((stored.record.transport, needs_credentials))
    }

    async fn run_connection_check(
        self,
        cancel: CancellationToken,
        generation: u64,
    ) -> Result<DiagnosticResult, DiagnosticProbeError> {
        let scope = self.catalog_scope();
        let outcome = async {
            let (_, needs_credentials) = self.authorize_record()?;
            let accounts = if needs_credentials {
                let root =
                    self.grant.credential_root.as_ref().ok_or(McpFailureCode::PermissionDenied)?;
                Some(
                    run_sync_database(|| {
                        Ok(AuthAccountStore::new(self.context.database.clone(), root))
                    })
                    .map_err(|_| McpFailureCode::CredentialUnavailable)?
                    .map_err(|_| McpFailureCode::CredentialUnavailable)?,
                )
            } else {
                None
            };
            let authorize = || self.authorize_record().map(|_| ());
            let host = &self.context.scope.target_host_id;
            let workspace = self.grant.permit.stdio_workspace.as_deref().unwrap_or(Path::new("/"));
            Ok::<_, McpFailureCode>(
                self.context
                    .catalog
                    .check_connection_inner(
                        ConnectionCheckHost {
                            scope: &scope,
                            execution_host: host,
                            workspace,
                            accounts: accounts.as_ref(),
                            authorize: &authorize,
                        },
                        &self.id,
                        CheckRevision::Connection(self.grant.permit.connection_revision),
                        Some(&cancel),
                        |definition, reference| {
                            self.grant.permit.permits_credential(definition, reference).then_some(
                                AuthAccountScope {
                                    tenant: self.context.scope.project.tenant_scope(),
                                    principal: &self.grant.principal,
                                    host,
                                    service: reference.service,
                                },
                            )
                        },
                    )
                    .await,
            )
        }
        .await
        .unwrap_or_else(|code| McpConnectionOutcome::Failed { code });
        if cancel.is_cancelled() {
            return Err(DiagnosticProbeError::Cancelled);
        }
        Ok(DiagnosticResult::McpConnectionCheck {
            scope,
            generation,
            item: McpConnectionCheckItem {
                id: self.id,
                connection_revision: self.grant.permit.connection_revision,
                outcome,
            },
        })
    }

    async fn run(
        self,
        cancel: CancellationToken,
        source: DiagnosticEvidenceSource,
    ) -> Result<DiagnosticResult, DiagnosticProbeError> {
        let start = Instant::now();
        let deadline = start + CHECK_TIMEOUT;
        let http_io = CancellationToken::new();
        let _close_io = http_io.clone().drop_guard();
        let cleanup = ConnectionCheckCleanup::default();
        let _close_cleanup = cleanup.cancellation().clone().drop_guard();
        let mut running = None;
        let mut child = None;
        let handshake = async {
            let (transport, needs_credentials) = self.authorize_record()?;
            let accounts = if needs_credentials {
                let root =
                    self.grant.credential_root.as_ref().ok_or(McpFailureCode::PermissionDenied)?;
                Some(
                    run_sync_database(|| {
                        Ok(AuthAccountStore::new(self.context.database.clone(), root))
                    })
                    .map_err(|_| McpFailureCode::CredentialUnavailable)?
                    .map_err(|_| McpFailureCode::CredentialUnavailable)?,
                )
            } else {
                None
            };
            let scope = self.catalog_scope();
            let host = &self.context.scope.target_host_id;
            let (mut connection, _) = self
                .context
                .catalog
                .resolve_execution_connection(
                    &scope,
                    &self.id,
                    |record| record.connection_revision == self.grant.permit.connection_revision,
                    host,
                    accounts.as_ref(),
                    |definition, reference| {
                        self.grant.permit.permits_credential(definition, reference).then_some(
                            AuthAccountScope {
                                tenant: self.context.scope.project.tenant_scope(),
                                principal: &self.grant.principal,
                                host,
                                service: reference.service,
                            },
                        )
                    },
                )
                .await?;
            self.authorize_record()?;
            if Instant::now() >= deadline {
                return Err(McpFailureCode::TimedOut);
            }
            if let McpExecutionConnection::Stdio { cwd, .. } = &mut connection {
                cwd.get_or_insert_with(|| ".".to_owned());
            }
            let workspace = self.grant.permit.stdio_workspace.as_deref().unwrap_or(Path::new("/"));
            let server = runtime_server(self.id.0.clone(), connection, workspace)
                .map_err(|_| McpFailureCode::InvalidDefinition)?;
            running = Some(
                open_connection_check_peer(
                    server,
                    deadline.saturating_duration_since(Instant::now()),
                    http_io.clone(),
                    Some(&mut child),
                    Some(cleanup.clone()),
                )
                .await
                .map_err(|_| McpFailureCode::ConnectionFailed)?,
            );
            Ok::<_, McpFailureCode>(transport)
        };
        let result = tokio::select! {
            biased;
            () = cancel.cancelled() => None,
            result = timeout_at(deadline, handshake) => Some(result),
        };
        http_io.cancel();
        // Finish process-group cleanup before returning the scheduler's permit, even
        // when cancellation dropped initialization before rmcp returned a peer.
        if let Some(child) = child {
            child.kill_and_reap().await;
        }
        let authorized = matches!(&result, Some(Ok(Ok(_)))) && self.authorize_record().is_ok();
        if !authorized || cancel.is_cancelled() {
            cleanup.cancellation().cancel();
        }
        if let Some(peer) = running {
            let _ = timeout_at(deadline, peer.cancel()).await;
        }
        if authorized {
            tokio::select! {
                biased;
                () = cancel.cancelled() => cleanup.cancellation().cancel(),
                _ = timeout_at(deadline, cleanup.wait()) => cleanup.cancellation().cancel(),
            }
        }
        let Some(result) = result.filter(|_| !cancel.is_cancelled()) else {
            return Err(DiagnosticProbeError::Cancelled);
        };
        let outcome = match result {
            _ if Instant::now() >= deadline => failed(McpFailureCode::TimedOut),
            Ok(Ok(transport)) => match self.authorize_record() {
                Ok(_) => DiagnosticOutcome::Succeeded {
                    evidence: McpHandshakeEvidence { transport, http_status: None },
                },
                Err(code) => failed(code),
            },
            Ok(Err(code)) => failed(code),
            Err(_) => failed(McpFailureCode::TimedOut),
        };
        Ok(DiagnosticResult::Mcp {
            server_id: self.id,
            report: DiagnosticReport {
                source,
                timings: vec![DiagnosticTiming {
                    phase: DiagnosticPhase::Handshake,
                    elapsed_ms: start.elapsed().as_millis().try_into().unwrap_or(u64::MAX),
                }],
                outcome,
            },
        })
    }
}

fn failed(code: McpFailureCode) -> DiagnosticOutcome<McpHandshakeEvidence> {
    let code = match code {
        McpFailureCode::PermissionDenied
        | McpFailureCode::CredentialUnavailable
        | McpFailureCode::StaleRevision
        | McpFailureCode::NotFound => DiagnosticFailureCode::Unauthorized,
        McpFailureCode::TimedOut => DiagnosticFailureCode::Timeout,
        McpFailureCode::InvalidDefinition => DiagnosticFailureCode::Unsupported,
        _ => DiagnosticFailureCode::Transport,
    };
    DiagnosticOutcome::Failed {
        error: DiagnosticFailure { phase: DiagnosticPhase::Handshake, code },
        http_status: None,
    }
}
