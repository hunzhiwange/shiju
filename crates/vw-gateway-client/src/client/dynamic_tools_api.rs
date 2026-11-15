//! 动态工具宿主兼容门面。
use super::GatewayClient;
pub use gateway_host_tools::GatewayHostToolsClient;
use vw_api_tools::{DynamicToolCallRequest, DynamicToolHostBinding, DynamicToolResponse, ReplaceDynamicToolsRequest, ReplaceDynamicToolsResponse};
use crate::stream::GatewayStreamAttemptResult;

impl GatewayClient {
    pub async fn register_dynamic_tool_host(
        &self,
        session_id: &str,
        directory: Option<&str>,
    ) -> Result<DynamicToolHostBinding, String> {
        GatewayHostToolsClient::new(self.transport.clone()).register_dynamic_tool_host(session_id, directory).await
    }

    pub async fn replace_dynamic_tools(
        &self,
        session_id: &str,
        directory: Option<&str>,
        binding: &DynamicToolHostBinding,
        body: &ReplaceDynamicToolsRequest,
    ) -> Result<ReplaceDynamicToolsResponse, String> {
        GatewayHostToolsClient::new(self.transport.clone()).replace_dynamic_tools(session_id, directory, binding, body).await
    }

    pub async fn respond_dynamic_tool_call(
        &self,
        session_id: &str,
        directory: Option<&str>,
        binding: &DynamicToolHostBinding,
        call_id: &str,
        body: &DynamicToolResponse,
    ) -> Result<(), String> {
        GatewayHostToolsClient::new(self.transport.clone()).respond_dynamic_tool_call(session_id, directory, binding, call_id, body).await
    }

    pub async fn stream_dynamic_tool_calls(
        &self,
        session_id: &str,
        directory: Option<&str>,
        binding: &DynamicToolHostBinding,
        on_call: impl FnMut(DynamicToolCallRequest) -> bool,
    ) -> Result<GatewayStreamAttemptResult, String> {
        GatewayHostToolsClient::new(self.transport.clone()).stream_dynamic_tool_calls(session_id, directory, binding, on_call).await
    }

}

#[cfg(test)]
#[path = "dynamic_tools_facade_tests.rs"]
mod tests;
