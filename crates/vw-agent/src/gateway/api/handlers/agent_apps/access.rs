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
    platform_id: String,
    company_id: String,
    host_id: String,
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
    let mut values = headers.get_all("x-agent-apps-token").iter();
    let token = values
        .next()
        .and_then(|v| v.to_str().ok())
        .filter(|v| v.len() >= 32 && v.len() <= 512 && v.bytes().all(|b| b.is_ascii_graphic()))
        .ok_or(StatusCode::UNAUTHORIZED)?;
    if values.next().is_some() {
        return Err(StatusCode::UNAUTHORIZED);
    }
    let path = path.ok_or(StatusCode::SERVICE_UNAVAILABLE)?;
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
    if !principal.grants.iter().any(|g| {
        g.platform_id == tenant.platform_id().as_ref()
            && g.company_id == tenant.company_id().as_ref()
            && g.host_id == target
            && action.is_none_or(|a| g.actions.contains(&a))
            && (!write_host || g.preferences_write)
    }) {
        return Err(StatusCode::FORBIDDEN);
    }
    let host = policy.hosts.into_iter().find(|h| h.id == target).ok_or(StatusCode::NOT_FOUND)?;
    if !host.online {
        return Err(StatusCode::SERVICE_UNAVAILABLE);
    }
    if matches!(host.binding, Binding::Remote) {
        return Err(StatusCode::NOT_IMPLEMENTED);
    }
    Ok((host, principal.id.clone()))
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
                })
            {
                return Err(());
            }
        }
        Ok(policy)
    }
}
