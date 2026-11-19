//! Pure paste conversion. Never reads files, resolves credentials or starts processes.
use serde_json::Value;
use std::collections::BTreeMap;
use vw_api_mcp_management::*;
#[derive(Clone)]
pub struct Candidate {
    pub id: String,
    pub connection: Result<McpConnectionDefinition, String>,
}
pub fn parse(source: &str) -> Result<Vec<Candidate>, String> {
    if source.len() > 1024 * 1024 {
        return Err("粘贴内容超过 1 MiB".into());
    }
    let root: Value = match serde_json::from_str(source) {
        Ok(value) => value,
        Err(error) if source.trim_start().starts_with('{') => {
            return Err(format!("JSON 无效：第 {} 行，第 {} 列", error.line(), error.column()));
        }
        Err(_) => {
            let value: toml::Value =
                toml::from_str(source).map_err(|_| "TOML 无效，请检查表名及字段类型")?;
            serde_json::to_value(value).map_err(|_| "TOML 无法转换")?
        }
    };
    let entries =
        root.get("mcpServers").or_else(|| root.get("mcp_servers")).or_else(|| root.get("mcp"));
    if let Some(entries) = entries {
        let entries = entries.as_object().ok_or("服务器列表必须为对象")?;
        if entries.len() > 256 {
            return Err("一次最多粘贴 256 项".into());
        }
        Ok(entries
            .iter()
            .map(|(id, value)| Candidate { id: id.clone(), connection: connection(value) })
            .collect())
    } else {
        let id = root
            .get("name")
            .or_else(|| root.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("pasted-server")
            .to_owned();
        Ok(vec![Candidate { id, connection: connection(&root) }])
    }
}
fn string(value: &Value, key: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("缺少有效 {key}"))
}
fn values(value: Option<&Value>) -> Result<BTreeMap<String, McpConfigValue>, String> {
    let Some(value) = value else {
        return Ok(BTreeMap::new());
    };
    value
        .as_object()
        .ok_or("环境或请求头必须为对象")?
        .iter()
        .map(|(key, value)| {
            let value = if let Some(value) = value.as_str() {
                McpConfigValue::Literal { value: value.into() }
            } else {
                serde_json::from_value(value.clone()).map_err(|_| "凭据引用无效")?
            };
            Ok((key.clone(), value))
        })
        .collect()
}
fn connection(value: &Value) -> Result<McpConnectionDefinition, String> {
    if let Ok(connection) = serde_json::from_value::<McpConnectionDefinition>(value.clone()) {
        return Ok(connection);
    }
    let kind = value.get("type").or_else(|| value.get("transport")).and_then(Value::as_str);
    if matches!(kind, Some("stdio" | "local")) || kind.is_none() && value.get("command").is_some() {
        let (command, args) = if let Some(command) = value.get("command").and_then(Value::as_array)
        {
            let strings: Vec<String> = command
                .iter()
                .map(|v| v.as_str().map(str::to_owned).ok_or("命令参数必须为字符串"))
                .collect::<Result<_, _>>()?;
            let (command, args) = strings.split_first().ok_or("命令不能为空")?;
            (command.clone(), args.to_vec())
        } else {
            (
                string(value, "command")?,
                value
                    .get("args")
                    .map(|args| serde_json::from_value(args.clone()))
                    .transpose()
                    .map_err(|_| "args 必须为字符串数组")?
                    .unwrap_or_default(),
            )
        };
        Ok(McpConnectionDefinition::Stdio {
            command,
            args,
            env: values(value.get("env").or_else(|| value.get("environment")))?,
            cwd: value.get("cwd").map(|_| string(value, "cwd")).transpose()?,
        })
    } else if matches!(
        kind,
        None | Some("http" | "streamable_http" | "streamable-http" | "sse" | "remote")
    ) {
        let url = string(value, "url")?;
        let headers = values(value.get("headers").or_else(|| value.get("http_headers")))?;
        if kind == Some("sse") {
            Ok(McpConnectionDefinition::Sse { url, headers })
        } else {
            Ok(McpConnectionDefinition::StreamableHttp { url, headers })
        }
    } else {
        Err("不支持的连接方式".into())
    }
}
