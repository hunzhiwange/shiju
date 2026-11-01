//! Host-issued app bindings retain the exact session tool and approval context.
use crate::tools::ToolRuntimeContext;
use anyhow::{Result, bail};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::Path,
    sync::{Mutex, OnceLock},
};
use vw_api_types::id::TenantScope;

#[derive(Clone)]
struct Binding {
    tenant: TenantScope,
    context: ToolRuntimeContext,
    server: String,
    cancelled: tokio_util::sync::CancellationToken,
    prompts: std::sync::Arc<
        Mutex<
            tokio::sync::mpsc::UnboundedReceiver<
                crate::agent::loop_::approval::NonCliApprovalPrompt,
            >,
        >,
    >,
}
fn bindings() -> &'static Mutex<HashMap<String, Binding>> {
    static BINDINGS: OnceLock<Mutex<HashMap<String, Binding>>> = OnceLock::new();
    BINDINGS.get_or_init(Default::default)
}

pub(crate) fn register(
    context: &ToolRuntimeContext,
    requested: &str,
    result: &mut crate::tools::ToolCallResult,
) -> Result<()> {
    let Some(app) = result.data.get_mut("mcp_app") else { return Ok(()) };
    let server = app["server"].as_str().ok_or_else(|| anyhow::anyhow!("missing app server"))?;
    let tool = app["tool"].as_str().ok_or_else(|| anyhow::anyhow!("missing app tool"))?;
    if requested != crate::tools::mcp_tool::mcp_tool_id(server, tool)?
        || !context.session_tools().tools().iter().any(|tool| tool.name() == requested)
    {
        bail!("MCP app requires a session-owned tool");
    }
    // A page must not inherit the model turn's full-access or approval bypass.
    let approval = (*context.tool_use_context())
        .clone()
        .with_full_access_enabled(false)
        .with_bypass_non_cli_approval_for_turn(false);
    let cancelled = context
        .tool_use_context()
        .abort_token()
        .map(|token| token.child_token())
        .unwrap_or_default();
    let (prompt_tx, prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let approval = approval
        .with_abort_token(cancelled.clone())
        .with_channel("desktop")
        .with_non_cli_approval_context(crate::agent::loop_::NonCliApprovalContext {
            sender: "desktop".into(),
            reply_target: context.session.clone(),
            prompt_tx,
        });
    let binding = Binding {
        cancelled,
        prompts: std::sync::Arc::new(Mutex::new(prompt_rx)),
        tenant: crate::session::ui_store::current_tenant_scope(),
        context: context.clone().with_tool_use_context(approval),
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

pub(crate) fn revoke(session: &str, root: &Path) {
    let tenant = crate::session::ui_store::current_tenant_scope();
    let root = root.canonicalize().unwrap_or_else(|_| root.to_owned());
    bindings().lock().unwrap_or_else(std::sync::PoisonError::into_inner).retain(|_, binding| {
        let remove = binding.tenant == tenant
            && binding.context.session == session
            && binding.context.root.as_deref().is_some_and(|dir| {
                Path::new(dir).canonicalize().unwrap_or_else(|_| dir.into()) == root
            });
        if remove {
            binding.cancelled.cancel();
        }
        !remove
    });
}

pub(crate) fn call(
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
    if binding.tenant != crate::session::ui_store::current_tenant_scope()
        || binding.context.session != session
        || binding.server != server
        || !arguments.is_object()
    {
        bail!("MCP app session mismatch or invalid arguments");
    }
    if binding.context.tool_use_context().abort_token().is_some_and(|token| token.is_cancelled()) {
        bail!("MCP app turn cancelled");
    }
    while binding
        .prompts
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .try_recv()
        .is_ok()
    {}
    let id = crate::tools::mcp_tool::mcp_tool_id(&binding.server, name)?;
    if !binding.context.session_tools().tools().iter().any(|tool| tool.name() == id) {
        bail!("MCP app tool absent from bound session");
    }
    let result =
        crate::tools::execute_bound_app_tool(&id, &arguments.to_string(), &binding.context)?;
    if binding.cancelled.is_cancelled() {
        bail!("MCP app binding revoked during execution");
    }
    result
        .data
        .get("mcp_result")
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("MCP tool did not return a protocol result"))
}
