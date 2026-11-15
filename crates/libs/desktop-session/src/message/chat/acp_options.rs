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

/// Confirmed preferences for the host's shared session model projection.
pub struct ConfirmedSource {
    pub source: vw_api_acp_model::acp_model_source::AcpModelSource,
    pub model: Option<(String, bool)>,
}

impl AcpOptionsState {
    /// None denotes a stale response or a busy session whose confirmed projection is unchanged.
    pub fn apply_source_loaded(
        &mut self, request_id: u64, agent_matches: bool, is_requesting: bool,
        result: Result<AcpSessionOptions, String>,
    ) -> Result<Option<ConfirmedSource>, String> {
        if !agent_matches || self.source_request_id != Some(request_id) || self.source_pending.is_some() {
            return Ok(None);
        }
        self.pending = false;
        self.source_request_id = None;
        let options = result?;
        if is_requesting {
            return Ok(None);
        }
        let mut source = options.acp_model_source;
        let model = options.selected_model.as_ref().and_then(|selection| {
            source = selection.source;
            selection.model.as_ref().map(|model| {
                let display = match &selection.provider_id {
                    Some(provider) if model != "auto" && !model.starts_with(&format!("{provider}/")) => format!("{provider}/{model}"),
                    _ => model.clone(),
                };
                (display, model == "auto")
            })
        });
        self.options = options;
        Ok(Some(ConfirmedSource { source, model }))
    }

    pub fn source_save_matches(&self, request_id: u64, update: &AcpModelSourceUpdate, agent: Option<&str>) -> bool {
        self.source_request_id == Some(request_id)
            && self.source_pending.as_ref() == Some(update)
            && agent == Some(update.agent.as_str())
    }

    pub fn apply_source_saved(&mut self, update: &AcpModelSourceUpdate, result: Result<AcpSessionOptions, String>) -> Result<ConfirmedSource, String> {
        self.source_pending = None;
        self.source_request_id = None;
        let options = result?;
        if options.acp_model_source != update.selection.source {
            return Err("后端未确认所选模型来源".into());
        }
        let source = options.acp_model_source;
        let model = options.selected_model.as_ref().and_then(|selection| selection.model.as_ref()).map(|model| (model.clone(), model == "auto"));
        self.options = options;
        self.model_input_draft = None;
        Ok(ConfirmedSource { source, model })
    }
}

impl AcpOptionsState {
    pub fn begin_option(&mut self, selection: Option<&vw_api_acp_model::acp_session::AcpSessionSelection>) -> Result<bool, &'static str> {
        use vw_api_acp_model::acp_session::AcpOptionKind;
        if let Some(selection) = selection {
            if matches!(&selection.kind, AcpOptionKind::Model)
                || matches!(&selection.kind, AcpOptionKind::Config { id } if id == "model") {
                return Err("会话开始后不能切换模型");
            }
            if matches!(&selection.kind, AcpOptionKind::Mode)
                || !self.options.options.iter().any(|option| option.kind == selection.kind && option.choices.iter().any(|choice| choice.id == selection.value)) {
                return Ok(false);
            }
        } else {
            self.open = true;
        }
        self.pending = true;
        Ok(true)
    }

    pub fn begin_source(&mut self, request_id: u64, agent: String, source: vw_api_acp_model::acp_model_source::AcpModelSource, model: String, is_requesting: bool) -> Result<AcpModelSourceUpdate, &'static str> {
        use vw_api_acp_model::acp_model_source::{AcpModelSource, AcpSessionModelSelection};
        if is_requesting || self.pending || self.source_pending.is_some() {
            return Err("请等待当前请求结束后切换模型来源");
        }
        if self.options.model_source_capabilities.iter().any(|capability| matches!(capability, AcpModelSourceCapability::Unsupported { source: rejected, .. } if *rejected == source)) {
            return Err("此智能体不支持所选模型来源");
        }
        let update = AcpModelSourceUpdate {
            agent, selection: AcpSessionModelSelection { source, provider_id: None, model: (source == AcpModelSource::SessionModel).then_some(model) },
        };
        self.source_pending = Some(update.clone());
        self.source_request_id = Some(request_id);
        Ok(update)
    }
}

pub async fn load_or_select_options(client: &GatewaySessionClient, session_id: &str, directory: Option<&str>, selection: Option<&vw_api_acp_model::acp_session::AcpSessionSelection>) -> Result<AcpSessionOptions, String> {
    match selection {
        Some(selection) => client.acp_session_select(session_id, directory, selection).await,
        None => client.acp_session_options(session_id, directory).await,
    }
}
