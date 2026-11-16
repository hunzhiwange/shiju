//! Shared catalog owner; preserves type and state identity.
pub use tool_catalog::tool_search::*;

use crate::{Tool, ToolResult};
use tool_context::{host::ContextHost, context::{ToolUseContext, current_tool_use_context}};
use serde_json::Value;
use async_trait::async_trait;

/// Authorized session and successful execution receipt supplied by the concrete host.
pub trait SearchHost: ContextHost {
    fn search_session(context: &ToolUseContext<Self>) -> Option<std::sync::Arc<ToolDiscoverySession>>;
    fn record_search(context: &ToolUseContext<Self>, input: Value);
}

pub struct ContextToolSearchTool<H: SearchHost> {
    metadata: ToolSearchTool,
    host: std::marker::PhantomData<fn() -> H>,
}
impl<H: SearchHost> ContextToolSearchTool<H> {
    pub fn new(catalog: &crate::discovery::ToolDiscoveryCatalog) -> Self {
        Self::from_metadata(ToolSearchTool::new(catalog))
    }
    pub fn from_metadata(metadata: ToolSearchTool) -> Self {
        Self { metadata, host: std::marker::PhantomData }
    }
}
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl<H: SearchHost> Tool for ContextToolSearchTool<H> {
    fn name(&self) -> &str { self.metadata.name() }
    fn description(&self) -> &str { self.metadata.description() }
    fn parameters_schema(&self) -> Value { self.metadata.parameters_schema() }
    fn is_read_only(&self) -> bool { self.metadata.is_read_only() }
    fn is_concurrency_safe(&self) -> bool { self.metadata.is_concurrency_safe() }
    fn validate_input(&self, input: Value) -> anyhow::Result<Value> { self.metadata.validate_input(input) }
    async fn execute(&self, input: Value) -> anyhow::Result<ToolResult> {
        self.metadata.validate_input(input.clone())?;
        let context = current_tool_use_context::<H>()
            .ok_or_else(|| anyhow::anyhow!("tool_search requires an authorized runtime catalog"))?;
        let discovery = H::search_session(&context)
            .ok_or_else(|| anyhow::anyhow!("tool_search requires an authorized runtime catalog"))?;
        let output = discovery.search(input.clone())?;
        H::record_search(&context, input);
        Ok(ToolResult { success: true, output, error: None })
    }
}
