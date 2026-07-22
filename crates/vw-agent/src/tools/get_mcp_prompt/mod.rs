//! 读取指定 MCP prompt 的工具实现。

use super::mcp_common::{
    find_server_by_name, load_resolved_acp_config, server_summary, with_server_peer,
};
use super::traits::{Tool, ToolResult, ToolSpec};
use async_trait::async_trait;
use rmcp::model::{
    GetPromptRequestParam, JsonObject, PromptMessage, PromptMessageContent, PromptMessageRole,
};
use rmcp::service::Peer;
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
struct Args {
    server: String,
    name: String,
    #[serde(default)]
    arguments: Option<Value>,
}

/// 获取指定 MCP prompt 消息的只读工具。
pub struct GetMcpPromptTool;

impl GetMcpPromptTool {
    /// 创建 MCP prompt 获取工具。
    pub fn new() -> Self {
        Self
    }
}

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Tool for GetMcpPromptTool {
    fn name(&self) -> &str {
        "GetMcpPrompt"
    }

    fn description(&self) -> &str {
        "按 server 和 prompt 名称获取 MCP prompt 消息。"
    }

    fn parameters_schema(&self) -> Value {
        json!({
            "type": "object",
            "additionalProperties": false,
            "properties": {
                "server": {
                    "type": "string",
                    "description": "MCP server 名称。"
                },
                "name": {
                    "type": "string",
                    "description": "Prompt 名称。"
                },
                "arguments": {
                    "type": "object",
                    "additionalProperties": true,
                    "description": "传递给 prompt 的可选参数对象。"
                }
            },
            "required": ["server", "name"]
        })
    }

    fn spec(&self) -> ToolSpec {
        ToolSpec::new(self.name(), self.description(), self.parameters_schema())
            .with_display_name("GetMcpPrompt")
            .with_aliases(vec!["get_mcp_prompt".to_string()])
            .with_read_only(true)
            .with_destructive(false)
            .with_concurrency_safe(false)
            .with_requires_user_interaction(false)
            .with_strict(true)
    }

    async fn execute(&self, args: Value) -> anyhow::Result<ToolResult> {
        let args: Args = serde_json::from_value(args)?;
        let server_name = args.server.trim();
        let prompt_name = args.name.trim();
        if server_name.is_empty() || prompt_name.is_empty() {
            anyhow::bail!("'server' and 'name' must not be empty");
        }
        let arguments = prompt_arguments(args.arguments)?;

        let config = load_resolved_acp_config().await?;
        let server = find_server_by_name(&config.mcp_servers, server_name)?;
        let result = with_server_peer(server, |peer: &Peer<rmcp::service::RoleClient>| {
            let prompt_name = prompt_name.to_string();
            let arguments = arguments.clone();
            Box::pin(async move {
                let result =
                    peer.get_prompt(GetPromptRequestParam { name: prompt_name, arguments }).await?;
                Ok(result)
            })
        })
        .await?;

        let data = json!({
            "server": server_summary(server),
            "name": prompt_name,
            "description": result.description,
            "messages": prompt_messages_json(&result.messages),
        });
        Ok(ToolResult { success: true, output: serde_json::to_string_pretty(&data)?, error: None })
    }
}

fn prompt_arguments(arguments: Option<Value>) -> anyhow::Result<Option<JsonObject>> {
    match arguments {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Object(arguments)) => Ok(Some(arguments)),
        Some(_) => anyhow::bail!("'arguments' must be a JSON object when provided"),
    }
}

pub(crate) fn prompt_messages_json(messages: &[PromptMessage]) -> Value {
    Value::Array(
        messages
            .iter()
            .map(|message| {
                json!({
                    "role": prompt_message_role_json(&message.role),
                    "content": prompt_message_content_json(&message.content),
                })
            })
            .collect(),
    )
}

fn prompt_message_role_json(role: &PromptMessageRole) -> &'static str {
    match role {
        PromptMessageRole::User => "user",
        PromptMessageRole::Assistant => "assistant",
    }
}

fn prompt_message_content_json(content: &PromptMessageContent) -> Value {
    match content {
        PromptMessageContent::Text { text } => json!({
            "type": "text",
            "text": text,
        }),
        PromptMessageContent::Image { image } => tagged_content_json("image", image),
        PromptMessageContent::Resource { resource } => {
            let resource = serde_json::to_value(&resource.resource).unwrap_or_else(|_| Value::Null);
            json!({
                "type": "resource",
                "resource": resource,
            })
        }
        PromptMessageContent::ResourceLink { link } => tagged_content_json("resource_link", link),
    }
}

fn tagged_content_json<T>(content_type: &str, content: &T) -> Value
where
    T: serde::Serialize,
{
    let mut value =
        serde_json::to_value(content).unwrap_or_else(|_| Value::Object(Default::default()));
    if let Value::Object(fields) = &mut value {
        fields.insert("type".to_string(), Value::String(content_type.to_string()));
    }
    value
}

#[cfg(test)]
mod tests;
