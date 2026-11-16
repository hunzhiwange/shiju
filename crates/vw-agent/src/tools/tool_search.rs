//! Host protocol and task-local execution adapters for shared discovery.
use super::ToolSpec;
use serde_json::Value;
#[cfg(not(target_arch = "wasm32"))]
use std::sync::Mutex;
pub(crate) use tool_assembly::tool_search::TOOL_SEARCH_ID;
pub struct ToolDiscoverySession {
    inner: std::sync::Arc<tool_assembly::tool_search::ToolDiscoverySession>,
    #[cfg(not(target_arch = "wasm32"))]
    last_search: Mutex<Option<(String, Value)>>,
}
impl std::ops::Deref for ToolDiscoverySession {
    type Target = tool_assembly::tool_search::ToolDiscoverySession;
    fn deref(&self) -> &Self::Target { &self.inner }
}
impl ToolDiscoverySession {
    pub(crate) fn new(specs: Vec<ToolSpec>) -> Self { Self::wrap(tool_assembly::tool_search::ToolDiscoverySession::new(specs)) }
    fn wrap(inner: tool_assembly::tool_search::ToolDiscoverySession) -> Self { Self { inner: std::sync::Arc::new(inner), #[cfg(not(target_arch = "wasm32"))] last_search: Mutex::new(None) } }
    pub(crate) fn rebase(&self, specs: Vec<ToolSpec>) -> Self { Self::wrap(self.inner.rebase(specs)) }
    pub(crate) fn search_tool(&self) -> ToolSearchTool { ToolSearchTool::from_metadata(self.inner.search_tool()) }
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

pub(crate) type ToolSearchTool = tool_assembly::tool_search::ContextToolSearchTool<super::context::AgentContextHost>;
impl tool_assembly::tool_search::SearchHost for super::context::AgentContextHost {
    fn search_session(context: &super::context::ToolUseContext) -> Option<std::sync::Arc<tool_assembly::tool_search::ToolDiscoverySession>> {
        context.tool_discovery().map(|discovery| discovery.inner.clone())
    }
    fn record_search(context: &super::context::ToolUseContext, input: Value) {
        #[cfg(not(target_arch = "wasm32"))]
        if let (Some(id), Some(discovery)) = (context.tool_call_id(), context.tool_discovery()) {
            *discovery.last_search.lock().unwrap_or_else(|error| error.into_inner()) = Some((id.to_string(), input));
        }
        #[cfg(target_arch = "wasm32")]
        let _ = (context, input);
    }
}
