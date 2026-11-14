//! Pi's native directory and incremental models.json membership.
use super::live::{
    patch_json::{JsonPatch, object_value},
    write_guard::WriteError,
};
use serde_json::{Map, Value};
use std::{collections::BTreeMap, ffi::OsStr, path::Path};
use vw_api_types::{
    agent_apps::AppId,
    app_providers::{catalog::ProviderReadUnavailable, pi::*},
};
pub mod form;
#[cfg(test)]
mod read_tests;
mod write;
pub use write::{PiDirectoryChange, PiDirectoryWriteError};
#[cfg(test)]
mod directory_project_tests;
#[cfg(test)]
mod write_tests;

fn invalid<T>() -> Result<T, WriteError> {
    Err(WriteError::InvalidResource)
}
pub fn validate(p: &PiProfile) -> Result<(), WriteError> {
    if p.name.trim().is_empty()
        || p.name.len() > 128
        || p.name.chars().any(char::is_control)
        || !p.declaration.is_object()
        || serde_json::to_vec(&p.declaration).map_err(|_| WriteError::InvalidResource)?.len()
            > 1024 * 1024
    {
        return invalid();
    }
    Ok(())
}
fn provider_entries(pre: Option<&[u8]>) -> Result<Map<String, Value>, WriteError> {
    let mut doc = object_value(pre.unwrap_or(b"{}")).map_err(|_| WriteError::InvalidResource)?;
    match doc.as_object_mut().and_then(|root| root.remove("providers")) {
        None => Ok(Map::new()),
        Some(Value::Object(providers)) => Ok(providers),
        Some(_) => invalid(),
    }
}
pub fn declarations(pre: Option<&[u8]>) -> Result<BTreeMap<String, PiProfile>, WriteError> {
    let mut rows = BTreeMap::new();
    for (id, declaration) in provider_entries(pre)? {
        super::validate_identity(&id).map_err(|_| WriteError::InvalidResource)?;
        let profile = PiProfile {
            name: declaration.get("name").and_then(Value::as_str).unwrap_or(&id).into(),
            declaration,
        };
        validate(&profile)?;
        rows.insert(id, profile);
    }
    Ok(rows)
}
pub fn state(pre: Option<&[u8]>) -> Result<PiState, WriteError> {
    Ok(PiState { members: declarations(pre)?.into_keys().collect() })
}
/// Add never replaces another declaration. IfMember edits cannot enable a disabled row.
pub fn patch(
    pre: Option<&[u8]>,
    id: &str,
    declaration: Option<&Value>,
    activation: PiActivation,
) -> Result<Vec<u8>, WriteError> {
    super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    let rows = declarations(pre)?;
    let mut patch = JsonPatch::default();
    if let Some(declaration) = declaration {
        validate(&PiProfile { name: id.into(), declaration: declaration.clone() })?;
        if activation == PiActivation::Add {
            if let Some(existing) = rows.get(id) {
                if existing.declaration != *declaration {
                    return Err(WriteError::Conflict);
                }
                return Ok(pre.unwrap_or_default().to_vec());
            }
        } else if activation == PiActivation::IfMember && !rows.contains_key(id) {
            return Ok(pre.unwrap_or_default().to_vec());
        }
        patch.set.push((vec!["providers".into(), id.into()], declaration.clone()));
    } else {
        if !rows.contains_key(id) {
            return Ok(pre.unwrap_or_default().to_vec());
        }
        patch.remove.push(vec!["providers".into(), id.into()]);
    }
    patch.apply(pre).map_err(|_| WriteError::InvalidResource)
}
pub(super) fn directory_components(
    home: &Path,
    raw: Option<&OsStr>,
) -> Result<Vec<String>, WriteError> {
    let raw = raw.map(|v| v.to_str().ok_or(WriteError::InvalidResource)).transpose()?;
    let path = raw
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".pi/agent"));
    checked_directory_components(home, &path)
}

