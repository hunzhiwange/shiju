//! MCP diagnostic identity and lifecycle; no production connection ownership.
use vw_gateway_client::{
    GatewayClient,
    vw_api_types::{
        agent_apps::AppId,
        agent_apps_diagnostics::*,
        mcp_management::{McpCatalogRecord, McpConnectionOutcome, McpRecordId, McpRecordRef},
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Context {
    pub(crate) gateway: String,
    pub(crate) tenant: vw_gateway_client::vw_api_types::TenantScope,
    pub(crate) project_path: Option<String>,
    pub(crate) credentials_generation: u64,
    pub(crate) query: DiagnosticScopeQuery,
    pub(crate) records: Vec<McpRecordRef>,
    pub(crate) connections: Vec<McpCatalogRecord>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Phase {
    Queued,
    Handshake,
    Succeeded,
    Failed,
    Cancelling,
    Cancelled,
    Unverified,
}
impl Phase {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Self::Queued => "checking · 排队中",
            Self::Handshake => "checking · 握手中",
            Self::Succeeded => "connected · 已连通",
            Self::Failed => "error · 检测失败",
            Self::Cancelling => "checking · 正在取消，等待清理",
            Self::Cancelled => "cancelled · 已取消",
            Self::Unverified => "unchecked · 未验证",
        }
    }
}
#[derive(Debug, Clone, Default)]
pub(crate) struct McpState {
    pub(crate) app: Option<AppChoice>,
    pub(crate) context: Option<Context>,
    pub(crate) generation: u64,
    pub(crate) request_serial: u64,
    pub(crate) targets: Vec<(McpRecordRef, Phase)>,
    pub(crate) job: Option<DiagnosticJobIdentity>,
    pub(crate) terminal: bool,
    pub(crate) cancelling: bool,
    pub(crate) error: Option<String>,
    pub(crate) transport: Option<super::ProbeTransport>,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppChoice(pub AppId, pub String);
impl std::fmt::Display for AppChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.1)
    }
}
impl McpState {
    pub(crate) fn active(&self) -> bool {
        self.context.is_some() && !self.terminal
    }
    pub(crate) fn begin(&mut self, context: Context, targets: Vec<McpRecordRef>) {
        self.generation = self.generation.wrapping_add(1);
        self.context = Some(context);
        self.targets = targets.into_iter().map(|r| (r, Phase::Queued)).collect();
        self.job = None;
        self.transport = None;
        self.terminal = false;
        self.cancelling = false;
        self.error = None;
    }
    pub(crate) fn accept(
        &mut self,
        current: &Context,
        request: &Context,
        generation: u64,
        serial: u64,
        snapshot: DiagnosticJobSnapshot,
    ) -> bool {
        if current != request
            || self.context.as_ref() != Some(request)
            || self.generation != generation
            || self.request_serial != serial
            || self.terminal
            || self.job.as_ref() != Some(&snapshot.job)
        {
            return false;
        }
        self.terminal = matches!(
            snapshot.state,
            DiagnosticJobState::Succeeded {}
                | DiagnosticJobState::Failed { .. }
                | DiagnosticJobState::Cancelled {}
                | DiagnosticJobState::Interrupted {}
        );
        self.cancelling |= matches!(snapshot.state, DiagnosticJobState::Cancelling {});
        let pending = match snapshot.state {
            DiagnosticJobState::Cancelled {} => Phase::Cancelled,
            DiagnosticJobState::Failed { .. } => Phase::Failed,
            DiagnosticJobState::Succeeded {} | DiagnosticJobState::Interrupted {} => {
                Phase::Unverified
            }
            _ if self.cancelling => Phase::Cancelling,
            DiagnosticJobState::Queued {} => Phase::Queued,
            DiagnosticJobState::Running {} => Phase::Handshake,
            DiagnosticJobState::Cancelling {} => Phase::Cancelling,
        };
        // Results are joined by stable server ID, never by response order.
        for (record, phase) in &mut self.targets {
            let matches: Vec<_> = snapshot
                .results
                .iter()
                .flatten()
                .filter_map(|result| match result {
                    DiagnosticResult::McpConnectionCheck {
                        scope,
                        generation: result_generation,
                        item,
                    } if item.id == record.id
                        && *result_generation == generation
                        && current.connections.iter().any(|r| {
                            r.record.id == item.id
                                && r.connection_revision == item.connection_revision
                                && &r.scope == scope
                        }) =>
                    {
                        Some(item.outcome)
                    }
                    _ => None,
                })
                .collect();
            *phase = match matches.as_slice() {
                [McpConnectionOutcome::Connected { .. }] => Phase::Succeeded,
                [McpConnectionOutcome::Failed { .. } | McpConnectionOutcome::Unsupported] => {
                    Phase::Failed
                }
                [McpConnectionOutcome::Cancelled] => Phase::Cancelled,
                [McpConnectionOutcome::NotVerified] => Phase::Unverified,
                [] if matches!(phase, Phase::Succeeded | Phase::Failed | Phase::Cancelled) => {
                    *phase
                }
                [] => pending,
                _ => Phase::Unverified,
            };
        }
        self.error = None;
        true
    }
}
pub(crate) async fn submit(
    client: GatewayClient,
    context: &Context,
    ids: Vec<McpRecordId>,
    token: &str,
    generation: u64,
) -> Result<DiagnosticJobIdentity, String> {
    client
        .mcp_management_check(&context.query, &ids, generation, token)
        .await
        .map_err(|_| "MCP 检测请求失败，结果未验证；请检查授权和目标主机".to_owned())
}
