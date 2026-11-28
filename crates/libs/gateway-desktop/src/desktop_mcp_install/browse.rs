//! Versioned OpenCode Browse binaries from Anomaly's stable release service.
use acp_agent_registry::{InstallDownload, InstallLog};
use semver::Version;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    path::{Path, PathBuf},
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::io::AsyncReadExt;

const ORIGIN: &str = "https://anoma.ly";
const USER_AGENT: &str = "browse/stable/shiju";
const MAX_MANIFEST_BYTES: usize = 64 * 1024;
const MAX_ARCHIVE_BYTES: u64 = 16 * 1024 * 1024;
const MAX_BINARY_BYTES: u64 = 32 * 1024 * 1024;
const VERSION_ERROR: &str = "无法读取已安装的 OpenCode Browse 版本";
pub(super) const ARGS: [&str; 1] = ["mcp"];
pub(super) const AUTO_UPDATE_ENV: &str = "OPENCODE_BROWSER_AUTO_UPDATE";

#[derive(Deserialize)]
struct Manifest {
    schema: u64,
    app: String,
    artifact: String,
    channel: String,
    release: PublishedRelease,
}

#[derive(Deserialize)]
struct PublishedRelease {
    version: String,
    targets: BTreeMap<String, Asset>,
    #[serde(default)]
    revoked_versions: Vec<String>,
}

#[derive(Deserialize)]
struct Asset {
    url: String,
    size: u64,
    sha256: String,
}

pub(super) struct Release {
    pub version: Version,
    asset: Asset,
}

fn asset_target() -> Result<String, String> {
    let (os, arch) = match (std::env::consts::OS, std::env::consts::ARCH) {
        (os @ ("macos" | "linux"), "aarch64") => (os, "arm64"),
        (os @ ("macos" | "linux" | "windows"), "x86_64") => (os, "x64"),
        // Anomaly uses the x64 build through Windows on ARM emulation.
        ("windows", "aarch64") => ("windows", "x64"),
        _ => return Err("此平台暂无 OpenCode Browse 发布包".into()),
    };
    #[cfg(target_arch = "x86_64")]
    let avx2 = std::is_x86_feature_detected!("avx2");
    #[cfg(not(target_arch = "x86_64"))]
    let avx2 = false;
    let baseline = if arch == "x64" && !avx2 { "-baseline" } else { "" };
    let libc = if os == "linux"
        && (cfg!(target_env = "musl") || Path::new("/etc/alpine-release").is_file())
    {
        "-musl"
    } else {
        ""
    };
    Ok(format!("{os}-{arch}{baseline}{libc}"))
}

fn parse_release(bytes: &[u8], target: &str) -> Result<Release, String> {
    let invalid = "OpenCode Browse 发布清单无效";
    let manifest: Manifest = serde_json::from_slice(bytes).map_err(|_| invalid)?;
    if manifest.schema != 1
        || manifest.app != "browse"
        || manifest.artifact != "cli"
        || manifest.channel != "stable"
    {
        return Err(invalid.into());
    }
    let PublishedRelease { version, mut targets, revoked_versions } = manifest.release;
    let version = Version::parse(&version).map_err(|_| invalid)?;
    if !version.pre.is_empty() || revoked_versions.contains(&version.to_string()) {
        return Err(invalid.into());
    }
    let asset = targets.remove(target).ok_or("此平台暂无 OpenCode Browse 发布包")?;
    let (digest, filename) = asset
        .url
        .strip_prefix("/release/browse/files/")
        .and_then(|path| path.split_once('/'))
        .ok_or(invalid)?;
    if asset.size == 0
        || asset.size > MAX_ARCHIVE_BYTES
        || digest != asset.sha256
        || digest.len() != 64
        || !digest.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
        || !filename.ends_with(".tgz")
        || !filename
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(invalid.into());
    }
    Ok(Release { version, asset })
}

