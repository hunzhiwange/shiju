//! Task-scoped host bindings for ACP session controls.
use vw_api_acp_model::acp_session::{AcpSessionOptions, AcpSessionSelection};
#[cfg(any(test, target_arch = "wasm32"))]
pub(super) use llm_runtime::acp::controls::{is_model_config, is_managed_effort_config, is_permission_config, has_saved_config_category, prepare_access_restore};
fn host() -> llm_runtime::acp::controls::Host {
    llm_runtime::acp::controls::Host {
        tenant: crate::session::ui_store::current_tenant_scope,
        open: super::lifecycle::Store::open,
        session_name: super::session::acp_session_name,
        verified_release: super::model_source::drivers::verified_release,
        capabilities: super::model_source::drivers::capabilities,
    }
}
pub(crate) fn annotate_model_source(options: &mut AcpSessionOptions, agent: &str) {
    llm_runtime::acp::controls::annotate_model_source(&host(), options, agent)
}
pub(crate) async fn read(session_id: &str, directory: &str) -> Result<AcpSessionOptions, String> {
    llm_runtime::acp::controls::read(&host(), session_id, directory).await
}
pub(crate) async fn select(session_id: &str, directory: &str, selection: AcpSessionSelection) -> Result<AcpSessionOptions, String> {
    llm_runtime::acp::controls::select(&host(), session_id, directory, selection).await
}
#[cfg(test)]
use std::sync::Arc;
#[cfg(test)]
#[path = "controls_tests.rs"]
mod tests;
