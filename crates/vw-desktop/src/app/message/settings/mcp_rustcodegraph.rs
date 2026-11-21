//! Install the official RustCodeGraph release on demand for the local gateway.

use config_tools::RUSTCODEGRAPH_MCP_SERVER_ID as NAME;
use desktop_config::ConfigAccess;
use gateway_endpoint::GatewayEndpoint;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::io::Read;
#[cfg(unix)]
use std::io::{Seek, Write};
use std::path::Path;
use std::time::Duration;

const VERSION: &str = "v2.0.0";
const ARGS: [&str; 4] = ["serve", "--mcp", "--path", "${workspaceRoot}"];
const MAX_ARCHIVE_BYTES: u64 = 16 * 1024 * 1024;

fn release_asset() -> Result<(&'static str, &'static str), String> {
    let asset = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => (
            "aarch64-apple-darwin.tar.xz",
            "fbc2a3e4eb60c6f74eeadf7878acb9cb94d1e8148473d8187bca211993ebd6c6",
        ),
        ("macos", "x86_64") => (
            "x86_64-apple-darwin.tar.xz",
            "d40ba43a9d69422531a8d8e63b55da11fd1c48ba48c5044edf7589725be06a25",
        ),
        ("linux", "aarch64") => (
            "aarch64-unknown-linux-gnu.tar.xz",
            "2ea85448bd849a31498a61b7e6a4d98c9a2984f319961b70f1edd09859b789cd",
        ),
        ("linux", "x86_64") => (
            "x86_64-unknown-linux-gnu.tar.xz",
            "d122beac1118ea8151e437571dd8cda27515128cee704ad8bc12210f243fc3bf",
        ),
        ("windows", "aarch64") => (
            "aarch64-pc-windows-msvc.zip",
            "709f1dc348ef61b3d791432813c490965f65e95bcfc6e8b39a1b481ca6a3a88f",
        ),
        ("windows", "x86_64") => (
            "x86_64-pc-windows-msvc.zip",
            "d4cad50a3abaffc48214211421b5086d96ba248a750541d4ed84bd51b7d0f95b",
        ),
        _ => return Err("此平台暂无 RustCodeGraph 发布包".into()),
    };
    Ok(asset)
}

pub(super) async fn install(endpoint: GatewayEndpoint, editor: String) -> Result<(), String> {
    if !endpoint.is_loopback() {
        return Err("RustCodeGraph 一键安装需要本机网关".into());
    }
    let access = ConfigAccess::new(endpoint, None);
    let home = directories::UserDirs::new().ok_or("无法定位用户主目录")?;
    let binary = config_runtime_paths::home_config_dir(home.home_dir())
        .join("bin")
        .join(NAME)
        .join(VERSION)
        .join(format!("{NAME}{}", std::env::consts::EXE_SUFFIX));
    let command = binary.to_str().ok_or("安装路径包含无效字符")?;
    let config = access.load_global_config_value_async().await?;
    let empty = Vec::new();
    let servers = config["mcpServers"].as_array().unwrap_or(&empty);
    let draft =
        serde_json::from_str::<Vec<Value>>(&editor).map_err(|_| "请先修正并保存 MCP 配置草稿")?;
    if &draft != servers {
        return Err("请先保存 MCP 配置草稿或刷新配置".into());
    }
    let existing =
        servers.iter().find(|server| server["name"].as_str().map(str::trim) == Some(NAME));
    let matches_installation = |server: &Value| {
        server["command"] == command
            && server["args"] == json!(ARGS)
            && server["type"].as_str().unwrap_or("stdio") == "stdio"
    };
    if existing.is_some_and(|server| !matches_installation(server)) {
        return Err("MCP 配置中已有同名的 RustCodeGraph 服务器".into());
    }

    let destination = binary.clone();
    tokio::task::spawn_blocking(move || install_binary(&destination))
        .await
        .map_err(|error| error.to_string())??;
    if existing.is_none() {
        access
            .patch_full_agent_config_async(json!({
                "mcpServerToAdd": {
                    "name": NAME,
                    "type": "stdio",
                    "command": command,
                    "args": ARGS,
                    "enabled": true,
                    "required": false
                }
            }))
            .await?;
        let saved = access.load_global_config_value_async().await?;
        if !saved["mcpServers"].as_array().is_some_and(|servers| {
            servers.iter().any(|server| server["name"] == NAME && matches_installation(server))
        }) {
            return Err("网关未保存 RustCodeGraph MCP 配置".into());
        }
    }
    Ok(())
}

fn install_binary(binary: &Path) -> Result<(), String> {
    if binary.is_file() {
        return Ok(());
    }

    let (asset, checksum) = release_asset()?;
    let asset = format!("{NAME}-{asset}");
    let url =
        format!("https://github.com/hunzhiwange/rustcodegraph/releases/download/{VERSION}/{asset}");
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get(url)
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| error.to_string())?;
    let mut archive = Vec::new();
    response
        .take(MAX_ARCHIVE_BYTES + 1)
        .read_to_end(&mut archive)
        .map_err(|error| error.to_string())?;
    if archive.len() as u64 > MAX_ARCHIVE_BYTES || hex::encode(Sha256::digest(&archive)) != checksum
    {
        return Err("RustCodeGraph 发布包校验失败".into());
    }

    let directory = binary.parent().expect("installation path has a parent");
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let mut staged =
        tempfile::NamedTempFile::new_in(directory).map_err(|error| error.to_string())?;
    extract_binary(&asset, &archive, staged.as_file_mut())?;
    staged.as_file().sync_all().map_err(|error| error.to_string())?;
    match staged.persist_noclobber(binary) {
        Ok(_) => Ok(()),
        Err(error)
            if error.error.kind() == std::io::ErrorKind::AlreadyExists && binary.is_file() =>
        {
            Ok(())
        }
        Err(error) => Err(error.error.to_string()),
    }
}

#[cfg(unix)]
fn extract_binary(asset: &str, archive: &[u8], output: &mut std::fs::File) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mut package = tempfile::NamedTempFile::new().map_err(|error| error.to_string())?;
    package.write_all(archive).map_err(|error| error.to_string())?;
    let target = asset.trim_end_matches(".tar.xz");
    let stdout = output.try_clone().map_err(|error| error.to_string())?;
    let status = std::process::Command::new("tar")
        .arg("-xJOf")
        .arg(package.path())
        .arg(format!("{target}/{NAME}"))
        .stdout(stdout)
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|error| format!("无法启动 tar 解压 RustCodeGraph：{error}"))?;
    if !status.success() || output.stream_position().map_err(|error| error.to_string())? == 0 {
        return Err("无法从发布包中解压 RustCodeGraph".into());
    }
    output
        .set_permissions(std::fs::Permissions::from_mode(0o755))
        .map_err(|error| error.to_string())
}

#[cfg(windows)]
fn extract_binary(_asset: &str, archive: &[u8], output: &mut std::fs::File) -> Result<(), String> {
    let mut zip =
        zip::ZipArchive::new(std::io::Cursor::new(archive)).map_err(|error| error.to_string())?;
    let mut executable = zip.by_name("rustcodegraph.exe").map_err(|error| error.to_string())?;
    std::io::copy(&mut executable, output).map_err(|error| error.to_string())?;
    Ok(())
}
