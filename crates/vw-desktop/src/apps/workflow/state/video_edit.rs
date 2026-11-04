//! Tenant-owned editor draft; server versions are kept outside the YAML.
use iced::widget::text_editor;
use vw_gateway_client::vw_api_types::{TenantScope, workflow::WorkflowRecord};

pub const TEMPLATE_ID: &str = "16cbb033-b312-48b6-9bfc-91c87391b167";

#[derive(Debug, Default)]
pub struct VideoEditUiState {
    pub open: bool,
    pub graph_version: Option<u64>,
    pub tenant: Option<TenantScope>,
    pub record: Option<WorkflowRecord>,
    pub editor: Option<text_editor::Content>,
    pub submitted: Option<String>,
    pub path: String,
    pub restore_confirm: bool,
    pub busy: bool,
    pub error: Option<String>,
}
impl VideoEditUiState {
    pub fn accept_record(&mut self, record: WorkflowRecord) {
        // Keep the draft and its original CAS version together across refreshes.
        let dirty = self
            .record
            .as_ref()
            .zip(self.editor.as_ref())
            .is_some_and(|(old, editor)| editor.text() != old.workflow_yaml);
        if dirty {
            self.busy = false;
            return;
        }
        self.editor = Some(text_editor::Content::with_text(&record.workflow_yaml));
        self.record = Some(record);
        self.busy = false;
    }
    pub fn validated_yaml(&self) -> Result<String, String> {
        let source = self.editor.as_ref().ok_or("请先读取模板")?.text();
        super::super::model::load_document_from_text(None, source.clone())
            .map_err(|e| e.to_string())?;
        Ok(source)
    }
    pub fn native_command(&self) -> Result<String, String> {
        let path = self.path.trim();
        if path.is_empty() || path.contains(['"', '\n', '\r', '\0']) {
            return Err("请选择有效视频路径（不含双引号或换行）".into());
        }
        Ok(format!("/shiju 剪辑 \"{path}\""))
    }
}

impl super::WorkflowState {
    /// Swap tenant-owned drafts without carrying an active run subscription across tenants.
    pub fn select_tenant(&mut self, tenant: TenantScope) {
        if let Some(previous_tenant) =
            self.video_edit.tenant.clone().filter(|current| current != &tenant)
        {
            let mut drafts = std::mem::take(&mut self.tenant_drafts);
            let mut previous = std::mem::take(self);
            previous.running_saved_app_uuid = None;
            previous.video_edit.busy = false;
            previous.saved_apps_loading = false;
            previous.opening_saved_app_uuid = None;
            drafts.insert(previous_tenant, Box::new(previous));
            *self = drafts.remove(&tenant).map(|draft| *draft).unwrap_or_default();
            self.tenant_drafts = drafts;
        }
        self.video_edit.tenant = Some(tenant);
    }
}
