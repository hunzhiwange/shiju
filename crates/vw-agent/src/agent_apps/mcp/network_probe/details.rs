//! Explicit discovery uses the same narrow permit, transport and cleanup as connection checks.
use super::*;
use vw_api_types::mcp_management::{McpToolDetails, McpToolDetailsRequest};

pub fn start_details(
    jobs: &DiagnosticJobs,
    database: GlobalDatabase,
    policy: Option<PathBuf>,
    headers: &HeaderMap,
    scope: DiagnosticScope,
    request: McpToolDetailsRequest,
) -> Result<DiagnosticJobIdentity, McpHandshakeStartError> {
    use McpHandshakeStartError::{Access, Job, Record};
    if request.record.revision == 0
        || request.connection_revision == 0
        || !validate_new_name(&request.record.id.0, &[]).is_ok_and(|id| id == request.record.id)
    {
        return Err(Job(DiagnosticJobError::InvalidInput));
    }
    let access = McpProbeAccess::new(policy, headers).map_err(Access)?;
    let grant = access.authorize_details(&scope, &request.record.id).map_err(Access)?;
    if request.scope != grant.permit.catalog_scope(&scope)
        || request.connection_revision != grant.permit.connection_revision
    {
        return Err(Record(McpFailureCode::PermissionDenied));
    }
    let principal = grant.principal.clone();
    let catalog = run_sync_database(|| Ok(McpCatalogStore::new(database.clone())))
        .map_err(|_| Record(McpFailureCode::StorageUnavailable))?
        .map_err(Record)?;
    let context = Arc::new(ProbeContext { access, scope, catalog, database });
    let probe =
        McpHandshakeProbe { context: context.clone(), id: request.record.id.clone(), grant };
    probe.authorize_details(&request).map_err(Record)?;
    let owner = DiagnosticJobOwner { scope: context.scope.clone(), principal };
    jobs.start(
        owner,
        DiagnosticProbeKind::Mcp,
        vec![move |cancel| async move { probe.discover_details(request, cancel).await }],
    )
    .map_err(Job)
}

impl McpHandshakeProbe {
    fn authorize_details(&self, request: &McpToolDetailsRequest) -> Result<bool, McpFailureCode> {
        if !self
            .context
            .access
            .authorize_details(&self.context.scope, &self.id)
            .is_ok_and(|grant| grant == self.grant)
        {
            return Err(McpFailureCode::PermissionDenied);
        }
        let (_, credentials) = self.authorize_record()?;
        let current =
            run_sync_database(|| Ok(self.context.catalog.active_record(&request.scope, &self.id)))
                .map_err(|_| McpFailureCode::StorageUnavailable)??;
        if current.record.record != request.record
            || current.record.connection_revision != request.connection_revision
        {
            return Err(McpFailureCode::StaleRevision);
        }
        Ok(credentials)
    }

    async fn discover_details(
        self,
        request: McpToolDetailsRequest,
        cancel: CancellationToken,
    ) -> Result<DiagnosticResult, DiagnosticProbeError> {
        let deadline = Instant::now() + CHECK_TIMEOUT;
        let http_io = CancellationToken::new();
        let _close_io = http_io.clone().drop_guard();
        let cleanup = ConnectionCheckCleanup::default();
        let _close_cleanup = cleanup.cancellation().clone().drop_guard();
        let mut child = None;
        let mut running = None;
        let mut connected = false;
        let mut details = McpToolDetails {
            request: request.clone(),
            tools: vec![],
            connection_error: None,
            tools_error: None,
            runtime: None,
            runtime_error: None,
        };
        let discovery = async {
            let credentials = self.authorize_details(&request)?;
            let accounts = if credentials {
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
            let host = &self.context.scope.target_host_id;
            let (mut connection, _) = self
                .context
                .catalog
                .resolve_execution_connection(
                    &request.scope,
                    &self.id,
                    |record| {
                        record.record == request.record
                            && record.connection_revision == request.connection_revision
                    },
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
            self.authorize_details(&request)?;
            if let McpExecutionConnection::Stdio { cwd, .. } = &mut connection {
                cwd.get_or_insert_with(|| ".".to_owned());
            }
            let workspace = self.grant.permit.stdio_workspace.as_deref().unwrap_or(Path::new("/"));
            let server = runtime_server(self.id.0.clone(), connection, workspace)
                .map_err(|_| McpFailureCode::InvalidDefinition)?;
            let client = running.insert(
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
            connected = true;
            self.authorize_details(&request)?;
            if client.peer_info().is_some_and(|info| info.capabilities.tools.is_some()) {
                let tools = crate::plugins::list_mcp_catalog_tools(client.peer()).await;
                match tools.and_then(|tools| {
                    crate::tools::mcp_tool::mcp_tools_to_specs(&self.id.0, None, &tools, &[])
                }) {
                    Ok(specs) => {
                        details.tools = specs.into_iter().map(|spec| spec.to_dto()).collect()
                    }
                    Err(_) => details.tools_error = Some(McpFailureCode::ToolDiscoveryFailed),
                }
            }
            self.authorize_details(&request)?;
            Ok::<_, McpFailureCode>(())
        };
        let result = tokio::select! {
            biased;
            () = cancel.cancelled() => None,
            result = timeout_at(deadline, discovery) => Some(match result {
                Ok(result) => result,
                Err(_) => Err(McpFailureCode::TimedOut),
            }),
        };
        http_io.cancel();
        if let Some(child) = child {
            child.kill_and_reap().await;
        }
        if let Some(peer) = running {
            let _ = timeout_at(deadline, peer.cancel()).await;
        }
        if result.is_none() || cancel.is_cancelled() {
            return Err(DiagnosticProbeError::Cancelled);
        }
        let _ = timeout_at(deadline, cleanup.wait()).await;
        let result = result.unwrap().and_then(|()| self.authorize_details(&request).map(|_| ()));
        if let Err(code) = result {
            details.tools.clear();
            if connected && code == McpFailureCode::TimedOut {
                details.tools_error = Some(code);
            } else {
                details.tools_error = None;
                details.connection_error = Some(code);
            }
        }
        if self.authorize_details(&request).is_ok() {
            match run_sync_database(|| {
                Ok(self.context.catalog.runtime_usage(
                    &self.context.scope.project,
                    &request.scope,
                    &self.id,
                ))
            }) {
                Ok(Ok(usage)) => details.runtime = Some(usage),
                Ok(Err(code)) => details.runtime_error = Some(code),
                Err(_) => details.runtime_error = Some(McpFailureCode::StorageUnavailable),
            }
            if let Err(code) = self.authorize_details(&request) {
                details.tools.clear();
                details.runtime = None;
                details.connection_error = Some(code);
            }
        }
        Ok(DiagnosticResult::McpToolDetails { details })
    }
}
