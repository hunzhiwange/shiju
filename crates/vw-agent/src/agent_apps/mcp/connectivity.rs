//! Explicit connection checks on the authorized execution host.

use std::{path::Path, time::Duration};

use tokio::time::{Instant, timeout, timeout_at};

use vw_api_types::mcp_management::{
    McpCatalogScope, McpConnectionDefinition, McpConnectionOutcome, McpCredentialRef,
    McpFailureCode, McpRecordRef,
};

use super::{McpCatalogStore, McpExecutionConnection, runtime::runtime_server};
use crate::{
    agent_apps::auth_accounts::{AuthAccountScope, AuthAccountStore},
    plugins::list_mcp_catalog_tools,
    tools::mcp_common::open_connection_check_peer,
};

const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(4);

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
        let deadline = Instant::now() + CHECK_TIMEOUT;
        let mut running = None;
        let check = async {
            let mut connection = self
                .resolve_for_execution(
                    scope,
                    expected,
                    execution_host,
                    accounts,
                    authorize_credential_use,
                )
                .await?;
            if let McpExecutionConnection::Stdio { cwd, .. } = &mut connection {
                // Relative and omitted working directories resolve on the trusted local host.
                if !workspace.is_absolute() {
                    return Err(McpFailureCode::TargetUnavailable);
                }
                cwd.get_or_insert_with(|| ".".to_owned());
            }
            let server = runtime_server(expected.id.0.clone(), connection, workspace)
                .map_err(|_| McpFailureCode::InvalidDefinition)?;
            if Instant::now() >= deadline {
                return Err(McpFailureCode::TimedOut);
            }
            let client = running.insert(
                open_connection_check_peer(server, CHECK_TIMEOUT)
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
            Ok(McpConnectionOutcome::Connected { tool_count: Some(tool_count) })
        };
        let result = timeout_at(deadline, check).await;
        let outcome = match result {
            Ok(Ok(outcome)) if Instant::now() < deadline => outcome,
            Ok(Err(code)) if Instant::now() < deadline => McpConnectionOutcome::Failed { code },
            _ => McpConnectionOutcome::Failed { code: McpFailureCode::TimedOut },
        };
        if let Some(client) = running {
            let _ = timeout(CLOSE_TIMEOUT, client.cancel()).await;
        }
        outcome
    }
}
