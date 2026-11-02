//! Explicit UI operation state; receiving a preview never confirms an installation.
use vw_gateway_client::DesktopPluginPreviewDto;
#[derive(Debug, Clone)]
pub enum PluginInstallPhase {
    Input,
    Preparing,
    Review(DesktopPluginPreviewDto),
    Committing(DesktopPluginPreviewDto),
    Failed(String),
}
#[derive(Debug, Clone)]
pub struct PluginInstallForm {
    pub source: String,
    pub update_id: Option<String>,
    pub request_id: u64,
    pub phase: PluginInstallPhase,
}
impl PluginInstallForm {
    pub fn new(request_id: u64, update_id: Option<String>) -> Self {
        Self { source: String::new(), update_id, request_id, phase: PluginInstallPhase::Input }
    }
    pub fn begin_prepare(&mut self) -> Result<(), String> {
        if self.source.trim().is_empty() && self.update_id.is_none() {
            return Err("来源不能为空".into());
        }
        if self.busy() {
            return Err("正在处理，请等待当前操作完成".into());
        }
        self.phase = PluginInstallPhase::Preparing;
        Ok(())
    }
    pub fn receive_preview(
        &mut self,
        id: u64,
        result: Result<DesktopPluginPreviewDto, String>,
    ) -> Option<DesktopPluginPreviewDto> {
        if id != self.request_id || !matches!(self.phase, PluginInstallPhase::Preparing) {
            return result.ok();
        }
        self.phase = match result {
            Ok(preview) => PluginInstallPhase::Review(preview),
            Err(error) => PluginInstallPhase::Failed(error),
        };
        None
    }
    pub fn confirm(&mut self) -> Option<DesktopPluginPreviewDto> {
        if !self.can_confirm() {
            return None;
        }
        let preview = self.preview()?.clone();
        self.phase = PluginInstallPhase::Committing(preview.clone());
        Some(preview)
    }
    pub fn preview(&self) -> Option<&DesktopPluginPreviewDto> {
        match &self.phase {
            PluginInstallPhase::Review(p) | PluginInstallPhase::Committing(p) => Some(p),
            _ => None,
        }
    }
    pub fn busy(&self) -> bool {
        matches!(self.phase, PluginInstallPhase::Preparing | PluginInstallPhase::Committing(_))
    }
    pub fn can_confirm(&self) -> bool {
        matches!(&self.phase,PluginInstallPhase::Review(p) if !p.token.is_empty() && !p.plugin_id.is_empty() && p.audit_status=="passed" && !p.dependency_checks.iter().any(|d|d.status=="invalid"))
    }
}
