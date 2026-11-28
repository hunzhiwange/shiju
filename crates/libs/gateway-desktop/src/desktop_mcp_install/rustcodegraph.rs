//! Versioned, checksum-verified RustCodeGraph installation on the gateway host.

use acp_agent_registry::{InstallDownload, InstallLog};
use config_tools::RUSTCODEGRAPH_MCP_SERVER_ID as NAME;
use sha2::{Digest, Sha256};
use std::io::Read;
#[cfg(unix)]
use std::io::{Seek, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(super) const ARGS: [&str; 4] = ["serve", "--mcp", "--path", "${workspaceRoot}"];
const MAX_ARCHIVE_BYTES: u64 = 16 * 1024 * 1024;

const REPOSITORY: &str = "https://github.com/hunzhiwange/rustcodegraph";
const TAG_PATH: &str = "/hunzhiwange/rustcodegraph/releases/tag/";

pub(super) struct Release {
    pub tag: String,
    pub version: semver::Version,
    pub(super) asset: String,
    pub(super) checksum: String,
}

impl Release {
    fn download_url(&self) -> String {
        format!("{REPOSITORY}/releases/download/{}/{}", self.tag, self.asset)
    }
}

fn release_asset() -> Result<String, String> {
    let suffix = match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => "aarch64-apple-darwin.tar.xz",
        ("macos", "x86_64") => "x86_64-apple-darwin.tar.xz",
        ("linux", "aarch64") => "aarch64-unknown-linux-gnu.tar.xz",
        ("linux", "x86_64") => "x86_64-unknown-linux-gnu.tar.xz",
        ("windows", "aarch64") => "aarch64-pc-windows-msvc.zip",
        ("windows", "x86_64") => "x86_64-pc-windows-msvc.zip",
        _ => return Err("此平台暂无 RustCodeGraph 发布包".into()),
    };
    Ok(format!("{NAME}-{suffix}"))
}

fn release_redirect() -> reqwest::redirect::Policy {
    reqwest::redirect::Policy::custom(|attempt| {
        if attempt.previous().len() < 5
            && attempt.url().scheme() == "https"
            && matches!(
                attempt.url().host_str(),
                Some(
                    "github.com"
                        | "release-assets.githubusercontent.com"
                        | "objects.githubusercontent.com"
                )
            )
        {
            attempt.follow()
        } else {
            attempt.error("unexpected RustCodeGraph release redirect")
        }
    })
}

pub(super) fn stable_release(url: &reqwest::Url) -> Option<(&str, semver::Version)> {
    if url.scheme() != "https" || url.host_str() != Some("github.com") {
        return None;
    }
    let tag = url.path().strip_prefix(TAG_PATH)?;
    let version = semver::Version::parse(tag.strip_prefix('v')?).ok()?;
    version.pre.is_empty().then_some((tag, version))
}

pub(super) fn package_checksum(content: &[u8], asset: &str) -> Option<String> {
    let mut fields = std::str::from_utf8(content).ok()?.split_whitespace();
    let hash = fields.next()?;
    let filename = fields.next()?.trim_start_matches('*');
    (hash.len() == 64
        && hash.bytes().all(|byte| byte.is_ascii_hexdigit())
        && filename == asset
        && fields.next().is_none())
    .then(|| hash.to_ascii_lowercase())
}

