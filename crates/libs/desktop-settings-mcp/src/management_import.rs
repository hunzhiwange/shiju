use gateway_mcp_distribution::*;
use vw_api_mcp_management::*;
use vw_gateway_client::GatewayClient;
#[derive(Clone)]
pub struct ImportState {
    pub context: crate::management_context::Context,
    pub generation: u64,
    pub source: Option<McpAppTarget>,
    pub account: String,
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
                || self.preview.as_ref().is_some_and(|p| {
                    Some(&p.target) == self.source.as_ref() && p.status == McpNativeStatus::Present
                }))
    }
    pub fn invalidate(&mut self) {
        self.generation = next_generation();
        self.preview = None;
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
            "{} · 目录已有项 · 原生 {:?} · {:?}",
            entry.id.0, entry.status, entry.transport
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
                        McpImportEntryOutcome::Added { .. } => "已导入".into(),
                        McpImportEntryOutcome::Merged { .. } => "已合并（保留已有定义）".into(),
                        McpImportEntryOutcome::Unchanged { .. } => "已导入，无需变更".into(),
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
