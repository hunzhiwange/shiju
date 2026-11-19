use gateway_mcp_distribution::*;
use vw_api_mcp_management::*;
use vw_gateway_client::GatewayClient;
#[derive(Clone)]
pub struct ImportState {
    pub context: crate::management_context::Context,
    pub generation: u64,
    pub source: Option<McpAppTarget>,
    pub account: String,
    pub all: bool,
    pub previews: Vec<(McpAppTarget, Result<McpNativePreview, String>)>,
    pub reports: Vec<SourceReport>,
    pub show_sources: bool,
    pub draft: Option<McpCreateRequest>,
    pub link_drafts: Vec<Result<McpCreateRequest, String>>,
    pub preview: Option<McpNativePreview>,
    pub loading: bool,
    pub importing: bool,
    pub result: Option<Result<Vec<String>, String>>,
}
impl std::fmt::Debug for ImportState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("McpImportState { .. }")
    }
}
#[derive(Clone)]
pub enum ImportMessage {
    Open,
    Close,
    Select(McpAppTarget),
    SelectAll,
    ToggleSources,
    Retry,
    BatchPreviewed(
        crate::management_context::Context,
        u64,
        Vec<(McpAppTarget, Result<McpNativePreview, String>)>,
    ),
    BatchFinished(crate::management_context::Context, u64, Vec<SourceReport>),
    Account(String),
    Preview,
    Previewed(crate::management_context::Context, u64, Result<McpNativePreview, String>),
    Paste,
    Pasted(crate::management_context::Context, u64, Option<String>),
    Confirm,
    Finished(crate::management_context::Context, u64, Result<Vec<String>, String>),
}
impl std::fmt::Debug for ImportMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("McpImportMessage { .. }")
    }
}
fn next_generation() -> u64 {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}
impl ImportState {
    pub fn new(context: crate::management_context::Context) -> Self {
        Self {
            context,
            generation: next_generation(),
            source: None,
            account: String::new(),
            all: false,
            previews: Vec::new(),
            reports: Vec::new(),
            show_sources: false,
            draft: None,
            link_drafts: Vec::new(),
            preview: None,
            loading: false,
            importing: false,
            result: None,
        }
    }
    pub fn can_confirm(&self) -> bool {
        !self.loading
            && !self.importing
            && self.result.is_none()
            && (!self.link_drafts.is_empty()
                || self.draft.is_some()
                || !self.previews.is_empty()
                || self.preview.as_ref().is_some_and(|p| {
                    Some(&p.target) == self.source.as_ref() && p.status == McpNativeStatus::Present
                }))
    }
    pub fn invalidate(&mut self) {
        self.generation = next_generation();
        self.preview = None;
        self.previews.clear();
        self.reports.clear();
        self.loading = false;
        self.result = None;
    }
}
pub fn preview_labels(preview: &McpNativePreview) -> Vec<String> {
    let mut labels = vec![
        format!("原生状态：{:?}（脱敏预览）", preview.status),
        "仅对比已有目录 ID；新项、定义差异与冲突由执行端在导入时判定。".into(),
    ];
    for entry in &preview.entries {
        labels.push(format!(
            "{} · 目录已有项 · 原生 {:?} · {:?} · {}",
            entry.id.0,
            entry.status,
            entry.transport,
            match entry.enabled {
                Some(false) => "来源禁用",
                Some(true) => "来源启用",
                None => "启用状态未读到",
            }
        ));
    }
    labels
}
pub fn result_labels(
    response: McpImportResponse,
    target: &McpAppTarget,
) -> Result<Vec<String>, String> {
    if response.sources.len() != 1 || &response.sources[0].target != target {
        return Err("导入响应目标不匹配，请刷新目录核实".into());
    }
    let source = response.sources.into_iter().next().expect("one source");
    match (source.entries, source.code) {
        (None, Some(code)) => Ok(vec![format!("来源导入失败：{code:?}")]),
        (Some(entries), None) => {
            if entries.is_empty() {
                return Ok(vec!["未发现可导入项".into()]);
            }
            Ok(entries
                .into_iter()
                .map(|entry| {
                    let id = entry.id.map(|id| id.0).unwrap_or_else(|| "无效名称".into());
                    let label = match entry.outcome {
                        McpImportEntryOutcome::Added { .. } => {
                            "已新增（保留来源启用或禁用状态）".into()
                        }
                        McpImportEntryOutcome::Merged { .. } => {
                            "同定义，已新增绑定（保留来源启用状态）".into()
                        }
                        McpImportEntryOutcome::Unchanged { .. } => {
                            "已有同定义绑定，无需变更".into()
                        }
                        McpImportEntryOutcome::Failed { code: McpFailureCode::Conflict } => {
                            "冲突，未覆盖已有定义".into()
                        }
                        McpImportEntryOutcome::Failed { code } => format!("导入失败：{code:?}"),
                    };
                    format!("{id} · {label}")
                })
                .collect())
        }
        _ => Err("导入响应不完整，请刷新目录核实".into()),
    }
}
pub async fn import(
    client: &GatewayClient,
    source: McpImportSource,
    token: &str,
) -> Result<Vec<String>, String> {
    let response =
        gateway_mcp_distribution::GatewayMcpDistributionClient::new(client.transport().clone())
            .mcp_management_import(
                &McpImportRequest { project_id: None, sources: vec![source.clone()] },
                token,
            )
            .await
            .map_err(|_| "导入请求失败，请刷新目录核实；不要直接重复提交".to_owned())?;
    result_labels(response, &source.target)
}

