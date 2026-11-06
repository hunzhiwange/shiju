//! Fixed macOS Desktop direct profiles; all four files share one durable intent.
use super::live::{
    patch_json::{JsonPatch, object_value},
    write_guard::WriteError,
};
use serde_json::{Value, json};
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
fn patch(
    pre: Option<&[u8]>,
    key: &str,
    profile: &ClaudeDesktopProfile,
) -> Result<Vec<u8>, WriteError> {
    settings(profile)?;
    let official = matches!(profile.config, ClaudeDesktopConfig::Official {});
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
            if let ClaudeDesktopConfig::ThirdParty { base_url, auth_token } = &profile.config {
                for (key, value) in [
                    ("inferenceProvider", json!("gateway")),
                    ("inferenceGatewayBaseUrl", json!(base_url.trim())),
                    ("inferenceGatewayApiKey", json!(auth_token.trim())),
                    ("inferenceGatewayAuthScheme", json!("bearer")),
                ] {
                    patch.set.push((vec![key.into()], value));
                }
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
        use std::{
            ffi::CString,
            fs::{File, OpenOptions},
            os::{
                fd::{AsRawFd, FromRawFd},
                unix::fs::{MetadataExt, OpenOptionsExt},
            },
        };
        fn child(parent: &File, name: &str) -> Result<File, WriteError> {
            let name = CString::new(name).map_err(|_| WriteError::InvalidResource)?;
            // SAFETY: fixed single component under the authorized retained HOME.
            if unsafe { libc::mkdirat(parent.as_raw_fd(), name.as_ptr(), 0o700) } != 0
                && std::io::Error::last_os_error().kind() != std::io::ErrorKind::AlreadyExists
            {
                return Err(WriteError::Io);
            }
            let fd = unsafe {
                libc::openat(
                    parent.as_raw_fd(),
                    name.as_ptr(),
                    libc::O_RDONLY | libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC,
                )
            };
            if fd < 0 {
                return Err(WriteError::InvalidResource);
            }
            // SAFETY: newly owned fd from openat.
            let file = unsafe { File::from_raw_fd(fd) };
            let m = file.metadata().map_err(|_| WriteError::Io)?;
            if m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
                return Err(WriteError::InvalidResource);
            }
            parent.sync_all().map_err(|_| WriteError::Io)?;
            Ok(file)
        }
        let home = OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
            .open(home)
            .map_err(|_| WriteError::InvalidResource)?;
        let library = child(&home, "Library")?;
        let support = child(&library, "Application Support")?;
        let normal = child(&support, "Claude")?;
        let third = child(&support, "Claude-3p")?;
        let profiles = child(&third, "configLibrary")?;
        let targets = [
            ("normal", 0, "claude_desktop_config.json".to_string()),
            ("third", 1, "claude_desktop_config.json".to_string()),
            ("profile", 2, format!("{PROFILE_ID}.json")),
            ("meta", 2, "_meta.json".to_string()),
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
        for key in KEYS {
            let before = self.guard.read(std::ffi::OsStr::new(key))?;
            patch(before.as_deref(), key, profile)?;
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
        use super::live::{
            commit::*,
            stage::{FileChange, WholeFile},
            write_guard::FilePrivacy,
        };
        use vw_api_types::agent_apps::AppId;
        if tx.recover(&mut self.guard)?.is_some() {
            return Err(WriteError::Recovered);
        }
        if state.pointer.as_ref().is_some_and(|p| p.tenant != *tenant)
            || state.mode_tenant.as_ref().is_some_and(|t| t != tenant)
            || state.mode.as_ref().is_some_and(|m| m.attached || m.mode != Mode::Direct)
        {
            return Err(WriteError::Conflict);
        }
        let mut plans = Vec::new();
        for key in KEYS {
            let before = self.guard.read(std::ffi::OsStr::new(key))?;
            if key == "profile"
                && before.is_none()
                && matches!(profile.config, ClaudeDesktopConfig::Official {})
            {
                continue;
            }
            plans.push((
                key,
                WholeFile::new(before.as_deref(), Some(patch(before.as_deref(), key, profile)?)),
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
                        exclusive: serde_json::Map::new(),
                        claude_login_operation: None,
                    }),
                }),
            },
            || {
                if unchanged() && tx.state(AppId::ClaudeDesktop)? == *state {
                    Ok(())
                } else {
                    Err(WriteError::Conflict)
                }
            },
        )
    }
}
