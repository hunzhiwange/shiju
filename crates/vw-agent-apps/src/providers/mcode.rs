//! MiniMax Code custom-provider edits; shared transactions own publication.
use super::live::write_guard::WriteError;
use serde_json::{Value, json};
use std::{collections::BTreeMap, ffi::OsStr, path::Path};
use vw_api_types::{agent_apps::AppId, app_providers::mcode::*};
fn invalid<T>() -> Result<T, WriteError> {
    Err(WriteError::InvalidResource)
}
pub fn validate(p: &McodeProfile) -> Result<(), WriteError> {
    if p.name.trim().is_empty()
        || p.name.len() > 128
        || p.name.chars().any(char::is_control)
        || serde_json::to_vec(&p.declaration).map_err(|_| WriteError::InvalidResource)?.len()
            > 1024 * 1024
    {
        return invalid();
    }
    let obj = p.declaration.as_object().ok_or(WriteError::InvalidResource)?;
    if ["custom_provider", "defaultModel", "defaultLightModel", "minimax_api"]
        .iter()
        .any(|k| obj.contains_key(*k))
        || obj.get("kind").is_some_and(|v| v.as_str() != Some("custom"))
        || obj.get("enabled").is_some_and(|v| !v.is_boolean())
        || obj.get("name").is_some_and(|v| !v.is_string())
        || !matches!(
            obj.get("api").map_or(Some("anthropic-messages"), Value::as_str),
            Some("anthropic-messages" | "openai-completions" | "openai-responses")
        )
    {
        return invalid();
    }
    let options =
        obj.get("options").and_then(Value::as_object).ok_or(WriteError::InvalidResource)?;
    let endpoint =
        options.get("baseURL").and_then(Value::as_str).ok_or(WriteError::InvalidResource)?;
    if !reqwest::Url::parse(endpoint)
        .is_ok_and(|u| matches!(u.scheme(), "http" | "https") && u.host_str().is_some())
    {
        return invalid();
    }
    let key = options.get("apiKey").and_then(Value::as_str).ok_or(WriteError::InvalidResource)?;
    if key.trim().is_empty()
        || matches!(key, "[redacted]" | "<redacted>")
        || key.chars().all(|c| matches!(c, '*' | '•'))
    {
        return invalid();
    }
    if let Some(headers) = options.get("headers")
        && !headers.as_object().is_some_and(|m| m.values().all(Value::is_string))
    {
        return invalid();
    }
    let models = obj.get("models").and_then(Value::as_object).ok_or(WriteError::InvalidResource)?;
    if models.is_empty() {
        return invalid();
    }
    for (id, m) in models {
        if id.trim().is_empty()
            || !m.is_object()
            || m.get("name").is_some_and(|v| !v.is_string())
            || m.get("enabled").is_some_and(|v| !v.is_boolean())
            || m.get("options").is_some_and(|v| !v.is_object())
        {
            return invalid();
        }
        if let Some(limit) = m.get("limit")
            && (!limit.is_object()
                || ["context", "output"]
                    .iter()
                    .any(|k| limit.get(*k).is_some_and(|v| !v.is_number())))
        {
            return invalid();
        }
    }
    Ok(())
}
fn document(pre: Option<&[u8]>) -> Result<Value, WriteError> {
    let bytes = pre.unwrap_or_default();
    if bytes.len() > 1024 * 1024 {
        return invalid();
    }
    let yaml: serde_yaml::Value =
        serde_yaml::from_slice(bytes).map_err(|_| WriteError::InvalidResource)?;
    let doc = serde_json::to_value(yaml).map_err(|_| WriteError::InvalidResource)?;
    let doc = if doc.is_null() { json!({}) } else { doc };
    if !doc.is_object()
        || doc.get("custom_provider").is_some_and(|v| !v.is_null() && !v.is_object())
    {
        return invalid();
    }
    Ok(doc)
}
pub fn declarations(pre: Option<&[u8]>) -> Result<BTreeMap<String, McodeProfile>, WriteError> {
    let doc = document(pre)?;
    let mut rows = BTreeMap::new();
    if let Some(map) = doc.get("custom_provider").and_then(Value::as_object) {
        for (id, v) in map {
            if !v.is_object() {
                return invalid();
            }
            if v.get("kind").and_then(Value::as_str).is_some_and(|kind| kind != "custom") {
                continue;
            }
            super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
            rows.insert(
                id.clone(),
                McodeProfile {
                    name: v.get("name").and_then(Value::as_str).unwrap_or(id).into(),
                    declaration: v.clone(),
                },
            );
        }
    }
    Ok(rows)
}
pub fn state(pre: Option<&[u8]>) -> Result<McodeState, WriteError> {
    Ok(McodeState { members: declarations(pre)?.into_keys().collect() })
}
/// Accept exactly custom_provider.<id>; credentials never leave the execution service.
pub fn parse_editor(id: &str, yaml: &str) -> Result<Value, WriteError> {
    super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    let doc = document(Some(yaml.as_bytes()))?;
    if doc.as_object().is_none_or(|m| m.len() != 1) {
        return invalid();
    }
    let nodes =
        doc.get("custom_provider").and_then(Value::as_object).ok_or(WriteError::InvalidResource)?;
    if nodes.len() != 1 {
        return invalid();
    }
    let node = nodes.get(id).ok_or(WriteError::InvalidResource)?.clone();
    validate(&McodeProfile { name: id.into(), declaration: node.clone() })?;
    Ok(node)
}
pub fn patch(
    pre: Option<&[u8]>,
    id: &str,
    declaration: Option<&Value>,
    activation: McodeActivation,
) -> Result<Vec<u8>, WriteError> {
    super::validate_identity(id).map_err(|_| WriteError::InvalidResource)?;
    let doc = document(pre)?;
    let previous = doc.get("custom_provider").and_then(|m| m.get(id));
    if previous
        .and_then(|v| v.get("kind"))
        .and_then(Value::as_str)
        .is_some_and(|kind| kind != "custom")
    {
        return invalid();
    }
    if let Some(node) = declaration {
        validate(&McodeProfile { name: id.into(), declaration: node.clone() })?;
        match activation {
            McodeActivation::CatalogOnly => return invalid(),
            McodeActivation::Add if previous.is_some() => {
                return if previous == Some(node) {
                    Ok(pre.unwrap_or_default().to_vec())
                } else {
                    Err(WriteError::Conflict)
                };
            }
            McodeActivation::IfMember if previous.is_none() => {
                return Ok(pre.unwrap_or_default().to_vec());
            }
            _ => {}
        }
    }
    let prefix = format!("custom_provider:{id}/");
    for field in ["defaultModel", "defaultLightModel"] {
        if let Some(model) =
            doc.get(field).and_then(Value::as_str).and_then(|s| s.strip_prefix(&prefix))
            && !declaration.is_some_and(|v| {
                v.get("enabled") != Some(&Value::Bool(false))
                    && v.get("models").and_then(|m| m.get(model)).is_some_and(|m| {
                        m.is_object() && m.get("enabled") != Some(&Value::Bool(false))
                    })
            })
        {
            return invalid();
        }
    }
    super::live::patch_yaml::patch_provider(
        pre,
        super::live::patch_yaml::ProviderSection::CustomProvider,
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
        .unwrap_or_else(|| home.join(".minimax"));
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
pub(super) fn native_components(
    home: &Path,
    minimax: Option<&OsStr>,
    mavis: Option<&OsStr>,
) -> Result<Vec<String>, WriteError> {
    let raw = [minimax, mavis]
        .into_iter()
        .flatten()
        .find(|v| v.to_str().is_none_or(|v| !v.trim().is_empty()));
    directory_components(home, raw)
}
fn components(home: &Path) -> Result<Vec<String>, WriteError> {
    native_components(
        home,
        std::env::var_os("MINIMAX_DATA_DIR").as_deref(),
        std::env::var_os("MAVIS_DATA_DIR").as_deref(),
    )
}
/// Read only the configured Mcode directory beneath the authorized execution HOME.
pub fn read(home: &Path) -> Result<Option<Vec<u8>>, WriteError> {
    let parts = components(home)?;
    let mut components: Vec<&str> = parts.iter().map(String::as_str).collect();
    components.push("config.yaml");
    super::catalog::read_file(home, &components).map_err(|_| WriteError::InvalidResource)
}
pub struct McodeFiles {
    native_lock: NativeLock,
    pub guard: super::live::write_guard::AppWriteGuard,
}
impl McodeFiles {
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
        let native_lock = NativeLock::acquire(&dir)?;
        Ok(Self {
            native_lock,
            guard: super::live::write_guard::lock_directory(AppId::Mcode, dir)?,
        })
    }
    pub fn snapshot(&self) -> Result<(&'static str, Option<Vec<u8>>), WriteError> {
        Ok(("config.yaml", self.guard.read(OsStr::new("config.yaml"))?))
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
        if name != "config.yaml" {
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
        let native_lock = &self.native_lock;
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
                if native_lock.is_current() && unchanged() && tx.state(AppId::Mcode)? == *expected {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}

/// Includes native account nodes, which must never be shadowed by a catalog create.
pub fn key_exists(pre: Option<&[u8]>, id: &str) -> Result<bool, WriteError> {
    Ok(document(pre)?.get("custom_provider").and_then(|m| m.get(id)).is_some())
}
/// Cooperate with Mcode's proper-lockfile directory. Existing locks require native recovery.
/// Keep its mtime alive while the shared transaction and any recovery use this directory.
struct NativeLock {
    directory: std::fs::File,
    inode: u64,
    device: u64,
    stop: Option<std::sync::mpsc::Sender<()>>,
    worker: Option<std::thread::JoinHandle<()>>,
}
impl NativeLock {
    fn acquire(directory: &std::fs::File) -> Result<Self, WriteError> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::MetadataExt,
        };
        let directory = directory.try_clone().map_err(|_| WriteError::Io)?;
        // SAFETY: fixed single child of the retained authorized directory.
        if unsafe { libc::mkdirat(directory.as_raw_fd(), c"config.yaml.lock".as_ptr(), 0o700) } != 0
        {
            return Err(WriteError::Conflict);
        }
        // SAFETY: fixed child created above; no symlinks; descriptor consumed exactly once.
        let fd = unsafe {
            libc::openat(
                directory.as_raw_fd(),
                c"config.yaml.lock".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(WriteError::Io);
        }
        let file = unsafe { std::fs::File::from_raw_fd(fd) };
        let metadata = file.metadata().map_err(|_| WriteError::Io)?;
        let mut lock = Self {
            directory,
            inode: metadata.ino(),
            device: metadata.dev(),
            stop: None,
            worker: None,
        };
        let (stop, recv) = std::sync::mpsc::channel();
        let worker = std::thread::Builder::new()
            .name("mcode-config-lock".into())
            .spawn(move || {
                while recv.recv_timeout(std::time::Duration::from_secs(2))
                    == Err(std::sync::mpsc::RecvTimeoutError::Timeout)
                {
                    if file.set_modified(std::time::SystemTime::now()).is_err() {
                        break;
                    }
                }
            })
            .map_err(|_| WriteError::Io)?;
        lock.stop = Some(stop);
        lock.worker = Some(worker);
        Ok(lock)
    }
    fn is_current(&self) -> bool {
        use std::os::fd::AsRawFd;
        let mut stat = std::mem::MaybeUninit::<libc::stat>::uninit();
        // SAFETY: valid output buffer, retained directory and literal child name.
        if unsafe {
            libc::fstatat(
                self.directory.as_raw_fd(),
                c"config.yaml.lock".as_ptr(),
                stat.as_mut_ptr(),
                libc::AT_SYMLINK_NOFOLLOW,
            )
        } != 0
        {
            return false;
        }
        let stat = unsafe { stat.assume_init() };
        stat.st_ino == self.inode && stat.st_dev as u64 == self.device
    }
}
impl Drop for NativeLock {
    fn drop(&mut self) {
        use std::os::fd::AsRawFd;
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        if self.is_current() {
            // SAFETY: only remove the empty lock directory whose identity we still own.
            unsafe {
                libc::unlinkat(
                    self.directory.as_raw_fd(),
                    c"config.yaml.lock".as_ptr(),
                    libc::AT_REMOVEDIR,
                );
            }
        }
    }
}

#[path = "mcode_form.rs"]
pub mod form;
