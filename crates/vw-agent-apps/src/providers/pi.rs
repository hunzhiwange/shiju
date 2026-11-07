//! Incremental Pi models.json membership; defaults and login files are never opened.
use super::live::{
    patch_json::{JsonPatch, object_value},
    write_guard::WriteError,
};
use serde_json::Value;
use std::{collections::BTreeMap, ffi::OsStr, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::pi::*};
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
pub fn declarations(pre: Option<&[u8]>) -> Result<BTreeMap<String, PiProfile>, WriteError> {
    let doc = object_value(pre.unwrap_or(b"{}")).map_err(|_| WriteError::InvalidResource)?;
    let mut rows = BTreeMap::new();
    if let Some(v) = doc.get("providers") {
        let map = v.as_object().ok_or(WriteError::InvalidResource)?;
        for (id, v) in map {
            super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
            let p = PiProfile {
                name: v.get("name").and_then(Value::as_str).unwrap_or(id).into(),
                declaration: v.clone(),
            };
            validate(&p)?;
            rows.insert(id.clone(), p);
        }
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
    if !home.is_absolute() {
        return invalid();
    }
    let raw = raw.map(|v| v.to_str().ok_or(WriteError::InvalidResource)).transpose()?;
    let path = raw
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| home.join(".pi/agent"));
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
    let mut components: Vec<&str> = parts.iter().map(String::as_str).collect();
    components.push("models.json");
    super::catalog::read_file(home, &components).map_err(|_| WriteError::InvalidResource)
}
pub struct PiFiles {
    pub guard: super::live::write_guard::AppWriteGuard,
}
impl PiFiles {
    pub fn open(home: &Path) -> Result<Self, WriteError> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        };
        if !home.is_absolute() {
            return invalid();
        }
        let mut dir = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(home)
            .map_err(|_| WriteError::InvalidResource)?;
        for component in
            directory_components(home, std::env::var_os("PI_CODING_AGENT_DIR").as_deref())?
        {
            let name =
                std::ffi::CString::new(component).map_err(|_| WriteError::InvalidResource)?;
            // SAFETY: fixed single child of the retained authorized directory.
            if unsafe { libc::mkdirat(dir.as_raw_fd(), name.as_ptr(), 0o700) } != 0
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
        Ok(Self { guard: super::live::write_guard::lock_directory(AppId::Pi, dir)? })
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