pub(super) async fn latest_release() -> Result<Release, String> {
    let target = asset_target()?;
    let error = "无法查询 OpenCode Browse 最新稳定版";
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| error)?;
    let mut response = client
        .get(format!("{ORIGIN}/release/browse/cli/stable.json"))
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .await
        .map_err(|_| error)?;
    if !response.status().is_success() {
        return Err(error.into());
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| error)? {
        if bytes.len() + chunk.len() > MAX_MANIFEST_BYTES {
            return Err("OpenCode Browse 发布清单无效".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    parse_release(&bytes, &target)
}

fn binary_name() -> String {
    format!("browse{}", std::env::consts::EXE_SUFFIX)
}

pub(super) fn install_root() -> Result<PathBuf, String> {
    let home = directories::UserDirs::new().ok_or("无法定位用户主目录")?;
    Ok(config_runtime_paths::home_config_dir(home.home_dir()).join("bin").join("browse"))
}

pub(super) fn binary_at(root: &Path, version: &Version) -> PathBuf {
    root.join(format!("v{version}")).join(binary_name())
}

pub(super) fn managed_version(root: &Path, binary: &Path) -> Option<Version> {
    let tag = binary.parent()?.file_name()?.to_str()?.strip_prefix('v')?;
    let version = Version::parse(tag).ok()?;
    (binary == binary_at(root, &version).as_path()).then_some(version)
}

// Only the upstream installer's conventional locations may be probed.
pub(super) fn is_official_path(binary: &Path) -> bool {
    let name = binary_name();
    let browse_home = std::env::var_os("OPENCODE_BROWSER_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join("bin").join(&name));
    let xdg_bin = std::env::var_os("XDG_BIN_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .map(|path| path.join(&name));
    let user_bin = directories::UserDirs::new()
        .map(|home| home.home_dir().join(".local").join("bin").join(&name));
    [browse_home, xdg_bin, user_bin].into_iter().flatten().any(|path| binary == path.as_path())
}

pub(super) async fn installed_version(binary: &Path) -> Result<Option<Version>, String> {
    match binary.symlink_metadata() {
        Ok(metadata) if metadata.is_file() => {}
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        _ => return Err(VERSION_ERROR.into()),
    }
    let mut process = tokio::process::Command::new(binary)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| VERSION_ERROR)?;
    let stdout = process.stdout.take().ok_or(VERSION_ERROR)?;
    let mut bytes = Vec::new();
    let exit = tokio::time::timeout(Duration::from_secs(3), async {
        stdout.take(1025).read_to_end(&mut bytes).await?;
        if bytes.len() > 1024 {
            return Err(io::Error::other("version output exceeded limit"));
        }
        process.wait().await
    })
    .await
    .map_err(|_| VERSION_ERROR)?
    .map_err(|_| VERSION_ERROR)?;
    if !exit.success() {
        return Err(VERSION_ERROR.into());
    }
    let output = std::str::from_utf8(&bytes).map_err(|_| VERSION_ERROR)?.trim();
    let version = output.strip_prefix("browse ").unwrap_or(output);
    Version::parse(version.trim_start_matches('v')).map(Some).map_err(|_| VERSION_ERROR.into())
}

pub(super) fn set_managed_environment(configuration: &mut serde_json::Value) -> Result<(), String> {
    if configuration.get("env").is_none() {
        configuration["env"] = serde_json::json!([]);
    }
    let env = configuration["env"].as_array_mut().ok_or("OpenCode Browse 的环境变量配置无效")?;
    // The gateway manages the versioned binary. Other Browse launchers retain their update settings.
    env.retain(|entry| entry["name"].as_str().map(str::trim) != Some(AUTO_UPDATE_ENV));
    env.push(serde_json::json!({"name": AUTO_UPDATE_ENV, "value": "off"}));
    Ok(())
}

pub(super) fn manual_command(binary: &Path, release: &Release) -> Result<String, String> {
    let target = super::executable(binary)?;
    let Asset { url, sha256, size } = &release.asset;
    let url = format!("{ORIGIN}{url}");
    let name = binary_name();
    if cfg!(windows) {
        let target = target.replace('\'', "''");
        return Ok(format!(
            r#"$ErrorActionPreference = 'Stop'
$target = '{target}'
$temp = Join-Path ([System.IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $temp | Out-Null
try {{
    $archive = Join-Path $temp 'browse.tgz'
    Invoke-WebRequest -UseBasicParsing -Uri '{url}' -OutFile $archive
    if ((Get-Item -LiteralPath $archive).Length -ne {size} -or (Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash -ne '{sha256}') {{ throw 'Browse archive verification failed' }}
    tar.exe -xzf $archive -C $temp 'package/bin/{name}'
    if ($LASTEXITCODE -ne 0) {{ throw 'Could not extract Browse' }}
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target) | Out-Null
    Copy-Item -LiteralPath (Join-Path $temp 'package/bin/{name}') -Destination $target -Force
}} finally {{
    Remove-Item -LiteralPath $temp -Recurse -Force
}}"#
        ));
    }
    let target = target.replace('\'', "'\"'\"'");
    let hash = if cfg!(target_os = "macos") { "shasum -a 256" } else { "sha256sum" };
    Ok(format!(
        r#"(
set -eu
target='{target}'
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT
archive="$temp/browse.tgz"
curl -fL --proto '=https' --proto-redir '=https' --max-time 120 '{url}' -o "$archive"
test "$(wc -c < "$archive" | tr -d ' ')" -eq {size}
actual=$({hash} "$archive")
set -- $actual
test "$1" = '{sha256}'
tar -xzOf "$archive" 'package/bin/{name}' > "$temp/{name}"
test -s "$temp/{name}"
mkdir -p "$(dirname "$target")"
install -m 755 "$temp/{name}" "$target"
)"#
    ))
}

pub(super) fn install_binary(
    binary: &Path,
    release: &Release,
    log: &mut InstallLog,
) -> Result<(), String> {
    if binary.symlink_metadata().is_ok_and(|metadata| metadata.is_file()) {
        log.step("复用已安装的目标版本");
        return Ok(());
    }

    log.step("下载 OpenCode Browse 发布包");
    let failed = "无法下载 OpenCode Browse 发布包";
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .redirect(reqwest::redirect::Policy::none())
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|_| failed)?;
    let asset = &release.asset;
    let response = client
        .get(format!("{ORIGIN}{}", asset.url))
        .header(reqwest::header::USER_AGENT, USER_AGENT)
        .send()
        .map_err(|_| failed)?;
    if !response.status().is_success() {
        return Err(failed.into());
    }
    let verify_error = "OpenCode Browse 发布包校验失败";
    if response.content_length().is_some_and(|length| length != asset.size) {
        return Err(verify_error.into());
    }
    let mut response = response.take(asset.size + 1);
    let mut bytes = Vec::new();
    let mut chunk = [0; 64 * 1024];
    let started = Instant::now();
    loop {
        let count = response.read(&mut chunk).map_err(|_| failed)?;
        if count == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..count]);
        let downloaded = bytes.len() as u64;
        log.progress.lock().unwrap().download = Some(InstallDownload {
            downloaded,
            total: Some(asset.size),
            bytes_per_second: (downloaded as f64 / started.elapsed().as_secs_f64().max(0.001))
                as u64,
        });
    }
    log.step("校验 OpenCode Browse 发布包");
    if bytes.len() as u64 != asset.size || hex::encode(Sha256::digest(&bytes)) != asset.sha256 {
        return Err(verify_error.into());
    }

    log.step("解压并安装 OpenCode Browse");
    let directory = binary.parent().ok_or("安装路径包含无效字符")?;
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let mut staged =
        tempfile::NamedTempFile::new_in(directory).map_err(|error| error.to_string())?;
    extract_binary(&bytes, staged.as_file_mut())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        staged
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o755))
            .map_err(|error| error.to_string())?;
    }
    staged.as_file().sync_all().map_err(|error| error.to_string())?;
    match staged.persist_noclobber(binary) {
        Ok(_) => Ok(()),
        Err(error)
            if error.error.kind() == io::ErrorKind::AlreadyExists
                && binary.symlink_metadata().is_ok_and(|metadata| metadata.is_file()) =>
        {
            Ok(())
        }
        Err(error) => Err(error.error.to_string()),
    }
}

fn extract_binary(bytes: &[u8], output: &mut impl Write) -> Result<(), String> {
    let wanted = PathBuf::from("package").join("bin").join(binary_name());
    let decoder = flate2::read::GzDecoder::new(bytes);
    let mut archive = tar::Archive::new(decoder.take(MAX_BINARY_BYTES * 2));
    let error = "OpenCode Browse 发布包中的可执行文件无效";
    for entry in archive.entries().map_err(|_| error)? {
        let mut entry = entry.map_err(|_| error)?;
        if entry.path().map_err(|_| error)?.as_ref() != wanted.as_path() {
            continue;
        }
        let size = entry.size();
        if !entry.header().entry_type().is_file() || size == 0 || size > MAX_BINARY_BYTES {
            return Err(error.into());
        }
        return match io::copy(&mut entry, output) {
            Ok(written) if written == size => Ok(()),
            _ => Err(error.into()),
        };
    }
    Err(error.into())
}

#[cfg(test)]
#[path = "browse_tests.rs"]
mod tests;
