//! Install the official Chrome DevTools MCP package on demand.

use config_tools::CHROME_DEVTOOLS_MCP_SERVER_ID as NAME;
use desktop_config::ConfigAccess;
use gateway_endpoint::GatewayEndpoint;
use serde_json::json;
use std::{process::Stdio, time::Duration};

const ARGS: &[&str] = &[
    "-y",
    "chrome-devtools-mcp@1.13.0",
    "--isolated",
    "--no-usage-statistics",
    "--no-performance-crux",
];

pub(super) async fn install(endpoint: GatewayEndpoint) -> Result<(), String> {
    if !endpoint.is_loopback() {
        return Err("Chrome DevTools MCP 一键安装需要本机网关".into());
    }

    let access = ConfigAccess::new(endpoint, None);
    let config = access.load_global_config_value_async().await?;
    let (npx, path) = tokio::task::spawn_blocking(|| {
        let program = if cfg!(windows) { "npx.cmd" } else { "npx" };
        let npx = shell_command::resolve_executable(program)
            .ok_or("找不到 npx，请安装 Node.js LTS 和 npm")?;
        Ok::<_, &str>((npx, shell_command::effective_path_env()))
    })
    .await
    .map_err(|_| "无法查找 npx")??;
    let command = npx.to_str().ok_or("npx 路径包含无效字符")?;
    let env = path.as_ref().map(|path| json!([{"name": "PATH", "value": path}]));
    let mut server = json!({
        "name": NAME,
        "type": "stdio",
        "command": command,
        "args": ARGS,
        "enabled": true,
        "required": false,
        "timeout": 60
    });
    if let Some(env) = env {
        server["env"] = env;
    }
    if let Some(existing) = config["mcpServers"].as_array().and_then(|servers| {
        servers.iter().find(|server| server["name"].as_str().map(str::trim) == Some(NAME))
    }) && existing != &server
    {
        return Err("MCP 配置中已有同名的 Chrome DevTools 服务器".into());
    }

    let mut download = tokio::process::Command::new(&npx);
    download
        .args(ARGS)
        .arg("--help")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    if let Some(path) = path {
        download.env("PATH", path);
    }
    let status = tokio::time::timeout(Duration::from_secs(120), download.status())
        .await
        .map_err(|_| "Chrome DevTools MCP 下载超时")?
        .map_err(|_| "无法启动 npx，请检查 Node.js 和 npm")?;
    if !status.success() {
        return Err("Chrome DevTools MCP 安装失败，请检查 Node.js LTS 和 npm 网络连接".into());
    }

    access.patch_full_agent_config_async(json!({ "mcpServerToAdd": &server })).await?;
    let saved = access.load_global_config_value_async().await?;
    if !saved["mcpServers"].as_array().is_some_and(|servers| servers.contains(&server)) {
        return Err("网关未保存 Chrome DevTools MCP 配置".into());
    }
    Ok(())
}
