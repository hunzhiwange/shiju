//! Title composition: install the primitive model IO before entering the engine.
pub(crate) use session_engine::title::first_response_source;
pub async fn generate_from_content(session_id: String, content: String, preferred_model: Option<String>, acp_agent: Option<String>) -> Result<String, String> {
    super::llm::prompt::ensure_host();
    session_engine::title::generate_from_content(session_id, content, preferred_model, acp_agent).await
}
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn spawn_first_response_title(runtime: &tokio::runtime::Handle, tenant: vw_api_transport::TenantScope, context: crate::project::instance::Context, session_id: String, preview: String, preferred_model: Option<String>) {
    super::llm::prompt::ensure_host();
    session_engine::title::spawn_first_response_title(runtime, tenant, context, session_id, preview, preferred_model)
}

#[cfg(test)]
#[path = "../../../libs/session-engine/tests/host_suites/title_tests.rs"]
mod title_tests;
