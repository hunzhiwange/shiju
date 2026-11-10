//! Desktop local-app initialization provisions a private, tenant-scoped local grant.
//! No HTTP request can create this grant. Explicit administrator policies take precedence.
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const HOST_ID: &str = "desktop-local";
#[cfg(unix)]
const PRINCIPAL_ID: &str = "desktop-local-user";

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

/// The private policy used by direct local account authorization requests.
pub fn auth_policy_path(home: &Path, platform: &str, company: &str) -> PathBuf {
    policy_path(home, platform, company).with_file_name("auth-access.json")
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
fn create_private_directory(path: &Path) -> std::io::Result<()> {
    use std::os::unix::fs::DirBuilderExt;
    match std::fs::DirBuilder::new().mode(0o700).create(path) {
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
        result => result,
    }
}

/// Provision authorization for the same private principal used by the local application list.
#[cfg(unix)]
pub fn initialize_auth(home: &Path, platform: &str, company: &str) -> anyhow::Result<Credentials> {
    let credentials = initialize(home, platform, company)?;
    let path = auth_policy_path(home, platform, company);
    let directory = path.parent().expect("local auth policy has a parent");
    let credential_root = directory.join("auth-credentials");
    if !credential_root.try_exists()? {
        create_private_directory(&credential_root)?;
    }
    check_directory(&credential_root)?;
    if path.try_exists()? {
        read_private(&path)?;
        return Ok(credentials);
    }

    let apps_policy: serde_json::Value =
        serde_json::from_slice(&read_private(&policy_path(home, platform, company))?)?;
    let apps_host = &apps_policy["hosts"][0];
    anyhow::ensure!(apps_host["id"] == HOST_ID, "本机应用授权缺少执行主机");
    let services = if cfg!(target_os = "macos") {
        serde_json::json!([
            "claude_official",
            "codex_oauth",
            "github_copilot",
            "xai_oauth",
            "kimi_oauth",
            "agy_oauth",
            "dsh_account"
        ])
    } else {
        serde_json::json!(["codex_oauth", "github_copilot", "xai_oauth", "kimi_oauth"])
    };
    // Deletion review reads binding references, but this policy cannot write provider config.
    let grant = serde_json::json!({
        "platform_id": platform, "company_id": company, "host_id": HOST_ID,
        "auth_account_read": services,
        "auth_flow_start": services,
        "auth_account_delete": services,
        "auth_logout": services,
        "provider_config_read": true,
        "auth_default_write": ["codex_oauth", "github_copilot", "xai_oauth", "agy_oauth"],
        "auth_account_revoke": ["dsh_account"],
        "auth_browser_local": true,
    });
    #[cfg(target_os = "macos")]
    let native_login_owners = {
        // AGY and DSH use a single native login per OS user. Claim it once for one tenant.
        let owner = directory.with_file_name("native-auth-owner");
        let scope = serde_json::to_vec(&(platform, company))?;
        publish_private(&owner, &scope)?;
        if read_private(&owner)? == scope {
            serde_json::json!(["agy_oauth", "dsh_account"].map(|service| serde_json::json!({
                    "service": service, "platform_id": platform,
                    "company_id": company, "principal": PRINCIPAL_ID,
            })))
        } else {
            serde_json::json!([])
        }
    };
    #[cfg(not(target_os = "macos"))]
    let native_login_owners = serde_json::json!([]);
    let policy = serde_json::json!({
        "principals": [{
            "id": PRINCIPAL_ID,
            "token_sha256": hex::encode(Sha256::digest(credentials.token.as_bytes())),
            "grants": [grant],
        }],
        "hosts": [{
            "id": HOST_ID, "online": true, "binding": "local", "home": home,
            "search_paths": apps_host["search_paths"], "applications": apps_host["applications"],
            "credential_root": credential_root, "native_login_owners": native_login_owners,
        }],
    });
    publish_private(&path, &serde_json::to_vec_pretty(&policy)?)?;
    Ok(credentials)
}

#[cfg(unix)]
fn publish_private(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    use std::{io::Write, os::unix::fs::PermissionsExt};
    let parent = path.parent().expect("local auth file has a parent");
    let mut file = tempfile::NamedTempFile::new_in(parent)?;
    file.as_file().set_permissions(std::fs::Permissions::from_mode(0o600))?;
    file.write_all(bytes)?;
    file.as_file().sync_all()?;
    match file.persist_noclobber(path) {
        Ok(_) => Ok(()),
        Err(error) if error.error.kind() == std::io::ErrorKind::AlreadyExists => {
            read_private(path)?;
            Ok(())
        }
        Err(error) => Err(error.error.into()),
    }
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
        os::unix::fs::{OpenOptionsExt, PermissionsExt},
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
    create_private_directory(config)?;
    check_config(config)?;
    create_private_directory(root)?;
    check_directory(root)?;
    let directory = policy.parent().ok_or_else(|| anyhow::anyhow!("授权目录缺失"))?;
    // Publish both files together; never overwrite an existing grant or credential.
    let staging = tempfile::Builder::new().prefix(".initialize-").tempdir_in(root)?;
    std::fs::set_permissions(staging.path(), std::fs::Permissions::from_mode(0o700))?;
    let prefix = vw_config_types::paths::home_config_dir(home).join("apps");
    create_private_directory(&prefix)?;
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
        "principals": [{"id": PRINCIPAL_ID, "token_sha256": hex::encode(Sha256::digest(token.as_bytes())),
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
        return restore(home, platform, company)?
            .ok_or_else(|| anyhow::anyhow!("本机授权目录已存在但不完整，请检查后重试"));
    }
    if let Err(error) = std::fs::rename(staging.path(), directory) {
        return restore(home, platform, company)?.ok_or_else(|| error.into());
    }
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