fn resolve_directory(
    home: &Path,
    configured_dir: Option<&Path>,
    env_dir: Option<&OsStr>,
) -> Result<Vec<String>, WriteError> {
    match configured_dir {
        Some(path) => checked_directory_components(home, path),
        None => directory_components(home, env_dir),
    }
}

fn checked_directory_components(home: &Path, path: &Path) -> Result<Vec<String>, WriteError> {
    if !home.is_absolute() || !path.is_absolute() {
        return invalid();
    }
    let relative = path.strip_prefix(home).map_err(|_| WriteError::InvalidResource)?;
    let mut parts = vec![];
    for part in relative.components() {
        let std::path::Component::Normal(part) = part else {
            return invalid();
        };
        parts.push(part.to_str().ok_or(WriteError::InvalidResource)?.to_string());
    }
    if parts.is_empty() {
        return invalid();
    }
    Ok(parts)
}
/// Read only the configured Pi directory beneath the authorized execution HOME.
pub fn read(home: &Path) -> Result<Option<Vec<u8>>, WriteError> {
    let parts = directory_components(home, std::env::var_os("PI_CODING_AGENT_DIR").as_deref())?;
    read_native_file(home, &parts, "models.json").map_err(|_| WriteError::InvalidResource)
}

fn read_native_file(
    home: &Path,
    parts: &[String],
    name: &str,
) -> Result<Option<Vec<u8>>, ProviderReadUnavailable> {
    let mut components: Vec<&str> = parts.iter().map(String::as_str).collect();
    components.push(name);
    super::catalog::read_file(home, &components)
}

#[derive(Debug, PartialEq, Eq)]
pub struct PiNativeDirectory {
    pub source_revision: String,
    pub members: Vec<String>,
    pub default_provider_id: Option<String>,
    pub default_model_id: Option<String>,
}

/// Directory overrides must be absolute paths beneath the authorized execution HOME.
/// Pass the target host's PI_CODING_AGENT_DIR as `env_dir`.
pub fn read_native_directory(
    home: &Path,
    configured_dir: Option<&Path>,
    env_dir: Option<&OsStr>,
) -> Result<PiNativeDirectory, ProviderReadUnavailable> {
    use ProviderReadUnavailable::InvalidConfiguration;

    let parts =
        resolve_directory(home, configured_dir, env_dir).map_err(|_| InvalidConfiguration)?;
    let models = read_native_file(home, &parts, "models.json")?;
    let providers = provider_entries(models.as_deref()).map_err(|_| InvalidConfiguration)?;
    let mut members: Vec<_> = providers.into_iter().map(|(id, _)| id).collect();
    members.sort_unstable();
    let settings = read_native_file(home, &parts, "settings.json")?;
    let settings =
        object_value(settings.as_deref().unwrap_or(b"{}")).map_err(|_| InvalidConfiguration)?;
    Ok(PiNativeDirectory {
        source_revision: write::source_revision(&home.join(parts.join("/")), models.as_deref()),
        members,
        default_provider_id: default_setting(&settings, "defaultProvider")?,
        default_model_id: default_setting(&settings, "defaultModel")?,
    })
}

fn default_setting(settings: &Value, key: &str) -> Result<Option<String>, ProviderReadUnavailable> {
    match settings.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(ProviderReadUnavailable::InvalidConfiguration),
    }
}

