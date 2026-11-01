//! Host-owned encrypted public MCP grants. Never stored in plugin packages.
use super::mcp_elicitation::FormOwner;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};

#[derive(Serialize, Deserialize)]
pub(super) struct Grant {
    pub issuer: String,
    pub client: String,
    pub resource: String,
    pub endpoint: String,
    pub access: String,
    pub refresh: Option<String>,
    pub expires: u64,
}
fn now() -> Result<u64> {
    Ok(SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs())
}
impl Grant {
    pub(super) fn response(
        value: &serde_json::Value,
        issuer: String,
        client: String,
        resource: String,
        endpoint: String,
        previous_refresh: Option<String>,
    ) -> Result<Self> {
        let token = |name: &str| -> Result<String> {
            let token = value[name]
                .as_str()
                .filter(|s| !s.is_empty() && s.bytes().all(|b| b > 32 && b < 127))
                .context("invalid MCP token response")?;
            Ok(token.to_owned())
        };
        if !value["token_type"].as_str().is_some_and(|s| s.eq_ignore_ascii_case("bearer")) {
            bail!("unsupported MCP token type");
        }
        let expires = now()?
            .checked_add(
                value["expires_in"]
                    .as_u64()
                    .filter(|n| *n > 0)
                    .context("MCP token requires positive expires_in")?,
            )
            .context("MCP token expiry overflow")?;
        let refresh = if value.get("refresh_token").is_some() {
            Some(token("refresh_token")?)
        } else {
            previous_refresh
        };
        Ok(Self {
            issuer,
            client,
            resource,
            endpoint,
            access: token("access_token")?,
            refresh,
            expires,
        })
    }
}
fn root() -> Result<PathBuf> {
    let (config, _) = crate::config::schema::default_config_and_workspace_dirs()?;
    Ok(config.join("plugin-mcp-oauth"))
}
fn key(server: &agent_client_protocol::McpServerHttp, owner: &FormOwner) -> Result<String> {
    if owner.session.trim().is_empty() {
        bail!("MCP OAuth requires a session owner");
    }
    Ok(hex::encode(Sha256::digest(serde_json::to_vec(&(server, &owner.session, &owner.tenant))?)))
}
fn locks() -> &'static Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>> {
    static LOCKS: OnceLock<Mutex<HashMap<String, Arc<tokio::sync::Mutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(Mutex::default)
}
pub(super) struct GrantLock {
    _memory: tokio::sync::OwnedMutexGuard<()>,
    _file: std::fs::File,
}
pub(super) async fn lock(
    server: &agent_client_protocol::McpServerHttp,
    owner: &FormOwner,
) -> Result<GrantLock> {
    let binding = key(server, owner)?;
    let lock = locks()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .entry(binding)
        .or_default()
        .clone();
    let memory = lock.lock_owned().await;
    let root = root()?;
    ensure_directory(&root)?;
    let mut options = std::fs::OpenOptions::new();
    options.read(true).write(true).create(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600).custom_flags(libc::O_NOFOLLOW);
    }
    let file = options.open(root.join(format!("{}.lock", key(server, owner)?)))?;
    file.try_lock().map_err(|_| anyhow::anyhow!("MCP OAuth grant is busy in another process"))?;
    Ok(GrantLock { _memory: memory, _file: file })
}
fn ensure_directory(root: &Path) -> Result<()> {
    let mut builder = std::fs::DirBuilder::new();
    builder.recursive(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(root)?;
    let metadata = std::fs::symlink_metadata(root)?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        bail!("MCP credential storage must be a real host directory");
    }
    Ok(())
}
fn path(root: &Path, key: &str) -> PathBuf {
    root.join(format!("{key}.grant"))
}
pub(super) fn write(root: &Path, key: &str, grant: &Grant) -> Result<()> {
    ensure_directory(root)?;
    let encrypted = crate::security::SecretStore::new(root, true)
        .encrypt(&serde_json::to_string(&(key, grant))?)?;
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut random = [0u8; 16];
    getrandom::fill(&mut random).context("MCP credential entropy unavailable")?;
    let temporary = root.join(format!("{}.tmp", hex::encode(random)));
    let result = (|| -> Result<()> {
        use std::io::Write;
        let mut file = options.open(&temporary)?;
        file.write_all(encrypted.as_bytes())?;
        file.sync_all()?;
        // Windows rename cannot replace an existing file. Readers hold the grant lock;
        // a crash between deletion and rename leaves the owner logged out.
        #[cfg(windows)]
        remove(root, key)?;
        std::fs::rename(&temporary, path(root, key))?;
        Ok(())
    })();
    if temporary.exists() {
        let _ = std::fs::remove_file(temporary);
    }
    result
}
pub(super) fn read(root: &Path, key: &str) -> Result<Option<Grant>> {
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW);
    }
    let file = match options.open(path(root, key)) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e.into()),
    };
    if !file.metadata()?.is_file() {
        bail!("invalid MCP grant storage file");
    }
    use std::io::Read;
    let mut encrypted = String::new();
    file.take(1024 * 1024 + 1).read_to_string(&mut encrypted)?;
    if encrypted.len() > 1024 * 1024 {
        bail!("MCP stored grant exceeds size limit");
    }
    if !crate::security::SecretStore::is_secure_encrypted(&encrypted) {
        bail!("MCP credentials are not securely encrypted");
    }
    let plaintext = crate::security::SecretStore::new(root, true).decrypt(&encrypted)?;
    let (binding, grant): (String, Grant) =
        serde_json::from_str(&plaintext).context("invalid stored MCP grant")?;
    if binding != key {
        bail!("MCP credential owner binding mismatch");
    }
    Ok(Some(grant))
}
pub(super) fn remove(root: &Path, key: &str) -> Result<()> {
    match std::fs::remove_file(path(root, key)) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}
