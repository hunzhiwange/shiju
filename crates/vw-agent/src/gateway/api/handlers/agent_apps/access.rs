//! Administrator-owned grants. Request headers never establish host ownership.
use axum::http::{HeaderMap, StatusCode};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashSet,
    io::Read,
    path::{Path, PathBuf},
};
use vw_api_types::id::TenantScope;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Policy {
    principals: Vec<Principal>,
    hosts: Vec<Host>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Principal {
    id: String,
    token_sha256: String,
    grants: Vec<Grant>,
}
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Grant {
    #[serde(default)]
    actions: Vec<vw_api_types::agent_apps::AppLifecycleAction>,
    #[serde(default)]
    preferences_write: bool,
    #[serde(default)]
    provider_config_read: bool,
    #[serde(default)]
    provider_config_write: bool,
    #[serde(default)]
    launch: bool,
    #[serde(default)]
    identities: Vec<IdentityPermit>,
    platform_id: String,
    company_id: String,
    host_id: String,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct IdentityPermit {
    pub app_id: vw_api_types::agent_apps::AppId,
    pub kind: vw_api_types::agent_apps::LaunchIdentityKind,
    pub id: String,
}
pub(super) struct Authorization {
    pub host: Host,
    pub principal: String,
    pub identities: Vec<IdentityPermit>,
}
impl Authorization {
    pub fn permits(
        &self,
        app: vw_api_types::agent_apps::AppId,
        kind: vw_api_types::agent_apps::LaunchIdentityKind,
        id: &str,
    ) -> bool {
        self.identities.iter().any(|p| p.app_id == app && p.kind == kind && p.id == id)
    }
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Host {
    pub id: String,
    pub online: bool,
    pub binding: Binding,
    pub home: PathBuf,
    pub search_paths: Vec<PathBuf>,
    pub applications: PathBuf,
    #[serde(default)]
    pub credential_root: Option<PathBuf>,
    #[serde(default)]
    pub lifecycle: Option<Lifecycle>,
}
#[derive(Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Lifecycle {
    pub prefix: PathBuf,
    pub npm: PathBuf,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum Binding {
    Local,
    Remote,
}

pub(super) fn authorize(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<Host, StatusCode> {
    authorize_action(path, headers, tenant, target, None)
}

pub(super) fn authorize_action(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    action: Option<vw_api_types::agent_apps::AppLifecycleAction>,
) -> Result<Host, StatusCode> {
    authorize_preferences(path, headers, tenant, target, action, false).map(|(host, _)| host)
}

pub(super) fn authorize_preferences(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    action: Option<vw_api_types::agent_apps::AppLifecycleAction>,
    write_host: bool,
) -> Result<(Host, String), StatusCode> {
    authorize_scoped(
        path,
        headers,
        tenant,
        target,
        action,
        if write_host { Capability::PreferencesWrite } else { Capability::Read },
    )
    .map(|a| (a.host, a.principal))
}

pub(super) fn authorize_launch_session(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<(Host, String), StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::Launch)
        .map(|a| (a.host, a.principal))
}
pub(super) fn authorize_identity_launch(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<Authorization, StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::Launch)
}

#[derive(Clone, Copy)]
enum Capability {
    Read,
    PreferencesWrite,
    Launch,
    ProviderConfigRead,
    ProviderConfigWrite,
}

fn authorize_scoped(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    action: Option<vw_api_types::agent_apps::AppLifecycleAction>,
    capability: Capability,
) -> Result<Authorization, StatusCode> {
    let mut values = headers.get_all("x-agent-apps-token").iter();
    let token = values
        .next()
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() >= 32 && v.len() <= 512 && v.bytes().all(|b| b.is_ascii_graphic()))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if values.next().is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let local_path;
    let path = match path {
        Some(path) => path,
        None => {
            local_path = vw_shared::local_apps::policy_path(
                &crate::global::paths().home,
                tenant.platform_id().as_ref(),
                tenant.company_id().as_ref(),
            );
            &local_path
        }
    };
    let policy = load(path).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
    let digest = hex::encode(Sha256::digest(token));
    let principal = policy
        .principals
        .iter()
        .find(|principal| {
            crate::security::pairing::constant_time_eq(&principal.token_sha256, &digest)
        })
        .ok_or(StatusCode::UNAUTHORIZED)?;
    // Do not reveal whether an unauthorized host exists.
    let grants: Vec<_> = principal
        .grants
        .iter()
        .filter(|g| {
            g.platform_id == tenant.platform_id().as_ref()
                && g.company_id == tenant.company_id().as_ref()
                && g.host_id == target
                && action.is_none_or(|a| g.actions.contains(&a))
                && match capability {
                    Capability::Read => true,
                    Capability::PreferencesWrite => g.preferences_write,
                    Capability::Launch => g.launch,
                    Capability::ProviderConfigRead => g.provider_config_read,
                    Capability::ProviderConfigWrite => g.provider_config_write,
                }
        })
        .collect();
    if grants.is_empty() {
        return Err(StatusCode::FORBIDDEN);
    }
    let identities = grants.into_iter().flat_map(|g| g.identities.clone()).collect();
    let host = policy.hosts.into_iter().find(|h| h.id == target).ok_or(StatusCode::NOT_FOUND)?;
    if !host.online {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if matches!(host.binding, Binding::Remote) {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }
    Ok(Authorization { host, principal: principal.id.clone(), identities })
}

fn load(path: &Path) -> Result<Policy, ()> {
    if !path.is_absolute() {
        return Err(());
    }
    #[cfg(not(unix))]
    {
        let _ = path;
        Err(())
    }
    #[cfg(unix)]
    {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::{MetadataExt, OpenOptionsExt},
        };
        let directory = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(path.parent().ok_or(())?)
            .map_err(|_| ())?;
        let parent = directory.metadata().map_err(|_| ())?;
        // The configured directory must not allow another OS principal to replace grants.
        let uid = unsafe { libc::geteuid() };
        if !parent.is_dir()
            || parent.mode() & 0o022 != 0
            || (parent.uid() != uid && parent.uid() != 0)
        {
            return Err(());
        }
        use std::os::unix::ffi::OsStrExt;
        let name =
            std::ffi::CString::new(path.file_name().ok_or(())?.as_bytes()).map_err(|_| ())?;
        // Keep the validated directory open so path replacement cannot change its identity.
        // SAFETY: directory is live and name is a NUL-terminated single path component.
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                name.as_ptr(),
                libc::O_RDONLY | libc::O_NOFOLLOW | libc::O_NONBLOCK | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(());
        }
        // SAFETY: openat returned an owned descriptor, consumed exactly once here.
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        let metadata = file.metadata().map_err(|_| ())?;
        if !metadata.is_file()
            || metadata.mode() & 0o077 != 0
            || (metadata.uid() != uid && metadata.uid() != 0)
            || metadata.len() > 65536
        {
            return Err(());
        }
        let mut bytes = Vec::new();
        file.take(65537).read_to_end(&mut bytes).map_err(|_| ())?;
        if bytes.len() > 65536 {
            return Err(());
        }
        let policy: Policy = serde_json::from_slice(&bytes).map_err(|_| ())?;
        let mut ids = HashSet::new();
        for host in &policy.hosts {
            if host.id.trim().is_empty()
                || !ids.insert(&host.id)
                || !host.home.is_absolute()
                || !host.applications.is_absolute()
                || host.credential_root.as_ref().is_some_and(|root| !root.is_absolute())
                || host.search_paths.len() > 64
                || host.search_paths.iter().any(|p| !p.is_absolute())
            {
                return Err(());
            }
        }
        let mut principals = HashSet::new();
        let mut digests = HashSet::new();
        for principal in &policy.principals {
            if principal.id.trim().is_empty()
                || !principals.insert(&principal.id)
                || !digests.insert(&principal.token_sha256)
                || principal.token_sha256.len() != 64
                || !principal
                    .token_sha256
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || principal.grants.iter().any(|g| {
                    g.platform_id.trim().is_empty()
                        || g.company_id.trim().is_empty()
                        || !ids.contains(&g.host_id)
                        || g.identities
                            .iter()
                            .any(|p| vw_agent_apps::providers::validate_identity(&p.id).is_err())
                })
            {
                return Err(());
            }
        }
        Ok(policy)
    }
}

