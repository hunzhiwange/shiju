//! 推理与事件订阅兼容门面。

pub use gateway_inference::*;
use super::GatewayClient;
use serde_json::Value;
use vw_api_agent_scheduler::AgentSchedulerScopeDto;

impl GatewayClient {
    pub async fn inference_agent(
        &self,
        request: &GatewayAgentRequest,
    ) -> Result<GatewayAgentResponse, String> {
        gateway_inference::GatewayInferenceClient::new(self.transport.clone())
            .inference_agent(request).await
    }

    pub async fn inference_openai_chat(&self, request: &Value) -> Result<Value, String> {
        gateway_inference::GatewayInferenceClient::new(self.transport.clone())
            .inference_openai_chat(request).await
    }

    pub async fn stream_gateway_events(
        &self,
        on_event: impl FnMut(crate::stream::GatewayJsonSseEvent) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        gateway_inference::GatewayInferenceClient::new(self.transport.clone())
            .stream_gateway_events(on_event).await
    }

    pub async fn stream_global_events(
        &self,
        on_event: impl FnMut(crate::stream::GatewayJsonSseEvent) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        gateway_inference::GatewayInferenceClient::new(self.transport.clone())
            .stream_global_events(on_event).await
    }

    pub async fn stream_instance_events(
        &self,
        directory: Option<&str>,
        on_event: impl FnMut(crate::stream::GatewayJsonSseEvent) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        gateway_inference::GatewayInferenceClient::new(self.transport.clone())
            .stream_instance_events(directory, on_event).await
    }

    pub async fn stream_agent_scheduler_events(
        &self,
        scope: &AgentSchedulerScopeDto,
        after_cursor: Option<i64>,
        on_event: impl FnMut(crate::stream::GatewayJsonSseEvent) -> bool,
    ) -> Result<crate::stream::GatewayStreamAttemptResult, String> {
        gateway_inference::GatewayInferenceClient::new(self.transport.clone())
            .stream_agent_scheduler_events(scope, after_cursor, on_event).await
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub async fn connect_chat_websocket(&self) -> Result<GatewayChatWebSocket, String> {
        gateway_inference::GatewayInferenceClient::new(self.transport.clone())
            .connect_chat_websocket().await
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "inference_facade_tests.rs"]
mod inference_facade_tests;
