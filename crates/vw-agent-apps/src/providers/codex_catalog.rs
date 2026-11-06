//! Codex adapter composes route, catalog and login preparations into the sole pending owner.
use super::live::{
    commit::*,
    stage::{FileChange, WholeFile},
    write_guard::*,
};
use serde_json::{Value, json};
use std::{ffi::OsStr, path::Path};
use vw_api_types::{
    agent_apps::AppId, app_providers::codex_catalog::CodexProfile, id::TenantScope,
    model_catalog::*,
};
pub const CATALOG_FILENAME: &str = "shiju-model-catalog.json";
pub fn settings(profile: &CodexProfile) -> Value {
    json!({"config":profile.config,"auth":{"OPENAI_API_KEY":profile.api_key}})
}
pub fn identity(
    host: &str,
    id: &str,
    profile: &CodexProfile,
) -> Result<ModelCatalogIdentity, WriteError> {
    let row: toml_edit::DocumentMut =
        profile.config.parse().map_err(|_| WriteError::InvalidResource)?;
    if row.get("cli_auth_credentials_store").is_some_and(|v| v.as_str() != Some("file")) {
        return Err(WriteError::InvalidResource);
    }
    let projected = super::codex::project("", &settings(profile), None, false)
        .map_err(|_| WriteError::InvalidResource)?;
    let doc = projected
        .config()
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| WriteError::InvalidResource)?;
    let endpoint = doc
        .get("model_provider")
        .and_then(toml_edit::Item::as_str)
        .and_then(|id| doc.get("model_providers")?.get(id)?.get("base_url")?.as_str())
        .ok_or(WriteError::InvalidResource)?;
    Ok(ModelCatalogIdentity {
        target_host_id: host.into(),
        app_id: AppId::Codex,
        provider_id: id.into(),
        endpoint: endpoint.into(),
    })
}
pub struct CodexFiles {
    pub guard: AppWriteGuard,
}
impl CodexFiles {
    pub fn open(home: &Path) -> Result<Self, WriteError> {
        use std::os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        };
        let home = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(home)
            .map_err(|_| WriteError::InvalidResource)?;
        // SAFETY: fixed child of the administrator-authorized retained HOME descriptor.
        if unsafe { libc::mkdirat(home.as_raw_fd(), c".codex".as_ptr(), 0o700) } != 0
            && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(WriteError::Io);
        }
        // SAFETY: fixed no-follow child, fresh fd consumed exactly once below.
        let fd = unsafe {
            libc::openat(
                home.as_raw_fd(),
                c".codex".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(WriteError::InvalidResource);
        }
        let dir = unsafe { std::fs::File::from_raw_fd(fd) };
        Ok(Self { guard: lock_directory(AppId::Codex, dir)? })
    }
    /// Read-only validation under the physical lock, before saved catalog changes.
    pub fn preflight(&self, profile: &CodexProfile) -> Result<(), WriteError> {
        let config = self.guard.read(OsStr::new("config.toml"))?;
        let config = std::str::from_utf8(config.as_deref().unwrap_or_default())
            .map_err(|_| WriteError::InvalidResource)?;
        super::codex::project(config, &settings(profile), None, false)
            .map_err(|_| WriteError::InvalidResource)?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        tx: &ProviderTransactions,
        tenant: &TenantScope,
        state: &AppState,
        id: &str,
        previous: Option<&Value>,
        profile: &CodexProfile,
        catalog: Option<&SavedCodexCatalog>,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        // Recovery changes state and must always force the caller to reload saved inputs.
        if tx.recover(&mut self.guard)?.is_some() {
            return Err(WriteError::Recovered);
        }
        if state.pointer.as_ref().is_some_and(|p| p.tenant != *tenant)
            || state.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(WriteError::Conflict);
        }
        let before = self.guard.read(OsStr::new("config.toml"))?;
        let live = std::str::from_utf8(before.as_deref().unwrap_or_default())
            .map_err(|_| WriteError::InvalidResource)?;
        let keys = previous
            .and_then(|v| v.pointer("/auth/OPENAI_API_KEY"))
            .and_then(Value::as_str)
            .map(|s| vec![s.into()])
            .unwrap_or_default();
        let login = super::codex_login::prepare(
            &self.guard,
            super::codex_login::LoginInput {
                target: super::codex_login::AuthTarget::ThirdParty {
                    preserve: profile.preserve_login,
                },
                leaving_official: None,
                official_logins: &[],
                third_party_keys: &keys,
                live_is_managed: false,
                now: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_err(|_| WriteError::Io)?
                    .as_secs() as i64,
            },
        )?;
        let result = super::codex::project_owned(
            live,
            &settings(profile),
            previous,
            login.login_on_disk,
            state
                .mode
                .as_ref()
                .and_then(|m| m.contract.as_ref())
                .filter(|c| {
                    c.version == 1
                        && state.mode_tenant.as_ref() == Some(tenant)
                        && state.pointer.as_ref().is_some_and(|p| p.id == c.key)
                })
                .map(|c| &c.exclusive),
        )
        .map_err(|_| WriteError::InvalidResource)?;
        let mut doc = result
            .config()
            .parse::<toml_edit::DocumentMut>()
            .map_err(|_| WriteError::InvalidResource)?;
        let row = profile
            .config
            .parse::<toml_edit::DocumentMut>()
            .map_err(|_| WriteError::InvalidResource)?;
        let foreign = row.get("model_catalog_json").and_then(toml_edit::Item::as_str).filter(|s| {
            !s.trim().is_empty()
                && std::path::Path::new(s).file_name().and_then(|n| n.to_str()).is_some_and(
                    |name| ![CATALOG_FILENAME, "cc-switch-model-catalog.json"].contains(&name),
                )
        });
        let bytes =
            crate::models::codex_projection(catalog).map_err(|_| WriteError::InvalidResource)?;
        if let Some(pointer) = foreign {
            doc.insert("model_catalog_json", toml_edit::value(pointer));
        } else if bytes.is_some() {
            doc.insert("model_catalog_json", toml_edit::value(CATALOG_FILENAME));
        } else {
            doc.remove("model_catalog_json");
        }
        let old_catalog = self.guard.read(OsStr::new(CATALOG_FILENAME))?;
        let catalog_patch = WholeFile::new(old_catalog.as_deref(), bytes);
        let config = WholeFile::new(before.as_deref(), Some(doc.to_string().into_bytes()));
        let guarded_config = |pre: Option<&[u8]>| {
            use super::live::stage::FilePatch;
            if !unchanged() || tx.state(AppId::Codex)? != *state {
                return Err(WriteError::Conflict);
            }
            config.apply(pre)?.ok_or(WriteError::InvalidResource)
        };
        let mut changes = login.changes().into_iter().collect::<Vec<_>>();
        changes.push(FileChange {
            name: OsStr::new("config.toml"),
            privacy: FilePrivacy::Private,
            patch: &guarded_config,
        });
        changes.push(FileChange {
            name: OsStr::new(CATALOG_FILENAME),
            privacy: FilePrivacy::Private,
            patch: &catalog_patch,
        });
        tx.commit_checked(
            tenant,
            &mut self.guard,
            &changes,
            PendingTarget {
                pointer: Some(id.into()),
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: state.mode.as_ref().and_then(|m| m.proxy_route.clone()),
                    contract: Some(Contract {
                        version: 1,
                        key: id.into(),
                        claude_login_operation: None,
                        exclusive: {
                            let row_value: Value = toml::from_str(&profile.config)
                                .map_err(|_| WriteError::InvalidResource)?;
                            super::projection::fields::CODEX_EXCLUSIVE_TOP
                                .iter()
                                .filter_map(|k| row_value.get(*k).map(|v| ((*k).into(), v.clone())))
                                .collect()
                        },
                    }),
                }),
            },
            || {
                if unchanged() && tx.state(AppId::Codex)? == *state {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}