/// Configuration metadata reads require an independent administrator grant.
pub(super) fn authorize_provider_config(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<Host, StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::ProviderConfigRead)
        .map(|a| a.host)
}

#[cfg(target_os = "macos")]
pub(super) fn authorize_provider_discovery(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<(Host, String), StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::ProviderConfigRead)
        .map(|a| (a.host, a.principal))
}

/// Startup consumes the same administrator-owned host identities and private
/// roots as account execution. No request path, token or ambient HOME fallback.
#[cfg(target_os = "macos")]
pub(super) fn recover_provider_policy(path: &Path) -> Result<(), ()> {
    use vw_agent_apps::providers::live::{
        commit::{ExecutionHostId, ProviderTransactions},
        recovery::RecoveryOutcome,
    };
    for host in load(path)?.hosts {
        if !host.online || !matches!(host.binding, Binding::Local) {
            continue;
        }
        let Some(root) = host.credential_root else {
            continue;
        };
        // Unused hosts have no transactions. Do not provision a root at startup.
        if !root.join("provider-live").try_exists().map_err(|_| ())? {
            continue;
        }
        let transactions =
            ProviderTransactions::new(&root, ExecutionHostId::new(&host.id).map_err(|_| ())?)
                .map_err(|_| ())?;
        let login = if host.home.join(".claude").exists() {
            Some(
                vw_agent_apps::providers::claude_login::LoginHost::open(&host.home)
                    .map_err(|_| ())?,
            )
        } else {
            None
        };
        if let Some(login) = &login
            && !login.journal_matches_host(&transactions).map_err(|_| ())?
        {
            tracing::warn!(
                "provider recovery deferred; login intent belongs to another execution host"
            );
            continue;
        }
        for (app, outcome) in transactions.recover_on_startup() {
            match outcome {
                Ok(RecoveryOutcome::Discarded) => {
                    tracing::info!(?app, "discarded unpublished provider transaction")
                }
                Ok(RecoveryOutcome::RolledForward) => {
                    tracing::info!(?app, "recovered provider transaction")
                }
                Ok(RecoveryOutcome::RolledForwardExcept { names }) => tracing::warn!(
                    ?app,
                    skipped = names.len(),
                    "partially recovered provider transaction; external files preserved"
                ),
                Err(_) => tracing::warn!(?app, "provider recovery failed; pending intent retained"),
            }
        }
        if let Some(login) = login {
            login
                .recover(&transactions, &vw_agent_apps::auth::claude_keychain::MacOsKeychain)
                .map_err(|_| ())?;
        }
    }
    Ok(())
}

