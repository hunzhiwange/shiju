//! Fixed macOS Desktop profiles; all four files share one durable intent.
use super::live::{
    patch_json::{JsonPatch, object_value},
    write_guard::WriteError,
};
use serde_json::{Map, Value, json};
use vw_api_types::app_providers::claude_desktop::*;

pub const PROFILE_ID: &str = "00000000-0000-4000-8000-000000157210";
const PROFILE_NAME: &str = "CC Switch";
const FLOOR: &[&str] = &[
    "inferenceProvider",
    "inferenceGatewayBaseUrl",
    "inferenceGatewayApiKey",
    "inferenceGatewayAuthScheme",
    "inferenceModels",
    "modelDiscoveryEnabled",
];
const KEYS: [&str; 4] = ["normal", "third", "profile", "meta"];
const CONFIG_FILE: &str = "claude_desktop_config.json";
const META_FILE: &str = "_meta.json";

#[cfg(target_os = "macos")]
pub mod publication;

#[cfg(target_os = "macos")]
use std::{fs::File, path::Path};

pub fn settings(profile: &ClaudeDesktopProfile) -> Result<Value, WriteError> {
    if profile.name.trim().is_empty()
        || profile.name.len() > 128
        || profile.name.chars().any(char::is_control)
    {
        return Err(WriteError::InvalidResource);
    }
    match &profile.config {
        ClaudeDesktopConfig::Official {} => Ok(json!({"env":{}})),
        ClaudeDesktopConfig::ThirdParty { base_url, auth_token } => {
            super::validation::endpoint(base_url.trim())
                .map_err(|_| WriteError::InvalidResource)?;
            if auth_token.trim().is_empty()
                || auth_token.len() > 16384
                || auth_token.chars().any(char::is_control)
            {
                return Err(WriteError::InvalidResource);
            }
            Ok(
                json!({"env":{"ANTHROPIC_BASE_URL":base_url.trim(),"ANTHROPIC_AUTH_TOKEN":auth_token.trim()}}),
            )
        }
    }
}
/// Desktop's generic live import cannot represent direct profiles losslessly.
pub fn generic_import() -> Result<Value, &'static str> {
    Err("claude_desktop.import_unsupported")
}
fn profile_fields(config: &ClaudeDesktopConfig, models: Option<&Value>) -> Map<String, Value> {
    let ClaudeDesktopConfig::ThirdParty { base_url, auth_token } = config else {
        return Map::new();
    };
    let mut fields = Map::from_iter([
        ("inferenceProvider".into(), json!("gateway")),
        ("inferenceGatewayBaseUrl".into(), json!(base_url.trim())),
        ("inferenceGatewayApiKey".into(), json!(auth_token.trim())),
        ("inferenceGatewayAuthScheme".into(), json!("bearer")),
    ]);
    if let Some(models) = models {
        fields.insert("inferenceModels".into(), models.clone());
        fields.insert("modelDiscoveryEnabled".into(), json!(true));
    }
    fields
}