use crate::import as ui;
pub fn panel(
    state: &ImportState,
    targets: Vec<vw_api_mcp_management::McpAppTarget>,
) -> iced::Element<'_, ImportMessage> {
    ui::panel(
        ui::ImportView {
            host: &state.context.host,
            importing: state.importing,
            loading: state.loading,
            link_drafts: &state.link_drafts,
            draft: &state.draft,
            source: &state.source,
            account: &state.account,
            all: state.all,
            show_sources: state.show_sources,
            can_retry: !state.reports.is_empty() && state.reports.iter().any(|r| r.retry),
            has_preview: state.preview.is_some() || !state.previews.is_empty(),
            can_confirm: state.can_confirm(),
            result: &state.result,
        },
        targets,
        {
            let mut labels = state.preview.as_ref().map(preview_labels).unwrap_or_default();
            for (target, preview) in &state.previews {
                labels.push(format!("{:?} · {}", target.app_id, target.installation_id.0));
                match preview {
                    Ok(p) => labels.extend(preview_labels(p)),
                    Err(_) => labels.push("未读到：预览失败，请检查账号与权限".into()),
                }
            }
            if state.show_sources {
                labels.push("来源报告未提供可信文件路径；以下仅列出实际请求的应用与实例。用户级扫描不包含项目配置或插件。".into());
                for r in &state.reports { labels.push(format!("{:?} · {} · 主机 {}", r.target.app_id, r.target.installation_id.0, r.target.target_host_id)); }
            }
            labels
        },
    )
    .map(|event| match event {
        ui::ImportMessage::SelectAll => ImportMessage::SelectAll,
        ui::ImportMessage::ToggleSources => ImportMessage::ToggleSources,
        ui::ImportMessage::Retry => ImportMessage::Retry,
        ui::ImportMessage::Close => ImportMessage::Close,
        ui::ImportMessage::Select(v) => ImportMessage::Select(v),
        ui::ImportMessage::Account(v) => ImportMessage::Account(v),
        ui::ImportMessage::Preview => ImportMessage::Preview,
        ui::ImportMessage::Paste => ImportMessage::Paste,
        ui::ImportMessage::Confirm => ImportMessage::Confirm,
    })
}

pub fn visible<'a>(
    state: Option<&'a ImportState>,
    current: Option<&crate::management_context::Context>,
) -> Option<&'a ImportState> {
    state.filter(|s| current == Some(&s.context))
}

/// Only redacted outcome labels leave the transport layer.
#[derive(Clone)]
pub struct SourceReport {
    pub target: McpAppTarget,
    pub labels: Vec<String>,
    pub added: usize,
    pub bindings: usize,
    pub skipped: usize,
    pub failed: usize,
    pub retry: bool,
}
impl SourceReport {
    pub fn failure(target: McpAppTarget, label: &str) -> Self {
        Self {
            target,
            labels: vec![label.into()],
            added: 0,
            bindings: 0,
            skipped: 0,
            failed: 1,
            retry: true,
        }
    }
}
pub async fn import_report(
    client: &GatewayClient,
    source: McpImportSource,
    token: &str,
) -> SourceReport {
    let response = GatewayMcpDistributionClient::new(client.transport().clone())
        .mcp_management_import(
            &McpImportRequest { project_id: None, sources: vec![source.clone()] },
            token,
        )
        .await;
    let Ok(response) = response else {
        return SourceReport::failure(source.target, "请求失败，重试前请核实目录状态");
    };
    if response.sources.len() != 1 || response.sources[0].target != source.target {
        return SourceReport::failure(source.target, "响应目标不匹配，请核实目录");
    }
    let mut report = SourceReport {
        target: source.target.clone(),
        labels: Vec::new(),
        added: 0,
        bindings: 0,
        skipped: 0,
        failed: 0,
        retry: false,
    };
    if let Some(entries) = &response.sources[0].entries {
        for entry in entries {
            match &entry.outcome {
                McpImportEntryOutcome::Added { .. } => report.added += 1,
                McpImportEntryOutcome::Merged { .. } => report.bindings += 1,
                McpImportEntryOutcome::Unchanged { .. } => report.skipped += 1,
                McpImportEntryOutcome::Failed { code: McpFailureCode::Conflict } => {
                    report.skipped += 1
                }
                McpImportEntryOutcome::Failed { .. } => {
                    report.failed += 1;
                    report.retry = true;
                }
            }
        }
    }
    match result_labels(response, &source.target) {
        Ok(labels) => report.labels = labels,
        Err(_) => return SourceReport::failure(source.target, "响应不完整，请核实目录"),
    }
    // A source-level failure has no entry outcomes.
    if report.added + report.bindings + report.skipped + report.failed == 0
        && report.labels.iter().any(|l| l.starts_with("来源导入失败"))
    {
        report.failed = 1;
        report.retry = true;
    }
    report
}
pub fn report_labels(reports: &[SourceReport]) -> Vec<String> {
    let mut labels = vec![format!(
        "导入完成：新增 {} · 绑定变更 {} · 跳过 {} · 失败 {}",
        reports.iter().map(|r| r.added).sum::<usize>(),
        reports.iter().map(|r| r.bindings).sum::<usize>(),
        reports.iter().map(|r| r.skipped).sum::<usize>(),
        reports.iter().map(|r| r.failed).sum::<usize>()
    )];
    for report in reports {
        labels.push(format!("{:?} · {}", report.target.app_id, report.target.installation_id.0));
        labels.extend(report.labels.clone());
    }
    labels
}
