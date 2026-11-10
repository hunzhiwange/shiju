//! Claude Desktop's native aggregation projection. The executor authorizes the
//! host, project, application and saved inputs before calling this module.
use super::{ClaudeDesktopFiles, PROFILE_ID, lock_profiles, profile_fields, settings};
use crate::providers::{
    desktop_mapping::catalog::DesktopModelCatalog,
    live::{
        commit::{AppState, Mode, ModeState, PendingTarget, ProviderTransactions},
        patch_json::object_value,
        write_guard::{AppWriteGuard, WriteError},
    },
    mode_apply::ClientEndpoint,
    mode_state::{self, ClientConfiguration, StackState},
};
use serde_json::{Value, json};
use std::{ffi::OsStr, path::Path};
use vw_api_types::{
    agent_apps::{AppDesktopObservation, AppId},
    app_providers::claude_desktop::{ClaudeDesktopConfig, ClaudeDesktopProfile},
    id::TenantScope,
};

const APP: AppId = AppId::ClaudeDesktop;
const MAX_INFERENCE_MODELS: usize = 200;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopInstallation {
    Installed,
    NotInstalled,
    Unsupported,
}

impl From<&AppDesktopObservation> for DesktopInstallation {
    fn from(observation: &AppDesktopObservation) -> Self {
        match (observation.supported, observation.app_bundles.as_deref()) {
            (true, Some([])) => Self::NotInstalled,
            (true, Some(_)) => Self::Installed,
            _ => Self::Unsupported,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopMode {
    Official,
    DirectGateway,
    Routing,
    Aggregation,
}

/// Native observation. The API layer adds the scope-bound revision and notices.
#[derive(Debug, PartialEq, Eq)]
pub struct DesktopStatus {
    pub installation: DesktopInstallation,
    pub configured: bool,
    pub mode: DesktopMode,
    pub applied_provider_id: Option<String>,
    pub recovery_pending: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DesktopPublicationError {
    UnsupportedTarget,
    Forbidden,
    RevisionConflict,
    RecoveryPending,
    InvalidConfiguration,
    TargetFailed,
}

impl From<WriteError> for DesktopPublicationError {
    fn from(error: WriteError) -> Self {
        match error {
            WriteError::Recovered => Self::RecoveryPending,
            WriteError::Conflict => Self::RevisionConflict,
            WriteError::InvalidResource => Self::InvalidConfiguration,
            WriteError::Io => Self::TargetFailed,
        }
    }
}

pub enum DesktopTarget<'a> {
    Route {
        /// The application-specific loopback URL and Desktop gateway token
        /// supplied by the authorized route executor.
        gateway: &'a ClaudeDesktopProfile,
        catalog: &'a DesktopModelCatalog,
        stack: Option<StackState>,
    },
    /// The authorized saved direct profile, or no profile if no direct pointer exists.
    Restore(Option<&'a ClaudeDesktopProfile>),
}

/// Publish client files and state using the existing Desktop transaction.
/// `unchanged` rechecks the executor's scoped revision and saved inputs, including
/// any saved profile supplied for restoration. It is checked again under the application guard before publication.
pub fn run(
    home: &Path,
    installation: DesktopInstallation,
    tx: &ProviderTransactions,
    tenant: &TenantScope,
    expected: &AppState,
    target: DesktopTarget<'_>,
    unchanged: impl Fn() -> bool,
) -> Result<(), DesktopPublicationError> {
    if installation != DesktopInstallation::Installed {
        return Err(DesktopPublicationError::UnsupportedTarget);
    }
    let (current, pending) = observe(tx, tenant)?;
    if pending {
        return Err(DesktopPublicationError::RecoveryPending);
    }
    if current != *expected || !unchanged() {
        return Err(DesktopPublicationError::RevisionConflict);
    }

    let official =
        ClaudeDesktopProfile { name: "Official".into(), config: ClaudeDesktopConfig::Official {} };
    let (profile, models, mode, proxy_route, stack) = match target {
        DesktopTarget::Route { gateway, catalog, stack } => {
            let ClaudeDesktopConfig::ThirdParty { base_url, .. } = &gateway.config else {
                return Err(DesktopPublicationError::InvalidConfiguration);
            };
            ClientEndpoint::new(base_url)
                .map_err(|_| DesktopPublicationError::InvalidConfiguration)?;
            let models = catalog.models();
            let default = models.first().ok_or(DesktopPublicationError::InvalidConfiguration)?;
            match stack.as_ref() {
                Some(stack)
                    if stack.enabled
                        && models
                            .iter()
                            .all(|model| stack.members.contains(&model.provider_id)) => {}
                None if !expected.stack.enabled
                    && models.iter().all(|model| model.provider_id == default.provider_id) => {}
                _ => return Err(DesktopPublicationError::InvalidConfiguration),
            }
            let projection = models
                .iter()
                .take(MAX_INFERENCE_MODELS)
                .map(|model| {
                    let mut entry = json!({"name": model.id, "labelOverride": model.label});
                    if model.supports_1m {
                        entry["supports1m"] = json!(true);
                        entry["prefer1m"] = json!(true);
                    }
                    entry
                })
                .collect();
            (
                gateway,
                Some(Value::Array(projection)),
                Mode::Proxy,
                Some(default.provider_id.clone()),
                stack,
            )
        }
        DesktopTarget::Restore(saved) => {
            if saved.is_some() != expected.pointer.is_some() {
                return Err(DesktopPublicationError::RevisionConflict);
            }
            let stack = expected
                .stack
                .enabled
                .then(|| StackState { enabled: false, ..expected.stack.clone() });
            (
                saved.unwrap_or(&official),
                None,
                Mode::Direct,
                expected.mode.as_ref().and_then(|mode| mode.proxy_route.clone()),
                stack,
            )
        }
    };
    settings(profile)?;
    let fields = profile_fields(&profile.config, models.as_ref());
    let configuration =
        ClientConfiguration::ClaudeDesktop { profile: Value::Object(fields.clone()) };
    let next = PendingTarget {
        mode: Some(ModeState {
            mode,
            attached: mode == Mode::Proxy,
            proxy_route,
            contract: Some(configuration.contract()),
        }),
        stack,
        ..Default::default()
    };
    let mut files =
        ClaudeDesktopFiles::open(home).map_err(|_| DesktopPublicationError::TargetFailed)?;
    files.commit_profile(tx, tenant, expected, &fields, next, unchanged)?;
    Ok(())
}

/// Read client-owned files and settled state without creating paths or recovering pending writes.
pub fn get_status(
    home: &Path,
    installation: DesktopInstallation,
    tx: Option<&ProviderTransactions>,
    tenant: &TenantScope,
) -> Result<DesktopStatus, DesktopPublicationError> {
    let guard =
        if installation == DesktopInstallation::Unsupported { None } else { lock_profiles(home)? };
    let (state, recovery_pending) = match tx {
        Some(tx) => observe(tx, tenant)?,
        None => (AppState::default(), false),
    };
    let profile = read_object(guard.as_ref(), "profile")?;
    let meta = read_object(guard.as_ref(), "meta")?;
    let entries = meta
        .get("entries")
        .map(|entries| entries.as_array().ok_or(DesktopPublicationError::InvalidConfiguration))
        .transpose()?;
    let configured = profile.get("inferenceProvider").is_some()
        || entries.is_some_and(|entries| entries.iter().any(|entry| entry["id"] == PROFILE_ID));
    let attached = state.mode.as_ref().filter(|mode| mode.mode == Mode::Proxy && mode.attached);
    let (mode, applied_provider_id) = if let Some(route) = attached {
        let mode =
            if state.stack.enabled { DesktopMode::Aggregation } else { DesktopMode::Routing };
        (mode, route.proxy_route.clone())
    } else {
        let mode = if profile["inferenceProvider"] == "gateway" && meta["appliedId"] == PROFILE_ID {
            DesktopMode::DirectGateway
        } else {
            DesktopMode::Official
        };
        (mode, state.pointer.map(|provider| provider.id))
    };
    Ok(DesktopStatus { installation, configured, mode, applied_provider_id, recovery_pending })
}

fn observe(
    tx: &ProviderTransactions,
    tenant: &TenantScope,
) -> Result<(AppState, bool), DesktopPublicationError> {
    let state = tx.state(APP)?;
    mode_state::observe(&state, tenant).map_err(|_| DesktopPublicationError::Forbidden)?;
    let pending = tx.pending(APP)?;
    if pending.as_ref().is_some_and(|intent| intent.tenant != *tenant) {
        return Err(DesktopPublicationError::Forbidden);
    }
    Ok((state, pending.is_some()))
}

fn read_object(guard: Option<&AppWriteGuard>, key: &str) -> Result<Value, DesktopPublicationError> {
    let bytes = match guard {
        Some(guard) => guard.read(OsStr::new(key))?,
        None => None,
    };
    match bytes {
        Some(bytes) => {
            object_value(&bytes).map_err(|_| DesktopPublicationError::InvalidConfiguration)
        }
        None => Ok(json!({})),
    }
}
