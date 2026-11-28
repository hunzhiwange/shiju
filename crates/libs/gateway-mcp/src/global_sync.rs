//! Shared request types for global MCP synchronization.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use vw_api_app_identity::AppId;
use vw_api_mcp_management::{
    McpAppCapability, McpConfigValue, McpConnectionDefinition, McpFailureCode, McpPresetDraftView,
};

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Options {
    pub presets: Vec<McpPresetDraftView>,
    pub applications: Vec<McpAppCapability>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncRequest {
    pub target_host_id: String,
    pub name: String,
    pub applications: Vec<AppId>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyncItem {
    pub app_id: AppId,
    pub failure: Option<McpFailureCode>,
}

impl super::GatewayMcpClient {
    pub async fn global_options(&self, host: &str) -> Result<Options, String> {
        self.transport
            .get_json("/v1/global/mcp/options", &[("targetHostId".into(), host.into())])
            .await
    }

    pub async fn sync_global(&self, request: &SyncRequest) -> Result<Vec<SyncItem>, String> {
        self.transport.post_json("/v1/global/mcp/sync", &[], request).await
    }
}

pub fn connection(server: &Value) -> Result<McpConnectionDefinition, &'static str> {
    let values = |key: &str| {
        let mut values = std::collections::BTreeMap::new();
        if let Some(entries) = server.get(key) {
            for entry in entries.as_array().ok_or("连接定义无效")? {
                let name = entry["name"].as_str().ok_or("连接定义无效")?;
                let value = entry["value"].as_str().ok_or("连接定义无效")?;
                if values.insert(name.into(), McpConfigValue::Literal { value: value.into() }).is_some() {
                    return Err("字段名已存在");
                }
            }
        }
        Ok(values)
    };
    let url = || server["url"].as_str().map(str::to_owned).ok_or("连接定义无效");
    match server["type"].as_str().unwrap_or("stdio") {
        "http" => Ok(McpConnectionDefinition::StreamableHttp { url: url()?, headers: values("headers")? }),
        "sse" => Ok(McpConnectionDefinition::Sse { url: url()?, headers: values("headers")? }),
        "stdio" => Ok(McpConnectionDefinition::Stdio {
            command: server["command"].as_str().ok_or("连接定义无效")?.to_owned(),
            args: server.get("args").map(|args| serde_json::from_value(args.clone()))
                .transpose().map_err(|_| "连接定义无效")?.unwrap_or_default(),
            env: values("env")?,
            cwd: None,
        }),
        _ => Err("不支持的传输"),
    }
}
