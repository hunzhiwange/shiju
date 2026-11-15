pub fn next_source_request_id() -> u64 {
    static NEXT_REQUEST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
    NEXT_REQUEST.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

use gateway_session::GatewaySessionClient;
use vw_api_acp_model::acp_session::AcpSessionOptions;
use vw_api_acp_model::acp_model_source::{AcpModelSourceUpdate, AcpModelSourceCapability};

/// Refresh capabilities without consulting an inherited binding for draft sessions.
pub async fn load_source_options(
    client: &GatewaySessionClient,
    session_id: &str,
    directory: Option<&str>,
    agent: &str,
    draft_options: Option<AcpSessionOptions>,
) -> Result<AcpSessionOptions, String> {
    let mut options = match draft_options {
        Some(options) => options,
        None => {
            client
                .acp_session_options(session_id, directory)
                .await?
        }
    };
    if options.model_source_capabilities.is_empty() {
        let capabilities = client.acp_model_source_capabilities(agent).await?;
        if options.selected_model.is_none() && !options.connected {
            options.acp_model_source = capabilities.acp_model_source;
        }
        options.model_source_capabilities =
            capabilities.model_source_capabilities;
        options.verified_cli_release = capabilities.verified_cli_release;
        options.verified_platform = capabilities.verified_platform;
    }
    Ok(options)
}

/// Validate a draft choice locally or persist an existing session choice at the gateway.
pub async fn save_source_options(
    client: &GatewaySessionClient,
    session_id: &str,
    directory: Option<&str>,
    update: &AcpModelSourceUpdate,
    draft_options: Option<AcpSessionOptions>,
) -> Result<AcpSessionOptions, String> {
    let directory = directory.ok_or("请先选择项目目录")?;
    if let Some(mut options) = draft_options {
        if options.model_source_capabilities.is_empty() {
            let capabilities = client.acp_model_source_capabilities(&update.agent).await?;
            options.model_source_capabilities = capabilities.model_source_capabilities;
            options.verified_cli_release = capabilities.verified_cli_release;
            options.verified_platform = capabilities.verified_platform;
        }
        if !options.model_source_capabilities.iter().any(|capability| {
            matches!(capability, AcpModelSourceCapability::Supported { source: supported } if *supported == update.selection.source)
        }) {
            return Err("此智能体不支持所选模型来源".to_string());
        }
        options.acp_model_source = update.selection.source;
        options.selected_model = Some(update.selection.clone());
        return Ok(options);
    }
    client.acp_session_model_source(session_id, Some(directory), update).await
}

/// Per-session pending and confirmed ACP controls.
#[derive(Debug, Clone, Default)]
pub struct AcpOptionsState {
    pub options: AcpSessionOptions,
    pub model_input_draft: Option<String>,
    pub source_request_id: Option<u64>,
    pub open: bool,
    pub pending: bool,
    pub source_pending: Option<AcpModelSourceUpdate>,
}

impl AcpOptionsState {
    pub fn set_model_input_draft(&mut self, value: String, is_requesting: bool) -> bool {
        if is_requesting || self.source_pending.is_some() {
            return false;
        }
        self.model_input_draft = Some(value);
        true
    }

    pub fn apply_loaded_options(&mut self, result: Result<AcpSessionOptions, String>) -> Result<(), String> {
        self.pending = false;
        match result {
            Ok(options) => {
                self.options = options;
                Ok(())
            }
            Err(error) => {
                // A failed control must not leave an earlier native success on screen.
                if let Some(status) = self.options.access_status.as_mut() {
                    status.support = vw_api_acp_model::acp_session::AcpAccessSupport::Failed;
                    status.explanation = "会话选项切换或读取失败，请刷新或重试；当前能力尚未确认".into();
                }
                Err(error)
            }
        }
    }
}
