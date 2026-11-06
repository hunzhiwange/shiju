//! Read-only observations of tenant records and explicitly authorized host configuration.
use std::{collections::BTreeMap, path::Path};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{AppProviderSummary, catalog::*},
};

/// `home` is the administrator-authorized execution home, never a client path.
/// Native reads currently cover Kimi TOML and OpenCode/OpenClaw JSON/JSONC at their default paths.
/// No CC database, ambient client home, separate credential files or processes are read.
/// Configurations can contain secrets; only the typed metadata projection leaves this owner.
pub fn read(home: &Path, app: AppId, rows: Vec<AppProviderSummary>) -> ProviderCatalog {
    let unavailable = |reason| ProviderSelection::Unavailable { reason };
    // Kimi is exceptional: its native default_model provides a selected member.
    let additive = matches!(
        app,
        AppId::OpenCode
            | AppId::OpenClaw
            | AppId::Hermes
            | AppId::Pi
            | AppId::Mcode
            | AppId::Dsh
            | AppId::Omp
    );
    let mut result = ProviderCatalog {
        app_id: app,
        rows: rows
            .into_iter()
            .filter(|p| p.app_id == app)
            .map(|provider| ProviderCatalogRow {
                provider,
                native_member: ProviderMembership::Unknown,
            })
            .collect(),
        direct: if additive {
            ProviderSelection::NotApplicable
        } else {
            unavailable(ProviderReadUnavailable::NotImplemented)
        },
        in_use: if additive {
            ProviderSelection::NotApplicable
        } else {
            unavailable(ProviderReadUnavailable::NotImplemented)
        },
        native_read: None,
    };
    match observe(home, app) {
        Ok((members, selection)) => {
            let mut rows: BTreeMap<_, _> = result
                .rows
                .into_iter()
                .map(|mut row| {
                    row.native_member = if members.contains_key(&row.provider.id) {
                        ProviderMembership::Present
                    } else {
                        ProviderMembership::Absent
                    };
                    (row.provider.id.clone(), row)
                })
                .collect();
            for id in members.into_keys() {
                rows.entry(id.clone()).or_insert_with(|| ProviderCatalogRow {
                    provider: AppProviderSummary {
                        id: id.clone(),
                        app_id: app,
                        name: id,
                        configured: true,
                    },
                    native_member: ProviderMembership::Present,
                });
            }
            result.rows = rows.into_values().collect();
            result.in_use = selection;
        }
        Err(reason) => {
            result.native_read = Some(reason);
            if !additive {
                result.in_use = unavailable(reason);
            }
        }
    }
    result
}

type Members = BTreeMap<String, ()>;
pub(super) fn observe(
    home: &Path,
    app: AppId,
) -> Result<(Members, ProviderSelection), ProviderReadUnavailable> {
    use ProviderReadUnavailable::*;
    if !cfg!(target_os = "macos") {
        return Err(UnsupportedPlatform);
    }
    #[cfg(target_os = "macos")]
    if app == AppId::OpenCode {
        let mut bytes = None;
        for name in ["opencode.jsonc", "opencode.json"] {
            bytes = read_file(home, &[".config", "opencode", name])?;
            if bytes.is_some() {
                break;
            }
        }
        let rows =
            super::opencode::declarations(bytes.as_deref()).map_err(|_| InvalidConfiguration)?;
        return Ok((
            rows.into_keys().map(|id| (id, ())).collect(),
            ProviderSelection::NotApplicable,
        ));
    }
    #[cfg(target_os = "macos")]
    if app == AppId::OpenClaw {
        let bytes = super::openclaw::read(home).map_err(|_| InvalidConfiguration)?;
        let rows =
            super::openclaw::declarations(bytes.as_deref()).map_err(|_| InvalidConfiguration)?;
        return Ok((
            rows.into_keys().map(|id| (id, ())).collect(),
            ProviderSelection::NotApplicable,
        ));
    }
    let components = match app {
        AppId::Kimi => {
            if std::env::var_os("KIMI_CODE_HOME").is_some_and(|v| !v.is_empty()) {
                return Err(CustomPathNotSupported);
            }
            vec![".kimi-code", "config.toml"]
        }
        _ => return Err(NotImplemented),
    };
    let Some(bytes) = read_file(home, &components)? else {
        return Ok((Members::new(), ProviderSelection::Unconfigured));
    };
    let value: serde_json::Value =
        toml::from_str(std::str::from_utf8(&bytes).map_err(|_| InvalidConfiguration)?)
            .map_err(|_| InvalidConfiguration)?;
    if !value.is_object() {
        return Err(InvalidConfiguration);
    }
    let mut members = Members::new();
    if let Some(providers) = value.get("providers") {
        let providers = providers.as_object().ok_or(InvalidConfiguration)?;
        for (id, config) in providers {
            if id.trim().is_empty()
                || id.len() > 128
                || id.chars().any(char::is_control)
                || !config.is_object()
            {
                return Err(InvalidConfiguration);
            }
            members.insert(id.clone(), ());
        }
    }
    if value.get("models").is_some_and(|v| !v.is_object()) {
        return Err(InvalidConfiguration);
    }
    let selection = match value.get("default_model") {
        None => ProviderSelection::Unconfigured,
        Some(selected) => {
            let alias = selected.as_str().ok_or(InvalidConfiguration)?;
            let id = value
                .get("models")
                .and_then(|m| m.get(alias))
                .and_then(|m| m.get("provider"))
                .and_then(|id| id.as_str())
                .ok_or(InvalidConfiguration)?;
            if !members.contains_key(id) {
                return Err(InvalidConfiguration);
            }
            ProviderSelection::Selected { id: id.into() }
        }
    };
    Ok((members, selection))
}

#[cfg(unix)]
pub(super) fn read_file(
    home: &Path,
    components: &[&str],
) -> Result<Option<Vec<u8>>, ProviderReadUnavailable> {
    use ProviderReadUnavailable::Unreadable;
    use std::{
        fs::OpenOptions,
        io::Read,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        },
    };
    if !home.is_absolute() {
        return Err(Unreadable);
    }
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(home)
        .map_err(|_| Unreadable)?;
    for (index, component) in components.iter().enumerate() {
        let last = index + 1 == components.len();
        let name = std::ffi::CString::new(*component).map_err(|_| Unreadable)?;
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK
            | if last { 0 } else { libc::O_DIRECTORY };
        // SAFETY: directory is live and name is a fixed single NUL-terminated component.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            let error = std::io::Error::last_os_error();
            return if error.kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(Unreadable)
            };
        }
        // SAFETY: openat returned a new owned descriptor, consumed once.
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        if last {
            let metadata = file.metadata().map_err(|_| Unreadable)?;
            if !metadata.is_file() || metadata.len() > 1024 * 1024 {
                return Err(Unreadable);
            }
            let mut bytes = Vec::new();
            file.take(1024 * 1024 + 1).read_to_end(&mut bytes).map_err(|_| Unreadable)?;
            if bytes.len() > 1024 * 1024 {
                return Err(Unreadable);
            }
            return Ok(Some(bytes));
        }
        directory = file;
    }
    Err(Unreadable)
}
#[cfg(not(unix))]
pub(super) fn read_file(
    _home: &Path,
    _components: &[&str],
) -> Result<Option<Vec<u8>>, ProviderReadUnavailable> {
    Err(ProviderReadUnavailable::UnsupportedPlatform)
}
