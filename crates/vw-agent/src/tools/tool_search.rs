//! Host protocol and task-local execution adapters for shared discovery.
use super::context::current_tool_use_context;
use super::discovery::ToolDiscoveryCatalog;
use super::{Tool, ToolResult, ToolSpec};
use async_trait::async_trait;
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
pub(crate) use tool_assembly::tool_search::TOOL_SEARCH_ID;
pub struct ToolDiscoverySession {
    inner: tool_assembly::tool_search::ToolDiscoverySession,
    #[cfg(not(target_arch = "wasm32"))]
    last_search: Mutex<Option<(String, Value)>>,
}
impl std::ops::Deref for ToolDiscoverySession {
    type Target = tool_assembly::tool_search::ToolDiscoverySession;
    fn deref(&self) -> &Self::Target { &self.inner }
}
impl ToolDiscoverySession {
    pub(crate) fn new(specs: Vec<ToolSpec>) -> Self { Self::wrap(tool_assembly::tool_search::ToolDiscoverySession::new(specs)) }
    fn wrap(inner: tool_assembly::tool_search::ToolDiscoverySession) -> Self { Self { inner, #[cfg(not(target_arch = "wasm32"))] last_search: Mutex::new(None) } }
    pub(crate) fn rebase(&self, specs: Vec<ToolSpec>) -> Self { Self::wrap(self.inner.rebase(specs)) }
    pub(crate) fn search_tool(&self) -> ToolSearchTool { ToolSearchTool(self.inner.search_tool()) }
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn native_output(
        &self,
        call: &crate::session::llm::ToolCall,
    ) -> anyhow::Result<aisdk::core::Message> {
        anyhow::ensure!(self.authorizes(TOOL_SEARCH_ID), "tool_search is not authorized for this request");
        crate::session::llm::native_tools::native_search_output_from_specs(
            call,
            self.inner.authorized_specs(),
            false,
        )
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn prepare_native_search(
        &self,
        call: &crate::session::llm::ToolCall,
    ) -> anyhow::Result<()> {
        self.native_output(call)?;
        // A previous ordinary call cannot supply this invocation's receipt.
        self.last_search.lock().unwrap_or_else(|error| error.into_inner()).take();
        Ok(())
    }

    /// The receipt is written by the real tool executor, never by model or Hook output text.
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn take_native_output(
        &self,
        call: &crate::session::llm::ToolCall,
    ) -> anyhow::Result<aisdk::core::Message> {
        let (id, arguments) =
            self.last_search.lock().unwrap_or_else(|error| error.into_inner()).take().ok_or_else(
                || anyhow::anyhow!("native search has no successful runtime receipt"),
            )?;
        anyhow::ensure!(id == call.id, "native search receipt call_id mismatch");
        let mut executed = call.clone();
        executed.arguments = arguments.to_string();
        self.native_output(&executed)
    }

}
pub(crate) struct ToolSearchTool(tool_assembly::tool_search::ToolSearchTool);
impl ToolSearchTool { pub(crate) fn new(catalog: &ToolDiscoveryCatalog) -> Self { Self(tool_assembly::tool_search::ToolSearchTool::new(catalog)) } }
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for ToolSearchTool {
    fn name(&self) -> &str { self.0.name() }
    fn description(&self) -> &str { self.0.description() }
    fn parameters_schema(&self) -> Value { self.0.parameters_schema() }
    fn is_read_only(&self) -> bool { self.0.is_read_only() }
    fn is_concurrency_safe(&self) -> bool { self.0.is_concurrency_safe() }
    fn validate_input(&self, input: Value) -> anyhow::Result<Value> { self.0.validate_input(input) }
    async fn execute(&self, input: Value) -> anyhow::Result<ToolResult> {
        self.0.validate_input(input.clone())?;
        let context = current_tool_use_context()
            .ok_or_else(|| anyhow::anyhow!("tool_search requires an authorized runtime catalog"))?;
        let discovery = context
            .tool_discovery()
            .ok_or_else(|| anyhow::anyhow!("tool_search requires an authorized runtime catalog"))?;
        let output = discovery.inner.search(input.clone())?;
        #[cfg(not(target_arch = "wasm32"))]
        if let Some(id) = context.tool_call_id() {
            *discovery.last_search.lock().unwrap_or_else(|error| error.into_inner()) =
                Some((id.to_string(), input));
        }
        Ok(ToolResult { success: true, output, error: None })
    }
}

#[cfg(test)]
#[path = "tool_search_tests.rs"]
mod tests;