pub(super) async fn latest_release() -> Result<Release, String> {
    let asset = release_asset()?;
    let client = reqwest::Client::builder()
        .https_only(true)
        .redirect(release_redirect())
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|_| "无法创建 RustCodeGraph 发布查询客户端")?;
    let latest = client
        .head(format!("{REPOSITORY}/releases/latest"))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| "无法查询 RustCodeGraph 最新稳定版")?;
    let (tag, version) = stable_release(latest.url()).ok_or("无法识别 RustCodeGraph 稳定版标签")?;
    let mut response = client
        .get(format!("{REPOSITORY}/releases/download/{tag}/{asset}.sha256"))
        .send()
        .await
        .and_then(reqwest::Response::error_for_status)
        .map_err(|_| "无法下载当前平台的 RustCodeGraph SHA-256 校验文件")?;
    let mut bytes = Vec::new();
    while let Some(chunk) =
        response.chunk().await.map_err(|_| "无法读取 RustCodeGraph SHA-256 校验文件")?
    {
        if bytes.len() + chunk.len() > 1024 {
            return Err("RustCodeGraph SHA-256 校验文件格式无效".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let checksum =
        package_checksum(&bytes, &asset).ok_or("RustCodeGraph SHA-256 校验文件格式无效")?;
    Ok(Release { tag: tag.into(), version, asset, checksum })
}

pub(super) fn install_root() -> Result<PathBuf, String> {
    let home = directories::UserDirs::new().ok_or("无法定位用户主目录")?;
    Ok(config_runtime_paths::home_config_dir(home.home_dir()).join("bin").join(NAME))
}

pub(super) fn binary_at(root: &Path, version: &str) -> PathBuf {
    root.join(version).join(format!("{NAME}{}", std::env::consts::EXE_SUFFIX))
}

pub(super) fn manual_command(binary: &Path, release: &Release) -> Result<String, String> {
    let Release { asset, checksum, .. } = release;
    let url = release.download_url();
    let target = binary.to_str().ok_or("安装路径包含无效字符")?;
    if cfg!(windows) {
        let target = target.replace('\'', "''");
        return Ok(format!(
            r#"$ErrorActionPreference = 'Stop'
$target = '{target}'
$temp = Join-Path ([System.IO.Path]::GetTempPath()) ([guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path $temp | Out-Null
try {{
    $archive = Join-Path $temp '{asset}'
    Invoke-WebRequest -UseBasicParsing -Uri '{url}' -OutFile $archive
    if ((Get-FileHash -Algorithm SHA256 -LiteralPath $archive).Hash -ne '{checksum}') {{ throw 'SHA-256 mismatch' }}
    Expand-Archive -LiteralPath $archive -DestinationPath $temp
    New-Item -ItemType Directory -Force -Path (Split-Path -Parent $target) | Out-Null
    Copy-Item -LiteralPath (Join-Path $temp 'rustcodegraph.exe') -Destination $target -Force
}} finally {{
    Remove-Item -LiteralPath $temp -Recurse -Force
}}"#
        ));
    }
    let target = target.replace('\'', "'\"'\"'");
    let member = asset.trim_end_matches(".tar.xz");
    let hash = if cfg!(target_os = "macos") { "shasum -a 256" } else { "sha256sum" };
    Ok(format!(
        r#"(
set -eu
target='{target}'
temp=$(mktemp -d)
trap 'rm -rf "$temp"' EXIT
archive="$temp/{asset}"
curl -fL --proto '=https' --proto-redir '=https' --max-time 120 '{url}' -o "$archive"
actual=$({hash} "$archive")
test "${{actual%% *}}" = '{checksum}' || {{ echo 'SHA-256 mismatch' >&2; exit 1; }}
tar -xJOf "$archive" '{member}/{NAME}' > "$temp/{NAME}"
test -s "$temp/{NAME}"
mkdir -p "$(dirname "$target")"
install -m 755 "$temp/{NAME}" "$target"
)"#
    ))
}

pub(super) fn install_binary(
    binary: &Path,
    release: &Release,
    log: &mut InstallLog,
) -> Result<(), String> {
    if binary.is_file() {
        log.step("复用已安装的目标版本");
        return Ok(());
    }

    log.step("下载 RustCodeGraph 发布包");
    let client = reqwest::blocking::Client::builder()
        .https_only(true)
        .redirect(release_redirect())
        .timeout(Duration::from_secs(120))
        .build()
        .map_err(|error| error.to_string())?;
    let response = client
        .get(release.download_url())
        .send()
        .and_then(reqwest::blocking::Response::error_for_status)
        .map_err(|error| error.without_url().to_string())?;
    let total = response.content_length();
    if total.is_some_and(|size| size > MAX_ARCHIVE_BYTES) {
        return Err("RustCodeGraph 发布包过大".into());
    }
    let mut response = response.take(MAX_ARCHIVE_BYTES + 1);
    let mut archive = Vec::new();
    let mut chunk = [0; 64 * 1024];
    let started = Instant::now();
    loop {
        let count = response.read(&mut chunk).map_err(|error| error.to_string())?;
        if count == 0 {
            break;
        }
        archive.extend_from_slice(&chunk[..count]);
        let downloaded = archive.len() as u64;
        log.progress.lock().unwrap().download = Some(InstallDownload {
            downloaded,
            total,
            bytes_per_second: (downloaded as f64 / started.elapsed().as_secs_f64().max(0.001))
                as u64,
        });
    }
    log.step("校验 RustCodeGraph 发布包");
    if archive.len() as u64 > MAX_ARCHIVE_BYTES
        || hex::encode(Sha256::digest(&archive)) != release.checksum
    {
        return Err("RustCodeGraph 发布包校验失败".into());
    }

    log.step("解压并安装 RustCodeGraph");
    let directory = binary.parent().expect("installation path has a parent");
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let mut staged =
        tempfile::NamedTempFile::new_in(directory).map_err(|error| error.to_string())?;
    extract_binary(&release.asset, &archive, staged.as_file_mut())?;
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
