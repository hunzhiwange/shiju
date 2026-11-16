//! Bind source selection to the trusted host catalog and tenant.
impl llm_runtime::acp::source_selection::SelectionHost for super::request::RuntimeHost {
    fn annotate_model_source(options: &mut vw_api_acp_model::acp_session::AcpSessionOptions, agent: &str) { super::controls::annotate_model_source(options, agent); }
    fn resolve_model(model: Option<String>) -> llm_runtime::acp::host::HostFuture<'static, Result<model_types::Model, String>> {
        Box::pin(super::super::prompt::resolve_model(model))
    }
    fn provider(id: &str) -> llm_runtime::acp::host::HostFuture<'_, anyhow::Result<Option<model_types::Info>>> {
        Box::pin(async move { Ok(crate::provider::provider::get_provider(id).await?) })
    }
}
pub(crate) async fn select(session: &str, directory: &str, update: vw_api_acp_model::acp_model_source::AcpModelSourceUpdate) -> Result<vw_api_acp_model::acp_session::AcpSessionOptions, String> {
    llm_runtime::acp::source_selection::select::<super::request::RuntimeHost>(session, directory, update).await
}
