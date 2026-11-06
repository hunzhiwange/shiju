//! Desktop local-app initialization provisions a private, tenant-scoped local grant.
//! No HTTP request can create this grant. Explicit administrator policies take precedence.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const HOST_ID: &str = "desktop-local";

/// Credentials intentionally have no Debug implementation.
#[derive(Clone)]
pub struct Credentials {
    pub token: String,
}

pub fn policy_path(home: &Path, platform: &str, company: &str) -> PathBuf {
    let scope = serde_json::to_vec(&(platform, company)).expect("string tuple serializes");
    vw_config_types::paths::home_config_dir(home)
        .join("local-apps")
        .join(hex::encode(Sha256::digest(scope)))
        .join("access.json")
}

#[cfg(unix)]
fn check_directory(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(path)?;
    anyhow::ensure!(metadata.is_dir() && !metadata.is_symlink(), "授权目录不可为符号链接");
    anyhow::ensure!(
        metadata.uid() == unsafe { libc::geteuid() } && metadata.mode() & 0o077 == 0,
        "授权目录权限不安全，请恢复为仅当前用户可访问"
    );
    Ok(())
}

#[cfg(unix)]
fn check_config(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(path)?;
    anyhow::ensure!(
        metadata.is_dir()
            && !metadata.is_symlink()
            && metadata.uid() == unsafe { libc::geteuid() }
            && metadata.mode() & 0o022 == 0,
        "配置目录权限不安全或为符号链接"
    );
    Ok(())
}

#[cfg(unix)]
fn read_private(path: &Path) -> anyhow::Result<Vec<u8>> {
    use std::{
        io::Read,
        os::unix::fs::{MetadataExt, OpenOptionsExt},
    };
    check_directory(path.parent().ok_or_else(|| anyhow::anyhow!("授权目录缺失"))?)?;
    let file = std::fs::OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC)
        .open(path)?;
    let metadata = file.metadata()?;
    anyhow::ensure!(
        metadata.is_file()
            && metadata.uid() == unsafe { libc::geteuid() }
            && metadata.mode() & 0o077 == 0
            && metadata.len() <= 65536,
        "授权文件权限或大小不安全"
    );
    let mut bytes = Vec::new();
    file.take(65537).read_to_end(&mut bytes)?;
    anyhow::ensure!(bytes.len() <= 65536, "授权文件过大");
    Ok(bytes)
}

/// Restore only an existing grant; never initializes or expands permissions.
#[cfg(unix)]
pub fn restore(home: &Path, platform: &str, company: &str) -> anyhow::Result<Option<Credentials>> {
    let policy = policy_path(home, platform, company);
    if !policy.try_exists()? {
        return Ok(None);
    }
    let directory = policy.parent().ok_or_else(|| anyhow::anyhow!("授权目录缺失"))?;
    check_directory(directory.parent().ok_or_else(|| anyhow::anyhow!("授权目录缺失"))?)?;
    check_config(
        directory.parent().and_then(Path::parent).ok_or_else(|| anyhow::anyhow!("配置目录缺失"))?,
    )?;
    let bytes = read_private(&policy.with_file_name("credential"))?;
    let token = String::from_utf8(bytes)?;
    anyhow::ensure!(
        token.len() == 64 && token.bytes().all(|b| b.is_ascii_hexdigit()),
        "本机授权凭证无效"
    );
    let document: serde_json::Value = serde_json::from_slice(&read_private(&policy)?)?;
    anyhow::ensure!(
        document["principals"][0]["token_sha256"] == hex::encode(Sha256::digest(token.as_bytes())),
        "本机授权凭证不匹配"
    );
    Ok(Some(Credentials { token }))
}

