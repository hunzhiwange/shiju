//! Incremental Omp models.yml membership; defaults and login files are never opened.
use super::live::write_guard::WriteError;
#[path = "omp_validation.rs"]
mod validation;
use serde_json::Value;
use std::{collections::BTreeMap, ffi::OsStr, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::omp::*};
fn invalid<T>() -> Result<T, WriteError> {
    Err(WriteError::InvalidResource)
}
pub fn validate(p: &OmpProfile) -> Result<(), WriteError> {
    if p.name.trim().is_empty()
        || p.name.len() > 128
        || p.name.chars().any(char::is_control)
        || !p.declaration.is_object()
        || serde_json::to_vec(&p.declaration).map_err(|_| WriteError::InvalidResource)?.len()
            > 1024 * 1024
    {
        return invalid();
    }
    validation::validate_provider("member", &p.declaration)
}
pub fn declarations(pre: Option<&[u8]>) -> Result<BTreeMap<String, OmpProfile>, WriteError> {
    let bytes = pre.unwrap_or_default();
    if bytes.len() > 1024 * 1024 {
        return invalid();
    }
    let yaml: serde_yaml::Value =
        serde_yaml::from_slice(bytes).map_err(|_| WriteError::InvalidResource)?;
    let doc = serde_json::to_value(yaml).map_err(|_| WriteError::InvalidResource)?;
    let doc = if doc.is_null() { serde_json::json!({}) } else { doc };
    if !doc.is_object() {
        return invalid();
    }
    let mut rows = BTreeMap::new();
    if let Some(v) = doc.get("providers").filter(|v| !v.is_null()) {
        let map = v.as_object().ok_or(WriteError::InvalidResource)?;
        for (id, v) in map {
            super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
            let p = OmpProfile {
                name: v.get("name").and_then(Value::as_str).unwrap_or(id).into(),
                declaration: v.clone(),
            };
            if !p.declaration.is_object() {
                return invalid();
            }
            rows.insert(id.clone(), p);
        }
    }
    Ok(rows)
}
pub fn state(pre: Option<&[u8]>) -> Result<OmpState, WriteError> {
    Ok(OmpState { members: declarations(pre)?.into_keys().collect() })
}
/// Add never replaces another declaration. IfMember edits cannot enable a disabled row.
pub fn patch(
    pre: Option<&[u8]>,
    id: &str,
    declaration: Option<&Value>,
    activation: OmpActivation,
) -> Result<Vec<u8>, WriteError> {
    super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    let rows = declarations(pre)?;
    if let Some(declaration) = declaration {
        validate(&OmpProfile { name: id.into(), declaration: declaration.clone() })?;
        match activation {
            OmpActivation::CatalogOnly => return invalid(),
            OmpActivation::Add => {
                if let Some(existing) = rows.get(id) {
                    return if existing.declaration == *declaration {
                        Ok(pre.unwrap_or_default().to_vec())
                    } else {
                        Err(WriteError::Conflict)
                    };
                }
            }
            OmpActivation::IfMember if !rows.contains_key(id) => {
                return Ok(pre.unwrap_or_default().to_vec());
            }
            OmpActivation::IfMember => {}
        }
    } else if !rows.contains_key(id) {
        return Ok(pre.unwrap_or_default().to_vec());
    }
    super::live::patch_yaml::patch_provider(
        pre,
        super::live::patch_yaml::ProviderSection::Providers,
        id,
        declaration,
    )
    .map_err(|_| WriteError::InvalidResource)
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
        .unwrap_or_else(|| home.join(".omp/agent"));
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
/// OMP profile wins over an inherited Pi directory. All paths stay under authorized HOME.
pub(super) fn native_components(
    home: &Path,
    config: Option<&str>,
    selected: Option<&str>,
    inherited: Option<&str>,
    agent: Option<&OsStr>,
) -> Result<Vec<String>, WriteError> {
    let profile = |raw: Option<&str>| -> Result<Option<String>, WriteError> {
        let name = raw.unwrap_or_default().trim();
        if name.is_empty() || name == "default" {
            return Ok(None);
        }
        let reserved = name.split('.').next().unwrap_or_default().to_ascii_uppercase();
        if name.len() > 64
            || !name.bytes().next().is_some_and(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
            || !name
                .bytes()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || b"._-".contains(&c))
            || name.ends_with('.')
            || matches!(reserved.as_str(), "CON" | "PRN" | "AUX" | "NUL")
            || (reserved.len() == 4
                && (reserved.starts_with("COM") || reserved.starts_with("LPT"))
                && reserved.as_bytes()[3].is_ascii_digit())
        {
            return invalid();
        }
        Ok(Some(name.into()))
    };
    let chosen = profile(selected.or(inherited))?;
    let root =
        home.join(config.filter(|c| !c.is_empty()).unwrap_or(".omp").trim_start_matches('/'));
    let path = if let Some(name) = chosen {
        root.join("profiles").join(name).join("agent")
    } else {
        let inherited_dir =
            profile(inherited)?.map(|name| root.join("profiles").join(name).join("agent"));
        let agent = agent.filter(|a| {
            !a.is_empty() && inherited_dir.as_ref().is_none_or(|p| p.as_os_str() != *a)
        });
        agent.map(std::path::PathBuf::from).unwrap_or_else(|| root.join("agent"))
    };
    directory_components(home, Some(path.as_os_str()))
}
fn components(home: &Path) -> Result<Vec<String>, WriteError> {
    let env = |key| {
        std::env::var(key).map(Some).or_else(|e| match e {
            std::env::VarError::NotPresent => Ok(None),
            _ => invalid(),
        })
    };
    native_components(
        home,
        env("PI_CONFIG_DIR")?.as_deref(),
        env("OMP_PROFILE")?.as_deref(),
        env("PI_PROFILE")?.as_deref(),
        std::env::var_os("PI_CODING_AGENT_DIR").as_deref(),
    )
}
/// Read only the configured Omp directory beneath the authorized execution HOME.
pub fn read(home: &Path) -> Result<Option<Vec<u8>>, WriteError> {
    let parts = components(home)?;
    let mut path: Vec<&str> = parts.iter().map(String::as_str).collect();
    for name in ["models.yml", "models.yaml", "models.json"] {
        path.push(name);
        let bytes =
            super::catalog::read_file(home, &path).map_err(|_| WriteError::InvalidResource)?;
        path.pop();
        if let Some(bytes) = bytes {
            return if name == "models.json" { invalid() } else { Ok(Some(bytes)) };
        }
    }
    Ok(None)
}

pub struct OmpFiles {
    directory: std::fs::File,
    pub guard: super::live::write_guard::AppWriteGuard,
}
impl OmpFiles {
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
        for component in components(home)? {
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
        let directory = dir.try_clone().map_err(|_| WriteError::Io)?;
        Ok(Self { directory, guard: super::live::write_guard::lock_directory(AppId::Omp, dir)? })
    }
    pub fn snapshot(&self) -> Result<(&'static str, Option<Vec<u8>>), WriteError> {
        for name in ["models.yml", "models.yaml", "models.json"] {
            if let Some(bytes) = self.guard.read(OsStr::new(name))? {
                return if name == "models.json" { invalid() } else { Ok((name, Some(bytes))) };
            }
        }
        Ok(("models.yml", None))
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
        if !matches!(name, "models.yml" | "models.yaml") {
            return invalid();
        }
        if expected.pointer.is_some()
            || expected.mode_tenant.as_ref().is_some_and(|t| t != tenant)
            || expected.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(WriteError::Conflict);
        }
        if self.snapshot()?.0 != name {
            return Err(WriteError::Conflict);
        }
        let after = if pre.is_none() && next.is_empty() { None } else { Some(next) };
        let patch = WholeFile::new(pre, after);
        let directory = &self.directory;
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
                if selection_unchanged(directory, name, pre.is_none())?
                    && unchanged()
                    && tx.state(AppId::Omp)? == *expected
                {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}

/// A newly created higher-priority file must force replanning before the publish marker.
fn selection_unchanged(
    directory: &std::fs::File,
    selected: &str,
    missing: bool,
) -> Result<bool, WriteError> {
    use std::os::fd::AsRawFd;
    for name in ["models.yml", "models.yaml", "models.json"] {
        if name == selected {
            continue;
        }
        if !missing && (selected == "models.yml" || name == "models.json") {
            continue;
        }
        let name = std::ffi::CString::new(name).map_err(|_| WriteError::InvalidResource)?;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: retained directory fd, fixed child name and valid stat output buffer.
        if unsafe {
            libc::fstatat(
                directory.as_raw_fd(),
                name.as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } == 0
        {
            return Ok(false);
        }
        if std::io::Error::last_os_error().kind() != std::io::ErrorKind::NotFound {
            return Err(WriteError::Io);
        }
    }
    Ok(true)
}
