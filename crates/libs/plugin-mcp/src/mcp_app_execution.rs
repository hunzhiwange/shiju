//! Host-issued app bindings retain the exact session tool and approval context.
use std::sync::Arc;
use tool_types::ToolCallResult;
use tokio_util::sync::CancellationToken;

/// Session-owned execution retained without exposing application runtime types.
pub trait AppContext: Send + Sync {
    fn session(&self) -> &str;
    fn root(&self) -> Option<&str>;
    fn abort_token(&self) -> Option<CancellationToken>;
    fn tool_id(&self, server: &str, name: &str) -> Result<String>;
    fn contains_tool(&self, id: &str) -> bool;
    fn prepare_approval(&self, cancelled: CancellationToken) -> Arc<dyn AppContext>;
    fn drain_prompts(&self);
    fn execute(&self, id: &str, input: &str) -> Result<ToolCallResult>;
}
use anyhow::{Result, bail};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::Path,
    sync::{Mutex, OnceLock},
};
use vw_api_transport::id::TenantScope;

#[derive(Clone)]
struct Binding {
    tenant: TenantScope,
    context: Arc<dyn AppContext>,
    server: String,
    cancelled: CancellationToken,
}
fn bindings() -> &'static Mutex<HashMap<String, Binding>> {
    static BINDINGS: OnceLock<Mutex<HashMap<String, Binding>>> = OnceLock::new();
    BINDINGS.get_or_init(Default::default)
}

pub fn register(
    context: Arc<dyn AppContext>,
    tenant: TenantScope,
    requested: &str,
    result: &mut ToolCallResult,
) -> Result<()> {
    let Some(app) = result.data.get_mut("mcp_app") else { return Ok(()) };
    let server = app["server"].as_str().ok_or_else(|| anyhow::anyhow!("missing app server"))?;
    let tool = app["tool"].as_str().ok_or_else(|| anyhow::anyhow!("missing app tool"))?;
    if requested != context.tool_id(server, tool)?
        || !context.contains_tool(requested)
    {
        bail!("MCP app requires a session-owned tool");
    }
    let cancelled = context.abort_token().map(|token| token.child_token()).unwrap_or_default();
    let context = context.prepare_approval(cancelled.clone());
    let binding = Binding {
        cancelled,
        tenant,
        context,
        server: server.to_owned(),
    };
    let mut entries = bindings().lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    if entries.len() >= 256 {
        bail!("too many active MCP apps; close a session");
    }
    let handle = uuid::Uuid::new_v4().to_string();
    entries.insert(handle.clone(), binding);
    app["execution_binding"] = Value::String(handle);
    Ok(())
}

pub fn revoke_for_tenant(tenant: &TenantScope, session: &str, root: &Path) {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_owned());
    bindings().lock().unwrap_or_else(std::sync::PoisonError::into_inner).retain(|_, binding| {
        let remove = &binding.tenant == tenant
            && binding.context.session() == session
            && binding.context.root().is_some_and(|dir| {
                Path::new(dir).canonicalize().unwrap_or_else(|_| dir.into()) == root
            });
        if remove {
            binding.cancelled.cancel();
        }
        !remove
    });
}

pub fn call(
    tenant: &TenantScope,
    session: &str,
    server: &str,
    handle: &str,
    name: &str,
    arguments: Value,
) -> Result<Value> {
    let binding = bindings()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .get(handle)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("MCP app binding expired"))?;
    if &binding.tenant != tenant
        || binding.context.session() != session
        || binding.server != server
        || !arguments.is_object()
    {
        bail!("MCP app session mismatch or invalid arguments");
    }
    if binding.context.abort_token().is_some_and(|token| token.is_cancelled()) {
        bail!("MCP app turn cancelled");
    }
    binding.context.drain_prompts();
    let id = binding.context.tool_id(&binding.server, name)?;
    if !binding.context.contains_tool(&id) {
        bail!("MCP app tool absent from bound session");
    }
    let result =
        binding.context.execute(&id, &arguments.to_string())?;
    if binding.cancelled.is_cancelled() {
        bail!("MCP app binding revoked during execution");
    }
    result
        .data
        .get("mcp_result")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("MCP tool did not return a protocol result"))
}