/// Initialize the desktop local-app default without requiring deployment configuration.
#[cfg(unix)]
pub fn initialize(home: &Path, platform: &str, company: &str) -> anyhow::Result<Credentials> {
    use std::{
        io::Write,
        os::unix::fs::{DirBuilderExt, OpenOptionsExt, PermissionsExt},
    };
    anyhow::ensure!(home.is_absolute() && home.is_dir(), "无法确定本机用户目录");
    anyhow::ensure!(!platform.trim().is_empty() && !company.trim().is_empty(), "缺少当前账号范围");
    if let Some(credentials) = restore(home, platform, company)? {
        return Ok(credentials);
    }
    let policy = policy_path(home, platform, company);
    let root =
        policy.parent().and_then(Path::parent).ok_or_else(|| anyhow::anyhow!("授权目录缺失"))?;
    let config = root.parent().ok_or_else(|| anyhow::anyhow!("配置目录缺失"))?;
    if !config.exists() {
        std::fs::DirBuilder::new().mode(0o700).create(config)?;
    }
    check_config(config)?;
    if !root.exists() {
        std::fs::DirBuilder::new().mode(0o700).create(root)?;
    }
    check_directory(root)?;
    let directory = policy.parent().ok_or_else(|| anyhow::anyhow!("授权目录缺失"))?;
    // Publish both files together; never overwrite an existing grant or credential.
    let staging = tempfile::Builder::new().prefix(".initialize-").tempdir_in(root)?;
    std::fs::set_permissions(staging.path(), std::fs::Permissions::from_mode(0o700))?;
    let prefix = vw_config_types::paths::home_config_dir(home).join("apps");
    if !prefix.exists() {
        std::fs::DirBuilder::new().mode(0o700).create(&prefix)?;
    }
    check_directory(&prefix)?;
    let prefix = prefix.canonicalize()?;
    let paths = search_paths(home, &prefix);
    let npm = paths
        .iter()
        .map(|p| p.join("npm"))
        .find(|p| p.is_file() && p.with_file_name("node").is_file());
    let mut entropy = [0u8; 32];
    getrandom::fill(&mut entropy).map_err(|_| anyhow::anyhow!("无法生成安全的本机授权"))?;
    let token = hex::encode(entropy);
    let document = serde_json::json!({
        "principals": [{"id": "desktop-local-user", "token_sha256": hex::encode(Sha256::digest(token.as_bytes())),
            "grants": [{"platform_id": platform, "company_id": company, "host_id": HOST_ID,
                "actions": ["install", "update", "uninstall"], "preferences_write": true, "launch": true}]}],
        "hosts": [{"id": HOST_ID, "online": true, "binding": "local", "home": home,
            "search_paths": paths, "applications": if cfg!(target_os = "macos") { PathBuf::from("/Applications") } else { home.join(".local/share/applications") },
            "lifecycle": npm.map(|npm| serde_json::json!({"prefix": prefix, "npm": npm}))}]
    });
    for (name, bytes) in [
        ("credential", token.as_bytes().to_vec()),
        ("access.json", serde_json::to_vec_pretty(&document)?),
    ] {
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(staging.path().join(name))?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    if directory.exists() {
        anyhow::bail!("本机授权目录已存在但不完整，请检查后重试");
    }
    std::fs::rename(staging.path(), directory)?;
    Ok(Credentials { token })
}

#[cfg(unix)]
fn search_paths(home: &Path, prefix: &Path) -> Vec<PathBuf> {
    let mut paths = vec![prefix.join("bin")];
    for relative in [
        ".local/bin",
        ".npm-global/bin",
        ".volta/bin",
        ".bun/bin",
        ".cargo/bin",
        "bin",
        ".opencode/bin",
        ".kimi-code/bin",
        ".grok/bin",
        ".minimax-code/bin",
        ".local/share/mise/shims",
        "Library/pnpm",
        ".local/share/pnpm",
    ] {
        let path = home.join(relative);
        paths.push(path);
    }
    for relative in [".nvm/versions/node", ".local/share/mise/installs/node"] {
        let mut entries: Vec<_> = std::fs::read_dir(home.join(relative))
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path().join("bin"))
            .filter(|p| p.is_dir())
            .collect();
        entries.sort();
        paths.extend(entries);
    }
    for path in [
        "/opt/homebrew/bin",
        "/home/linuxbrew/.linuxbrew/bin",
        "/usr/local/bin",
        "/usr/bin",
        "/bin",
    ] {
        if Path::new(path).is_dir() {
            paths.push(path.into());
        }
    }
    if let Some(value) = std::env::var_os("PATH") {
        for path in std::env::split_paths(&value) {
            if path.is_absolute() && path.is_dir() && !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let path = PathBuf::from(
            "/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS",
        );
        if path.is_dir() && !paths.contains(&path) {
            paths.push(path);
        }
    }
    paths.truncate(64);
    paths
}

#[cfg(not(unix))]
pub fn restore(_: &Path, _: &str, _: &str) -> anyhow::Result<Option<Credentials>> {
    anyhow::bail!("此系统暂不支持本机应用授权")
}
#[cfg(not(unix))]
pub fn initialize(_: &Path, _: &str, _: &str) -> anyhow::Result<Credentials> {
    anyhow::bail!("此系统暂不支持本机应用授权")
}

#[cfg(all(test, unix))]
#[path = "local_apps_tests.rs"]
mod tests;
