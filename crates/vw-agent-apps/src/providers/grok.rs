//! Fixed Grok model-table adapter using the shared native projection and transaction owner.
use super::live::write_guard::WriteError;
use serde_json::{Value, json};
use toml_edit::{DocumentMut, Item};
use vw_api_types::app_providers::grok::*;

pub fn settings(profile: &GrokProfile) -> Result<Value, WriteError> {
    if profile.name.trim().is_empty()
        || profile.name.len() > 128
        || profile.name.chars().any(char::is_control)
    {
        return Err(WriteError::InvalidResource);
    }
    let text = match &profile.config {
        GrokConfig::Official {} => "",
        GrokConfig::ThirdParty { toml } => toml.as_str(),
    };
    if text.len() > 1024 * 1024 {
        return Err(WriteError::InvalidResource);
    }
    let row = json!({"config": text});
    super::projection::grok_document(
        DocumentMut::new(),
        &row,
        None,
        matches!(profile.config, GrokConfig::Official {}),
    )
    .map_err(|_| WriteError::InvalidResource)?;
    Ok(row)
}
/// Only the selected supplier table and managed model selectors are projected.
pub fn patch(
    pre: Option<&[u8]>,
    profile: &GrokProfile,
    written: Option<&str>,
) -> Result<Vec<u8>, WriteError> {
    let row = settings(profile)?;
    let text =
        std::str::from_utf8(pre.unwrap_or_default()).map_err(|_| WriteError::InvalidResource)?;
    if text.len() > 1024 * 1024 {
        return Err(WriteError::InvalidResource);
    }
    let live = text.parse::<DocumentMut>().map_err(|_| WriteError::InvalidResource)?;
    // The projection consumes an owner row; reconstruct only its recorded table name,
    // never the mutable provider row or the user's current models.default.
    let previous = written.filter(|s| !s.is_empty()).map(|name| {
        let mut doc = DocumentMut::new();
        let mut tables = toml_edit::Table::new();
        tables.insert(name, Item::Table(toml_edit::Table::new()));
        doc.insert("model", Item::Table(tables));
        json!({"config": doc.to_string()})
    });
    super::projection::grok_document(
        live,
        &row,
        previous.as_ref(),
        matches!(profile.config, GrokConfig::Official {}),
    )
    .map(|doc| doc.to_string().into_bytes())
    .map_err(|_| WriteError::InvalidResource)
}
#[cfg(target_os = "macos")]
fn written(profile: &GrokProfile) -> Result<String, WriteError> {
    let bytes = patch(None, profile, None)?;
    let text = std::str::from_utf8(&bytes).map_err(|_| WriteError::InvalidResource)?;
    let doc = text.parse::<DocumentMut>().map_err(|_| WriteError::InvalidResource)?;
    Ok(doc.get("models").and_then(|m| m.get("default")).and_then(Item::as_str).unwrap_or("").into())
}
#[cfg(target_os = "macos")]
fn previous_table(state: &super::live::commit::AppState) -> Result<Option<&str>, WriteError> {
    let Some(pointer) = &state.pointer else { return Ok(None) };
    let contract =
        state.mode.as_ref().and_then(|mode| mode.contract.as_ref()).ok_or(WriteError::Conflict)?;
    if contract.version != 1 || contract.key != pointer.id {
        return Err(WriteError::Conflict);
    }
    let table = contract
        .exclusive
        .get("grokWrittenTable")
        .and_then(Value::as_str)
        .ok_or(WriteError::Conflict)?;
    Ok((!table.is_empty()).then_some(table))
}
#[cfg(target_os = "macos")]
pub struct GrokFiles {
    pub guard: super::live::write_guard::AppWriteGuard,
}
#[cfg(target_os = "macos")]
impl GrokFiles {
    pub fn open(home: &std::path::Path) -> Result<Self, WriteError> {
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
        if unsafe { libc::mkdirat(home.as_raw_fd(), c".grok".as_ptr(), 0o700) } != 0
            && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(WriteError::Io);
        }
        // SAFETY: fixed no-follow child, fresh fd consumed exactly once below.
        let fd = unsafe {
            libc::openat(
                home.as_raw_fd(),
                c".grok".as_ptr(),
                libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
            )
        };
        if fd < 0 {
            return Err(WriteError::InvalidResource);
        }
        let dir = unsafe { std::fs::File::from_raw_fd(fd) };
        Ok(Self {
            guard: super::live::write_guard::lock_directory(
                vw_api_types::agent_apps::AppId::GrokBuild,
                dir,
            )?,
        })
    }

    pub fn preflight(
        &self,
        profile: &GrokProfile,
        state: &super::live::commit::AppState,
    ) -> Result<(), WriteError> {
        let config = self.guard.read(std::ffi::OsStr::new("config.toml"))?;
        patch(config.as_deref(), profile, previous_table(state)?)?;
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        tx: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        state: &super::live::commit::AppState,
        id: &str,
        profile: &GrokProfile,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        use super::live::{
            commit::*,
            stage::{FileChange, WholeFile},
            write_guard::*,
        };
        use std::ffi::OsStr;
        use vw_api_types::agent_apps::AppId;
        if tx.recover(&mut self.guard)?.is_some() {
            return Err(WriteError::Recovered);
        }
        if state.mode_tenant.as_ref().is_some_and(|t| t != tenant)
            || state.pointer.as_ref().is_some_and(|p| p.tenant != *tenant)
            || state.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(WriteError::Conflict);
        }
        let config = self.guard.read(OsStr::new("config.toml"))?;
        let next = patch(config.as_deref(), profile, previous_table(state)?)?;
        let config_patch = WholeFile::new(config.as_deref(), Some(next));
        tx.commit_checked(
            tenant,
            &mut self.guard,
            &[FileChange {
                name: OsStr::new("config.toml"),
                privacy: FilePrivacy::Private,
                patch: &config_patch,
            }],
            PendingTarget {
                pointer: Some(id.into()),
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: state.mode.as_ref().and_then(|m| m.proxy_route.clone()),
                    contract: Some(Contract {
                        version: 1,
                        key: id.into(),
                        exclusive: serde_json::Map::from_iter([(
                            "grokWrittenTable".into(),
                            json!(written(profile)?),
                        )]),
                        claude_login_operation: None,
                    }),
                }),
            },
            || {
                if unchanged() && tx.state(AppId::GrokBuild)? == *state {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}

/// Import only native provider configuration; MCP is not part of a supplier profile.
/// Official login can be requested manually, but must not be recreated on startup.
pub fn import(bytes: &[u8], startup: bool) -> Result<Option<GrokProfile>, WriteError> {
    super::import::reject_placeholders(bytes)?;
    let text = std::str::from_utf8(bytes).map_err(|_| WriteError::InvalidResource)?;
    let mut doc: DocumentMut = text.parse().map_err(|_| WriteError::InvalidResource)?;
    doc.remove("mcp_servers");
    let official = doc.get("model").is_none() && doc.get("models").is_none();
    if official && startup {
        return Ok(None);
    }
    let p = GrokProfile {
        name: if official { "Grok Official" } else { "default" }.into(),
        config: if official {
            GrokConfig::Official {}
        } else {
            GrokConfig::ThirdParty { toml: doc.to_string() }
        },
    };
    settings(&p)?;
    Ok(Some(p))
}
