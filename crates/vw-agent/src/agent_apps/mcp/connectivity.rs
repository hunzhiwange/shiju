//! Explicit connection checks on the authorized execution host.

use std::{future::pending, path::Path, time::Duration};

use tokio::time::{Instant, timeout, timeout_at};
use tokio_util::sync::CancellationToken;

use vw_api_types::mcp_management::{
    McpCatalogScope, McpConnectionDefinition, McpConnectionOutcome, McpCredentialRef,
    McpFailureCode, McpRecordId, McpRecordRef,
};

use super::{McpCatalogStore, McpExecutionConnection, runtime::runtime_server};
use crate::{
    agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore},
    global_database::run_sync_database,
    plugins::list_mcp_catalog_tools,
    tools::mcp_common::open_connection_check_peer,
};

const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(4);

pub(super) struct ConnectionCheckHost<'a> {
    pub scope: &'a McpCatalogScope,
    pub execution_host: &'a str,
    pub workspace: &'a Path,
    pub accounts: &'a AuthAccountStore,
}

pub(super) enum CheckRevision {
    Record(u64),
    Connection(u64),
}

impl McpCatalogStore {
    /// Check an exact revision on the caller-authorized local host and workspace.
    /// Authorization, task scheduling and result publication belong to the host caller.
    pub async fn check_connection<'a>(
        &self,
        scope: &McpCatalogScope,
        expected: &McpRecordRef,
        execution_host: &str,
        workspace: &Path,
        accounts: &AuthAccountStore,
        authorize_credential_use: impl Fn(
            &McpConnectionDefinition,
            &McpCredentialRef,
        ) -> Option<AuthAccountScope<'a>>
        + Send
        + Sync,
    ) -> McpConnectionOutcome {
        self.check_connection_inner(
            ConnectionCheckHost { scope, execution_host, workspace, accounts },
            &expected.id,
            CheckRevision::Record(expected.revision),
            None,
            authorize_credential_use,
        )
        .await
    }

    pub(super) async fn check_connection_inner<'a>(
        &self,
        host: ConnectionCheckHost<'_>,
        id: &McpRecordId,
        expected: CheckRevision,
        cancellation: Option<&CancellationToken>,
        authorize_credential_use: impl Fn(
            &McpConnectionDefinition,
            &McpCredentialRef,
        ) -> Option<AuthAccountScope<'a>>
        + Send
        + Sync,
    ) -> McpConnectionOutcome {
        let deadline = Instant::now() + CHECK_TIMEOUT;
        let mut running = None;
        let mut child = None;
        let http_io = CancellationToken::new();
        let _cancel_http_io = http_io.clone().drop_guard();
        let check = async {
            let (mut connection, revision) = self
                .resolve_execution_connection(
                    host.scope,
                    id,
                    |record| match expected {
                        CheckRevision::Record(revision) => record.record.revision == revision,
                        CheckRevision::Connection(revision) => {
                            record.connection_revision == revision
                        }
                    },
                    host.execution_host,
                    Some(host.accounts),
                    authorize_credential_use,
                )
                .await?;
            if let McpExecutionConnection::Stdio { cwd, .. } = &mut connection {
                // Relative and omitted working directories resolve on the trusted local host.
                if !host.workspace.is_absolute() {
                    return Err(McpFailureCode::TargetUnavailable);
                }
                cwd.get_or_insert_with(|| ".".to_owned());
            }
            let server = runtime_server(id.0.clone(), connection, host.workspace)
                .map_err(|_| McpFailureCode::InvalidDefinition)?;
            if Instant::now() >= deadline {
                return Err(McpFailureCode::TimedOut);
            }
            let client = running.insert(
                open_connection_check_peer(
                    server,
                    deadline.saturating_duration_since(Instant::now()),
                    http_io.clone(),
                    cancellation.map(|_| &mut child),
                    None,
                )
                .await
                .map_err(|_| McpFailureCode::ConnectionFailed)?,
            );
            let tool_count =
                if client.peer_info().is_some_and(|info| info.capabilities.tools.is_some()) {
                    let tools = list_mcp_catalog_tools(client.peer())
                        .await
                        .map_err(|_| McpFailureCode::ToolDiscoveryFailed)?;
                    // The shared catalog loader bounds the count to 2048.
                    tools.len() as u32
                } else {
                    0
                };
            Ok((tool_count, revision))
        };
        let cancelled = async {
            match cancellation {
                Some(token) => token.cancelled().await,
                None => pending().await,
            }
        };
        let result = tokio::select! {
            biased;
            () = cancelled => Err(McpConnectionOutcome::Cancelled),
            result = timeout_at(deadline, check) => match result {
                Ok(Ok(result)) if Instant::now() < deadline => Ok(result),
                Ok(Err(code)) if Instant::now() < deadline => {
                    Err(McpConnectionOutcome::Failed { code })
                }
                _ => Err(McpConnectionOutcome::Failed { code: McpFailureCode::TimedOut }),
            },
        };
        // rmcp can still be awaiting a Streamable HTTP response during initialization.
        http_io.cancel();
        if let Some(child) = child {
            child.kill_and_reap().await;
        }
        if let Some(client) = running {
            let _ = timeout(CLOSE_TIMEOUT, client.cancel()).await;
        }
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            return McpConnectionOutcome::Cancelled;
        }
        match result {
            Ok((tool_count, revision)) => match self.current_connection_revision(host.scope, id) {
                Ok(current) if current == revision => {
                    McpConnectionOutcome::Connected { tool_count: Some(tool_count) }
                }
                Ok(_) | Err(McpFailureCode::NotFound) => {
                    McpConnectionOutcome::Failed { code: McpFailureCode::StaleRevision }
                }
                Err(code) => McpConnectionOutcome::Failed { code },
            },
            Err(outcome) => outcome,
        }
    }

    pub(super) fn current_connection_revision(
        &self,
        scope: &McpCatalogScope,
        id: &McpRecordId,
    ) -> Result<u64, McpFailureCode> {
        run_sync_database(|| {
            Ok(self.active_record(scope, id).map(|stored| stored.record.connection_revision))
        })
        .map_err(|_| McpFailureCode::StorageUnavailable)?
    }
}