pub struct PiFiles {
    directory: std::path::PathBuf,
    pub guard: super::live::write_guard::AppWriteGuard,
}
impl PiFiles {
    pub fn open(home: &Path) -> Result<Self, WriteError> {
        Self::open_directory(home, None, std::env::var_os("PI_CODING_AGENT_DIR").as_deref())
    }
    pub fn open_directory(
        home: &Path,
        configured_dir: Option<&Path>,
        env_dir: Option<&OsStr>,
    ) -> Result<Self, WriteError> {
        Self::open_directory_inner(home, configured_dir, env_dir, true)
    }
    /// Read/API entry: never creates the native directory as a side effect.
    pub fn open_existing_directory(
        home: &Path,
        configured_dir: Option<&Path>,
        env_dir: Option<&OsStr>,
    ) -> Result<Self, WriteError> {
        Self::open_directory_inner(home, configured_dir, env_dir, false)
    }
    fn open_directory_inner(
        home: &Path,
        configured_dir: Option<&Path>,
        env_dir: Option<&OsStr>,
        create: bool,
    ) -> Result<Self, WriteError> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        };
        let parts = resolve_directory(home, configured_dir, env_dir)?;
        let directory = home.join(parts.join("/"));
        let mut dir = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(home)
            .map_err(|_| WriteError::InvalidResource)?;
        for component in parts {
            let name =
                std::ffi::CString::new(component).map_err(|_| WriteError::InvalidResource)?;
            // SAFETY: fixed single child of the retained authorized directory.
            if create && unsafe { libc::mkdirat(dir.as_raw_fd(), name.as_ptr(), 0o700) } != 0
                && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
            {
                return Err(WriteError::Io);
            }
            // SAFETY: fixed child; no symlinks; returned fd consumed once.
            let fd = unsafe {
                libc::openat(
                    dir.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return invalid();
            }
            dir = unsafe { std::fs::File::from_raw_fd(fd) };
        }
        Ok(Self { directory, guard: super::live::write_guard::lock_directory(AppId::Pi, dir)? })
    }
    /// Resolve only the fixed prompts child beneath an authorized Pi directory.
    /// Missing directories are empty for reads; writes prepare the private child.
    pub fn open_prompt_templates(
        home: &Path, configured_dir: Option<&Path>, env_dir: Option<&OsStr>, create: bool,
    ) -> Result<Option<super::live::write_guard::AppWriteGuard>, WriteError> {
        use std::os::{fd::{AsRawFd, FromRawFd}, unix::fs::OpenOptionsExt};
        let mut parts = resolve_directory(home, configured_dir, env_dir)?;
        parts.push("prompts".into());
        let mut directory = std::fs::OpenOptions::new().read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(home).map_err(|_| WriteError::InvalidResource)?;
        for part in parts {
            let name = std::ffi::CString::new(part).map_err(|_| WriteError::InvalidResource)?;
            if create && unsafe { libc::mkdirat(directory.as_raw_fd(), name.as_ptr(), 0o700) } != 0
                && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists {
                return Err(WriteError::Io);
            }
            let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC) };
            if fd < 0 {
                return if !create && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                    Ok(None)
                } else { Err(WriteError::InvalidResource) };
            }
            directory = unsafe { std::fs::File::from_raw_fd(fd) };
        }
        super::live::write_guard::lock_directory(AppId::Pi, directory).map(Some)
    }
    pub fn snapshot(&self) -> Result<(&'static str, Option<Vec<u8>>), WriteError> {
        Ok(("models.json", self.guard.read(OsStr::new("models.json"))?))
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        tx: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        expected: &super::live::commit::AppState,
        name: &str,
        pre: Option<&[u8]>,
        next: Vec<u8>,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        use super::live::{
            commit::*,
            stage::{FileChange, WholeFile},
            write_guard::FilePrivacy,
        };
        if name != "models.json" {
            return invalid();
        }
        if expected.pointer.is_some()
            || expected.mode_tenant.as_ref().is_some_and(|t| t != tenant)
            || expected.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(WriteError::Conflict);
        }
        let after = if pre.is_none() && next.is_empty() { None } else { Some(next) };
        let patch = WholeFile::new(pre, after);
        tx.commit_checked(
            tenant,
            &mut self.guard,
            &[FileChange { name: OsStr::new(name), privacy: FilePrivacy::Private, patch: &patch }],
            PendingTarget {
                pointer: None,
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: expected.mode.as_ref().and_then(|m| m.proxy_route.clone()),
                    contract: None,
                }),
                ..Default::default()
            },
            || {
                if unchanged() && tx.state(AppId::Pi)? == *expected {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}