fn patch(
    pre: Option<&[u8]>,
    key: &str,
    fields: &Map<String, Value>,
) -> Result<Vec<u8>, WriteError> {
    let official = fields.is_empty();
    let mut patch = JsonPatch::default();
    let value = object_value(pre.unwrap_or(b"{}")).map_err(|_| WriteError::InvalidResource)?;
    match key {
        "normal" | "third" => {
            patch
                .set
                .push((vec!["deploymentMode".into()], json!(if official { "1p" } else { "3p" })));
            if key == "third" && official {
                for key in [
                    "disableDeploymentModeChooser",
                    "inferenceGatewayApiKey",
                    "inferenceGatewayAuthScheme",
                    "inferenceGatewayBaseUrl",
                    "inferenceProvider",
                ] {
                    patch.remove.push(vec!["enterpriseConfig".into(), key.into()]);
                }
            }
        }
        "profile" => {
            patch.remove = FLOOR.iter().map(|s| vec![(*s).into()]).collect();
            if !official {
                patch.set =
                    fields.iter().map(|(key, value)| (vec![key.clone()], value.clone())).collect();
                patch.seed = vec![
                    (vec!["disableDeploymentModeChooser".into()], json!(true)),
                    (vec!["coworkEgressAllowedHosts".into()], json!(["*"])),
                ];
            }
        }
        "meta" => {
            let mut entries = match value.get("entries") {
                None => Vec::new(),
                Some(Value::Array(a)) => a.clone(),
                _ => return Err(WriteError::InvalidResource),
            };
            if official {
                entries.retain(|e| e.get("id").and_then(Value::as_str) != Some(PROFILE_ID));
                if value.get("appliedId").and_then(Value::as_str) == Some(PROFILE_ID) {
                    if let Some(id) =
                        entries.iter().find_map(|e| e.get("id").and_then(Value::as_str))
                    {
                        patch.set.push((vec!["appliedId".into()], json!(id)));
                    } else {
                        patch.remove.push(vec!["appliedId".into()]);
                    }
                }
            } else {
                if let Some(entry) = entries
                    .iter_mut()
                    .find(|e| e.get("id").and_then(Value::as_str) == Some(PROFILE_ID))
                {
                    entry["name"] = json!(PROFILE_NAME);
                } else {
                    entries.push(json!({"id":PROFILE_ID,"name":PROFILE_NAME}));
                }
                patch.set.push((vec!["appliedId".into()], json!(PROFILE_ID)));
            }
            patch.set.push((vec!["entries".into()], json!(entries)));
        }
        _ => return Err(WriteError::InvalidResource),
    }
    let result = patch.apply(pre).map_err(|_| WriteError::InvalidResource)?;
    if key == "third"
        && official
        && object_value(&result)
            .map_err(|_| WriteError::InvalidResource)?
            .get("enterpriseConfig")
            .and_then(Value::as_object)
            .is_some_and(|o| o.is_empty())
    {
        return JsonPatch { remove: vec![vec!["enterpriseConfig".into()]], ..JsonPatch::default() }
            .apply(Some(&result))
            .map_err(|_| WriteError::InvalidResource);
    }
    Ok(result)
}
#[cfg(target_os = "macos")]
pub struct ClaudeDesktopFiles {
    pub guard: super::live::write_guard::AppWriteGuard,
}
#[cfg(target_os = "macos")]
impl ClaudeDesktopFiles {
    pub fn open(home: &std::path::Path) -> Result<Self, WriteError> {
        use super::live::write_guard::{DirectoryTarget, lock_directories};
        let child =
            |parent: &File, name: &str| open_child(parent, name, true)?.ok_or(WriteError::Io);
        let home = open_home(home)?;
        let library = child(&home, "Library")?;
        let support = child(&library, "Application Support")?;
        let normal = child(&support, "Claude")?;
        let third = child(&support, "Claude-3p")?;
        let profiles = child(&third, "configLibrary")?;
        let targets = [
            ("normal", 0, CONFIG_FILE.to_string()),
            ("third", 1, CONFIG_FILE.to_string()),
            ("profile", 2, format!("{PROFILE_ID}.json")),
            ("meta", 2, META_FILE.to_string()),
        ]
        .into_iter()
        .map(|(key, directory, name)| DirectoryTarget {
            key: key.into(),
            directory,
            name: name.into(),
        })
        .collect();
        Ok(Self {
            guard: lock_directories(
                vw_api_types::agent_apps::AppId::ClaudeDesktop,
                normal,
                vec![third, profiles],
                targets,
            )?,
        })
    }
    pub fn preflight(
        &self,
        profile: &ClaudeDesktopProfile,
        _: &super::live::commit::AppState,
    ) -> Result<(), WriteError> {
        settings(profile)?;
        let fields = profile_fields(&profile.config, None);
        for key in KEYS {
            let before = self.guard.read(std::ffi::OsStr::new(key))?;
            patch(before.as_deref(), key, &fields)?;
        }
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub fn apply(
        &mut self,
        tx: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        state: &super::live::commit::AppState,
        id: &str,
        profile: &ClaudeDesktopProfile,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        use super::{live::commit::*, mode_state};
        mode_state::observe(state, tenant).map_err(|_| WriteError::Conflict)?;
        if tx.recover_owned(tenant, &mut self.guard)?.is_some() {
            return Err(WriteError::Recovered);
        }
        if state.mode.as_ref().is_some_and(|mode| mode.attached || mode.mode != Mode::Direct) {
            return Err(WriteError::Conflict);
        }
        settings(profile)?;
        self.commit_profile(
            tx,
            tenant,
            state,
            &profile_fields(&profile.config, None),
            PendingTarget {
                pointer: Some(id.into()),
                mode: Some(ModeState {
                    mode: Mode::Direct,
                    attached: false,
                    proxy_route: state.mode.as_ref().and_then(|m| m.proxy_route.clone()),
                    contract: Some(Contract {
                        version: 1,
                        key: id.into(),
                        exclusive: Map::new(),
                        claude_login_operation: None,
                    }),
                }),
                ..Default::default()
            },
            unchanged,
        )
    }

    fn commit_profile(
        &mut self,
        tx: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        state: &super::live::commit::AppState,
        fields: &Map<String, Value>,
        target: super::live::commit::PendingTarget,
        unchanged: impl Fn() -> bool,
    ) -> Result<(), WriteError> {
        self.commit_profile_record(tx, tenant, state, fields, target, unchanged, None)
    }

    #[allow(clippy::too_many_arguments)]
    fn commit_profile_record(
        &mut self,
        tx: &super::live::commit::ProviderTransactions,
        tenant: &vw_api_types::id::TenantScope,
        state: &super::live::commit::AppState,
        fields: &Map<String, Value>,
        target: super::live::commit::PendingTarget,
        unchanged: impl Fn() -> bool,
        record: Option<(
            &super::live::record_publication::ProviderRecordChange,
            &dyn super::live::record_publication::ProviderRecordParticipant,
        )>,
    ) -> Result<(), WriteError> {
        use super::live::{
            stage::{FileChange, WholeFile},
            write_guard::FilePrivacy,
        };
        use vw_api_types::agent_apps::AppId;
        let mut plans = Vec::new();
        for key in KEYS {
            let before = self.guard.read(std::ffi::OsStr::new(key))?;
            if key == "profile" && before.is_none() && fields.is_empty() {
                continue;
            }
            plans.push((
                key,
                WholeFile::new(before.as_deref(), Some(patch(before.as_deref(), key, fields)?)),
            ));
        }
        let changes: Vec<_> = plans
            .iter()
            .map(|(key, patch)| FileChange {
                name: std::ffi::OsStr::new(key),
                privacy: if *key == "profile" { FilePrivacy::Private } else { FilePrivacy::Shared },
                patch,
            })
            .collect();
        let validate = || {
            if unchanged() && tx.state(AppId::ClaudeDesktop)? == *state {
                Ok(())
            } else {
                Err(WriteError::Conflict)
            }
        };
        if let Some((change, participant)) = record {
            let project = target.project_scope.as_ref().ok_or(WriteError::Conflict)?;
            return tx.commit_project_record(
                project,
                &mut self.guard,
                state,
                &changes,
                target.clone(),
                change,
                participant,
                validate,
            );
        }
        if let Some(project) = &target.project_scope {
            if project.tenant_scope() != tenant {
                return Err(WriteError::Conflict);
            }
            tx.commit_project_checked(
                project,
                &mut self.guard,
                state,
                false,
                &changes,
                target.clone(),
                validate,
            )
        } else {
            tx.commit_checked(tenant, &mut self.guard, &changes, target, validate)
        }
    }
}

#[cfg(target_os = "macos")]
fn open_home(home: &Path) -> Result<File, WriteError> {
    use std::{fs::OpenOptions, os::unix::fs::OpenOptionsExt};
    if !home.is_absolute() {
        return Err(WriteError::InvalidResource);
    }
    OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(home)
        .map_err(|_| WriteError::InvalidResource)
}

#[cfg(target_os = "macos")]
fn open_child(parent: &File, name: &str, create: bool) -> Result<Option<File>, WriteError> {
    use std::{
        ffi::CString,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::MetadataExt,
        },
    };
    let name = CString::new(name).map_err(|_| WriteError::InvalidResource)?;
    // SAFETY: fixed single component and a retained, authorized directory.
    if create
        && unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0
        && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
    {
        return Err(WriteError::Io);
    }
    // SAFETY: same retained directory and fixed component.
    let fd = unsafe {
        libc::openat(
            parent.as_raw_fd(),
            name.as_ptr(),
            libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
        )
    };
    if fd < 0 {
        if !create && std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
            return Ok(None);
        }
        return Err(WriteError::InvalidResource);
    }
    // SAFETY: newly owned descriptor returned by openat.
    let file = unsafe { File::from_raw_fd(fd) };
    let metadata = file.metadata().map_err(|_| WriteError::Io)?;
    if metadata.uid() != unsafe { libc::geteuid() } || metadata.mode() & 0o022 != 0 {
        return Err(WriteError::InvalidResource);
    }
    if create {
        parent.sync_all().map_err(|_| WriteError::Io)?;
    }
    Ok(Some(file))
}

#[cfg(target_os = "macos")]
fn lock_profiles(
    home: &Path,
) -> Result<Option<super::live::write_guard::AppWriteGuard>, WriteError> {
    use super::live::write_guard::{DirectoryTarget, lock_directories};
    let mut directory = open_home(home)?;
    for name in ["Library", "Application Support", "Claude-3p", "configLibrary"] {
        let Some(child) = open_child(&directory, name, false)? else { return Ok(None) };
        directory = child;
    }
    let targets = [("profile", format!("{PROFILE_ID}.json")), ("meta", META_FILE.to_string())]
        .into_iter()
        .map(|(key, name)| DirectoryTarget { key: key.into(), directory: 0, name: name.into() })
        .collect();
    lock_directories(vw_api_types::agent_apps::AppId::ClaudeDesktop, directory, vec![], targets)
        .map(Some)
}

#[cfg(all(test, target_os = "macos"))]
#[path = "claude_desktop/publish_tests.rs"]
mod publish_tests;

#[cfg(all(test, target_os = "macos"))]
#[path = "claude_desktop/project_tests.rs"]
mod project_tests;
