//! Client-sourced tools use the ordinary permission, approval and hook executor.

use tool_types::{Tool, ToolCallResult, ToolResult, ToolSpec};
use tool_context::{context::current_tool_use_context, host::ContextHost};
use std::path::Path;
use vw_api_transport::TenantScope;

/// Authenticated connection; its original owner and pending calls remain host-owned.
pub trait ClientConnection: tool_context::host::HostValue + 'static {
    fn owner_tenant(&self) -> &TenantScope;
    fn owner_session(&self) -> &str;
    fn owner_directory(&self) -> &Path;
    fn request<'a>(self: &'a Arc<Self>, spec: &'a ToolSpec, args: Value, turn_id: String)
        -> impl Future<Output = anyhow::Result<vw_api_tools::DynamicToolResponse>> + tool_context::host::HostFuture + 'a;
}
use serde_json::{Value, json};
use std::sync::Arc;
use vw_api_tools::{DynamicToolContent, DynamicToolRegistration, ToolResultContentDto};

const MAX_CLIENT_SCHEMA_BYTES: usize = 64 * 1024;

pub struct ClientTool<H: ContextHost, C: ClientConnection> {
    context: std::marker::PhantomData<H>,
    spec: ToolSpec,
    host: Arc<C>,
}

impl<H: ContextHost, C: ClientConnection> ClientTool<H, C> {
    pub fn new(spec: ToolSpec, host: Arc<C>) -> anyhow::Result<Self> {
        DynamicToolRegistration::Function(spec.to_dto()).validate().map_err(anyhow::Error::msg)?;
        if serde_json::to_vec(&spec.input_schema)?.len() > MAX_CLIENT_SCHEMA_BYTES {
            anyhow::bail!("client tool schema exceeds byte limit");
        }
        Ok(Self { spec, host, context: std::marker::PhantomData })
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<H: ContextHost, C: ClientConnection> Tool for ClientTool<H, C> {
    fn name(&self) -> &str {
        &self.spec.id
    }
    fn description(&self) -> &str {
        &self.spec.description
    }
    fn parameters_schema(&self) -> Value {
        self.spec.input_schema.clone()
    }
    fn spec(&self) -> ToolSpec {
        self.spec.clone()
    }
    async fn execute(&self, _args: Value) -> anyhow::Result<ToolResult> {
        anyhow::bail!("client tools require the structured Tool::call executor");
    }
    async fn call(&self, args: Value) -> anyhow::Result<ToolCallResult> {
        let context = current_tool_use_context::<H>()
            .ok_or_else(|| anyhow::anyhow!("client tool requires execution context"))?;
        let root = context
            .workspace_root()
            .ok_or_else(|| anyhow::anyhow!("client tool requires workspace scope"))?;
        if context.tenant_scope() != *self.host.owner_tenant()
            || context.session() != self.host.owner_session()
            || std::fs::canonicalize(root)? != self.host.owner_directory()
        {
            anyhow::bail!("client tool execution owner mismatch");
        }
        let turn_id = context
            .message_id()
            .filter(|id| !id.trim().is_empty())
            .ok_or_else(|| anyhow::anyhow!("client tool requires a turn ID"))?;
        let response = self.host.request(&self.spec, args, turn_id.into()).await?;
        // The shared result DTO represents media as typed JSON blocks. Preserve the
        // original tagged content in model_result as well, without flattening URLs.
        let model_result = serde_json::to_value(&response.content)?;
        let content_blocks = response
            .content
            .into_iter()
            .map(|content| match content {
                DynamicToolContent::Text { text } => Ok(ToolResultContentDto::Text { text }),
                media => Ok(ToolResultContentDto::Json { value: serde_json::to_value(media)? }),
            })
            .collect::<anyhow::Result<Vec<_>>>()?;
        Ok(ToolCallResult {
            data: json!({"success":response.success}),
            model_result,
            content_blocks,
            ..ToolCallResult::default()
        })
    }
}
