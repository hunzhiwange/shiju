//! MCP diagnostic identity and lifecycle; no production connection ownership.
use vw_gateway_client::{
    GatewayClient,
    vw_api_types::{
        agent_apps::AppId,
        agent_apps_diagnostics::*,
        mcp_management::{McpRecordId, McpRecordRef},
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
            Self::Queued => "排队中",
            Self::Handshake => "握手中",
            Self::Succeeded => "握手成功",
            Self::Failed => "检测失败",
            Self::Cancelling => "正在取消，等待清理",
            Self::Cancelled => "已取消",
            Self::Unverified => "未验证",
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
            let outcome =
                snapshot.results.as_deref().unwrap_or_default().iter().find_map(|result| {
                    match result {
                        DiagnosticResult::Mcp { server_id, report } if server_id == &record.id => {
                            Some(&report.outcome)
                        }
                        _ => None,
                    }
                });
            match outcome {
                Some(DiagnosticOutcome::Succeeded { .. }) => *phase = Phase::Succeeded,
                Some(DiagnosticOutcome::Failed { .. }) => *phase = Phase::Failed,
                Some(DiagnosticOutcome::Cancelled {}) => *phase = Phase::Cancelled,
                None if matches!(phase, Phase::Queued | Phase::Handshake | Phase::Cancelling) => {
                    *phase = pending
                }
                None => {}
            }
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
) -> Result<DiagnosticJobIdentity, String> {
    client
        .diagnostic_start(&context.query, &DiagnosticStartRequest::Mcp { server_ids: ids }, token)
        .await
        .map_err(|_| "MCP 检测请求失败，结果未验证；请检查授权和目标主机".to_owned())
}
