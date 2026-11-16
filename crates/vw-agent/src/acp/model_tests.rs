//! Real configuration persistence and runtime request model boundary.
use super::super::{VibeWindowBackend, start_prompt_with_runtime_runner};
struct NativeModelPorts;
#[async_trait::async_trait]
impl agent_acp::host_migration_53_model_contracts::ModelPorts for NativeModelPorts {
    type Host = super::super::NativeAcpHost;
    async fn save_fixture(tenant: &vw_api_transport::TenantScope, provider: serde_json::Value) {
        let mut config = crate::config::Config::default();
        config.providers.insert("fixture".into(), provider);
        crate::config::save_config_for_tenant(&config, tenant).await.unwrap();
    }
    async fn prompt_selected(backend: &VibeWindowBackend, id: &str) -> acp_server::server::BackendPrompt {
        start_prompt_with_runtime_runner(backend, id, vec![acp_server::server::PromptContent::Text("hello".into())], |request, mut emit| {
            assert_eq!(request.model.as_deref(), Some("fixture/second"));
            emit(crate::session::processor::StreamEvent::Done(crate::session::ui_types::TokenUsage::default()));
        }).await.unwrap()
    }
}
#[tokio::test]
async fn model_selection_reaches_runtime_and_preserves_other_sessions() {
    agent_acp::host_migration_53_model_contracts::model_selection_reaches_runtime_and_preserves_other_sessions::<NativeModelPorts>().await;
}
