use vw_api_mcp_management::*;
use vw_api_app_operations::AppOperationPhase;
pub fn summary(result: &McpDistributionResult) -> &'static str {
    if result.outcome == McpBatchOutcome::Succeeded
        && !result.items.is_empty()
        && result.items.iter().all(|item| {
            item.desired_committed
                && matches!(
                    item.native_outcome,
                    McpDistributionOutcome::Applied | McpDistributionOutcome::Unchanged
                )
        })
    {
        "全部目标已应用"
    } else {
        "存在失败、取消或未验证目标，请查看逐项结果"
    }
}
pub fn item_label(item: &McpDistributionItem) -> String {
    let native = match item.native_outcome {
        McpDistributionOutcome::Applied => "已应用".into(),
        McpDistributionOutcome::Unchanged => "无需变更".into(),
        McpDistributionOutcome::NotVerified => "未验证".into(),
        McpDistributionOutcome::Unsupported => "不支持".into(),
        McpDistributionOutcome::Cancelled => "已取消".into(),
        McpDistributionOutcome::Failed { code } => format!("失败：{code:?}"),
    };
    format!(
        "{} · {:?} / {} · 期望{} · {} · 原生{}",
        item.record.id.0,
        item.target.app_id,
        item.target.installation_id.0,
        if item.enabled { "启用" } else { "禁用" },
        if item.desired_committed { "期望已保存" } else { "期望未保存" },
        native
    )
}
pub fn terminal(status: &McpResyncOperationStatus) -> bool {
    status.operation.phase == AppOperationPhase::Terminal
}
pub fn failed(target: &McpResyncTargetResult) -> bool {
    target.code.is_some()
        || target.distribution.as_ref().is_none_or(|d| {
            matches!(d.outcome, McpBatchOutcome::Failed | McpBatchOutcome::PartiallySucceeded)
                || d.items
                    .iter()
                    .any(|i| matches!(i.native_outcome, McpDistributionOutcome::Failed { .. }))
        })
}
pub fn successful(status: &McpResyncOperationStatus) -> bool {
    status.outcome == Some(McpBatchOutcome::Succeeded)
        && status.targets.iter().all(|target| {
            target.code.is_none()
                && target.distribution.as_ref().is_some_and(|d| {
                    d.outcome == McpBatchOutcome::Succeeded
                        && d.items.iter().all(|item| {
                            item.desired_committed
                                && matches!(
                                    item.native_outcome,
                                    McpDistributionOutcome::Applied
                                        | McpDistributionOutcome::Unchanged
                                )
                        })
                })
        })
}
use vw_api_app_identity::AppId;
use vw_api_transport::ProjectId;
pub const APPS: [AppId; 14] = [
    AppId::Claude,
    AppId::ClaudeDesktop,
    AppId::Codex,
    AppId::Gemini,
    AppId::GrokBuild,
    AppId::OpenCode,
    AppId::OpenClaw,
    AppId::Hermes,
    AppId::Pi,
    AppId::Mcode,
    AppId::Kimi,
    AppId::Dsh,
    AppId::Agy,
    AppId::Omp,
];
pub fn unsupported_reason(app: AppId) -> Option<&'static str> {
    match app {
        AppId::ClaudeDesktop | AppId::OpenClaw => Some("上游不支持原生分发"),
        _ => None,
    }
}
pub fn project(records: &[McpCatalogRecord]) -> Result<Option<ProjectId>, String> {
    let first = records.first().ok_or("请先选择目录项")?;
    if records.len() > 256 || records.iter().any(|record| record.scope != first.scope) {
        return Err("一次最多选择256项，且必须属于同一作用域".into());
    }
    Ok(match &first.scope {
        McpCatalogScope::Tenant { .. } => None,
        McpCatalogScope::Project { scope } => Some(scope.project_id().clone()),
    })
}
pub fn supported(
    catalog: &McpCatalogResponse,
    records: &[McpCatalogRecord],
    app: AppId,
) -> bool {
    unsupported_reason(app).is_none()
        && !records.is_empty()
        && records.iter().all(|record| {
            catalog.capabilities.iter().any(|cap| {
                cap.app_id == app
                    && match record.transport {
                        McpTransport::Stdio => cap.stdio,
                        McpTransport::StreamableHttp => cap.streamable_http,
                        McpTransport::Sse => cap.sse,
                    } == McpSupport::Supported
            })
        })
}
