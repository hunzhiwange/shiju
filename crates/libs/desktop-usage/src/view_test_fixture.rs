use crate::view_types::{UsageModelInfo, UsageStepRawModal};
#[derive(Default)]
pub struct Fixture {
    pub usage: session_ui_types::TokenUsage,
    pub usage_model_info: Option<UsageModelInfo>,
    pub auto_model: bool,
    pub model: String,
    pub session_menu_id: Option<String>,
    pub session_menu_anchor: Option<iced::Point>,
    pub usage_step_expanded: std::collections::HashSet<u32>,
    pub usage_step_raw_loading: std::collections::HashSet<(String, String, u32)>,
    pub usage_step_raw_payloads: std::collections::HashMap<(String, String, u32), Result<Option<serde_json::Value>, String>>,
    pub step_llm_raw: std::collections::HashMap<(String, String, u32), serde_json::Value>,
    pub usage_step_raw_modal: Option<UsageStepRawModal>,
    pub last_copied_code_hash: Option<u64>,
    pub active_session_id: Option<String>,
}
impl Fixture {
    pub fn new() -> (Self, ()) {
        (Self { auto_model: true, ..Self::default() }, ())
    }

    pub fn input(&self) -> crate::view_types::ViewInput<'_> {
        crate::view_types::ViewInput {
            desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
            usage: &self.usage,
            usage_step_expanded: &self.usage_step_expanded,
            usage_step_raw_loading: &self.usage_step_raw_loading,
            usage_step_raw_payloads: &self.usage_step_raw_payloads,
            step_llm_raw: &self.step_llm_raw,
            usage_model_info: self.usage_model_info.as_ref(),
            session_menu_id: self.session_menu_id.as_ref(),
            usage_step_raw_modal: self.usage_step_raw_modal.as_ref(),
            active_session_id: self.active_session_id.as_ref(),
            auto_model: self.auto_model,
            session_menu_anchor: self.session_menu_anchor,
            last_copied_code_hash: self.last_copied_code_hash,
            model: self.model.clone(),
        }
    }
}
