//! Execute public MCP hooks through the host tool approval boundary.
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{path::Path, sync::Arc, time::Duration};

pub(crate) async fn execute(
    root: &Path,
    handler: &super::hook_manifest::CodexHookHandler,
    event: &Value,
) -> Result<Vec<u8>> {
    use super::hook_manifest::CodexHookHandler;
    use crate::tools::{
        Tool,
        mcp_tool::{McpTool, discover_mcp_tool_specs, mcp_tool_id},
    };
    let CodexHookHandler::McpTool { server, tool, input, timeout, .. } = handler else {
        anyhow::bail!("expected MCP hook")
    };
    ensure!(!server.trim().is_empty() && !tool.trim().is_empty(), "empty MCP hook target");
    let context =
        crate::tools::current_tool_use_context().context("MCP hook requires active turn")?;
    let workspace = context.workspace_root().context("MCP hook requires workspace")?;
    let timeout = if event["hook_event_name"] == "Interrupt" {
        timeout.unwrap_or(1).clamp(1, 3)
    } else {
        timeout.unwrap_or(5).max(1)
    };
    let duration = Duration::from_secs(timeout);
    let execution = async {
        let config = crate::tools::mcp_common::load_resolved_acp_config_from(&workspace).await?;
        let selected = config
            .mcp_servers
            .iter()
            .find(|candidate| {
                if let agent_client_protocol::McpServer::Http(http) = candidate {
                    return super::mcp_auth::hook_source_matches(http, root, server)
                        .unwrap_or(false);
                }
                false
            })
            .context("authorized public MCP hook server unavailable for this plugin")?
            .clone();
        let name = crate::tools::mcp_common::server_name(&selected).to_owned();
        let pool = crate::plugins::mcp_session_connections(context.session(), &workspace);
        let query_name = name.clone();
        let specs = pool
            .with_peer(&selected, Some(duration), |peer| {
                Box::pin(async move { discover_mcp_tool_specs(peer, &query_name, None, &[]).await })
            })
            .await?;
        let id = mcp_tool_id(&name, tool)?;
        let spec =
            specs.into_iter().find(|spec| spec.id == id).context("MCP hook tool unavailable")?;
        let bound: Box<dyn Tool> =
            Box::new(McpTool::from_session_server(selected, tool, spec, pool, Some(duration)));
        let input = expand(&Value::Object(input.clone()), event)?;
        let isolated = Arc::new(
            context.as_ref().clone().with_hook_runner(Arc::new(crate::hooks::HookRunner::new())),
        );
        let result =
            crate::tools::execute_tool_from_registry(&[bound], &id, input, isolated).await?;
        ensure!(result.result.is_success(), "MCP hook tool failed");
        Ok(result.result.model_text().into_bytes())
    };
    tokio::time::timeout(duration, execution).await.context("MCP hook timed out")?
}

pub(crate) fn expand(value: &Value, event: &Value) -> Result<Value> {
    Ok(match value {
        Value::Object(map) => Value::Object(
            map.iter().map(|(k, v)| Ok((k.clone(), expand(v, event)?))).collect::<Result<_>>()?,
        ),
        Value::Array(items) => {
            Value::Array(items.iter().map(|v| expand(v, event)).collect::<Result<_>>()?)
        }
        Value::String(text) => {
            let pattern = regex::Regex::new(r"\$\{([^{}]+)\}")?;
            let resolve = |path: &str| -> Result<Value> {
                let mut current = event;
                for part in path.split('.') {
                    current = current.get(part).context("missing MCP hook template field")?;
                }
                Ok(current.clone())
            };
            if let Some(capture) = pattern.captures(text)
                && capture.get(0).unwrap().as_str() == text
            {
                resolve(&capture[1])?
            } else {
                let mut output = String::new();
                let mut previous = 0;
                for capture in pattern.captures_iter(text) {
                    let matched = capture.get(0).unwrap();
                    output.push_str(&text[previous..matched.start()]);
                    let value = resolve(&capture[1])?;
                    output.push_str(
                        &value.as_str().map(str::to_owned).unwrap_or_else(|| value.to_string()),
                    );
                    previous = matched.end();
                }
                output.push_str(&text[previous..]);
                Value::String(output)
            }
        }
        _ => value.clone(),
    })
}