pub(super) fn save(
    server: &agent_client_protocol::McpServerHttp,
    owner: &FormOwner,
    grant: &Grant,
) -> Result<()> {
    super::mcp_auth::source_config(server)?.context("MCP OAuth requires plugin provenance")?;
    write(&root()?, &key(server, owner)?, grant)
}

pub(crate) async fn access(
    server: &agent_client_protocol::McpServerHttp,
    owner: &FormOwner,
) -> Result<Option<String>> {
    if super::mcp_auth::source_config(server)?.is_none() {
        return Ok(None);
    }
    let _guard = lock(server, owner).await?;
    let root = root()?;
    let key = key(server, owner)?;
    access_with(&root, &key, |grant| async move {
        let metadata = super::mcp_auth::discover_oauth(server).await?;
        if metadata.issuer != grant.issuer
            || metadata.resource != grant.resource
            || metadata.token_endpoint != grant.endpoint
            || metadata.client_id.as_ref().is_some_and(|id| id != &grant.client)
        {
            bail!("MCP OAuth grant identity changed");
        }
        let refresh =
            grant.refresh.as_deref().context("MCP authorization expired; login required")?;
        let body = url::form_urlencoded::Serializer::new(String::new())
            .extend_pairs([
                ("grant_type", "refresh_token"),
                ("refresh_token", refresh),
                ("client_id", &grant.client),
                ("resource", &grant.resource),
            ])
            .finish();
        let endpoint = super::mcp_oauth_discovery::endpoint(&grant.endpoint)?;
        let client = super::mcp_oauth_discovery::public_client(&endpoint).await?;
        let value = super::mcp_oauth_flow::send_provider_request(
            client,
            endpoint,
            body,
            "application/x-www-form-urlencoded",
        )
        .await?;
        Grant::response(
            &value,
            grant.issuer,
            grant.client,
            grant.resource,
            grant.endpoint,
            grant.refresh,
        )
    })
    .await
}
pub(super) async fn access_with<F, Fut>(
    root: &Path,
    key: &str,
    refresh: F,
) -> Result<Option<String>>
where
    F: FnOnce(Grant) -> Fut,
    Fut: std::future::Future<Output = Result<Grant>>,
{
    let Some(grant) = read(root, key)? else { return Ok(None) };
    if grant.expires > now()?.saturating_add(30) {
        return Ok(Some(grant.access));
    }
    // Delete before network I/O: errors/cancellation must never revive an expired grant.
    remove(root, key)?;
    let grant = refresh(grant).await?;
    write(root, key, &grant)?;
    Ok(Some(grant.access))
}
/// Delete this owner's grant. The next operation rebuilds or rejects the connection.
pub async fn logout(
    server: &agent_client_protocol::McpServerHttp,
    session: String,
    tenant: vw_api_types::id::TenantScope,
) -> Result<()> {
    let owner = FormOwner { session, tenant };
    let _guard = lock(server, &owner).await?;
    let binding = key(server, &owner)?;
    // Close in-flight calls even if storage deletion fails.
    if let Some(token) =
        cancellations().lock().unwrap_or_else(std::sync::PoisonError::into_inner).remove(&binding)
    {
        token.cancel();
    }
    remove(&root()?, &binding)
}

fn cancellations() -> &'static Mutex<HashMap<String, tokio_util::sync::CancellationToken>> {
    static TOKENS: OnceLock<Mutex<HashMap<String, tokio_util::sync::CancellationToken>>> =
        OnceLock::new();
    TOKENS.get_or_init(Mutex::default)
}
pub(super) fn connection_cancellation(
    server: &agent_client_protocol::McpServerHttp,
    owner: &FormOwner,
) -> Result<tokio_util::sync::CancellationToken> {
    let key = key(server, owner)?;
    Ok(cancellations()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .entry(key)
        .or_default()
        .child_token())
}