pub(super) fn authorize_provider_write(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
) -> Result<Host, StatusCode> {
    authorize_scoped(path, headers, tenant, target, None, Capability::ProviderConfigWrite)
        .map(|a| a.host)
}

/// Configuration writes do not grant access to every registered official identity.
pub(super) fn authorize_provider_login(
    path: Option<&Path>,
    headers: &HeaderMap,
    tenant: &TenantScope,
    target: &str,
    account: &str,
) -> Result<(), StatusCode> {
    let authorization =
        authorize_scoped(path, headers, tenant, target, None, Capability::ProviderConfigWrite)?;
    if !authorization.permits(
        vw_api_types::agent_apps::AppId::Claude,
        vw_api_types::agent_apps::LaunchIdentityKind::OfficialAccount,
        account,
    ) {
        return Err(StatusCode::FORBIDDEN);
    }
    Ok(())
}

/// Import only empty exclusive catalogs for administrator-authorized startup hosts.
#[cfg(target_os = "macos")]
pub(super) fn import_provider_policy(
    path: &Path,
    database: &crate::global_database::GlobalDatabase,
) -> Result<(), ()> {
    use vw_api_types::{agent_apps::AppId, app_providers::import::ImportTrigger};
    let policy = load(path)?;
    for principal in &policy.principals {
        for grant in &principal.grants {
            if !grant.provider_config_read || !grant.provider_config_write {
                continue;
            }
            let Some(host) = policy
                .hosts
                .iter()
                .find(|h| h.id == grant.host_id && h.online && matches!(h.binding, Binding::Local))
            else {
                continue;
            };
            let tenant = TenantScope::new(&grant.platform_id, &grant.company_id).map_err(|_| ())?;
            let store =
                crate::app_providers::AppProviderStore::new(database.clone()).map_err(|_| ())?;
            for app in [AppId::Claude, AppId::Codex, AppId::Gemini, AppId::GrokBuild, AppId::Agy] {
                if store.import_native(&tenant, &host.home, app, ImportTrigger::Startup).is_err() {
                    // Missing, unsupported and proxy-owned native files are all
                    // deferred. Never log their bytes or error payloads.
                    tracing::debug!(?app, "native provider startup import deferred");
                }
            }
        }
    }
    Ok(())
}
