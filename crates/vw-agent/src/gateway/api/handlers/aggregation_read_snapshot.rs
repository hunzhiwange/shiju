//! Executor-only snapshot and integrity for aggregation reads. Never persists state.
use crate::{gateway::settings_store::GatewaySettingsStore, global_database::GlobalDatabase};
use axum::http::StatusCode;
use serde::Serialize;
use serde_json::{Value, json};
use std::{path::Path, sync::LazyLock};
use vw_agent_apps::{aggregation::preview as native, providers::live::commit::AppState};
use vw_api_types::{
    agent_apps::{AppId, aggregation::*},
    id::{ProjectScope, TenantScope},
    model_catalog::SavedCodexCatalog,
};

#[derive(Serialize)]
pub(super) struct Snapshot {
    rows: Vec<(String, Value)>,
    catalogs: Vec<Option<SavedCodexCatalog>>,
    queue: Option<String>,
    route: Vec<Option<String>>,
    native: Vec<Option<Vec<u8>>>,
    direct_restorable: bool,
}
fn unavailable<T>(_: T) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn invalid<T>(_: T) -> StatusCode {
    StatusCode::UNPROCESSABLE_ENTITY
}

pub(super) fn load(
    db: &GlobalDatabase,
    project: &ProjectScope,
    host: &str,
    app: AppId,
    home: &Path,
    state: &AppState,
    candidate: Option<&str>,
) -> Result<Snapshot, StatusCode> {
    use crate::app_providers::AppProviderStore;
    use vw_agent_apps::providers::{
        codex_catalog::{form, identity},
        mode_state::ClientConfiguration,
    };
    let mut ids = state.stack.members.clone();
    for id in [
        state.mode.as_ref().and_then(|mode| mode.proxy_route.as_deref()),
        state.pointer.as_ref().map(|pointer| pointer.id.as_str()),
        candidate,
    ]
    .into_iter()
    .flatten()
    {
        if !ids.iter().any(|existing| existing == id) {
            ids.push(id.into());
        }
    }
    let mut rows = Vec::new();
    let mut catalogs = Vec::new();
    for id in ids {
        let row = match AppProviderStore::preview_record(db, project.tenant_scope(), app, &id) {
            Ok(row) => row,
            Err(error) if error == "application provider not found" => continue,
            Err(_) => return Err(StatusCode::SERVICE_UNAVAILABLE),
        };
        // Managed identities require a verified login-generation snapshot from
        // their account owner. Do not silently treat them as API-key members.
        if row.get("managedCodexLogin").is_some()
            || row.pointer("/meta/authBinding").is_some()
            || row.pointer("/meta/codexAccountId").is_some()
            || row.pointer("/meta/liveConfigManaged").is_some_and(|v| v == true)
        {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
        let catalog = if app == AppId::Codex && row["category"] != "official" {
            let profile =
                form::editable_profile(&id, &row).ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            let deployment = identity(host, &id, &profile).map_err(invalid)?;
            crate::model_catalog::ModelCatalogStore::read_existing(
                db,
                project.tenant_scope(),
                &deployment,
                row["modelCatalogRevision"].as_str(),
            )
            .map_err(unavailable)?
        } else {
            None
        };
        rows.push((id, row));
        catalogs.push(catalog);
    }
    let scope = vw_api_types::agent_apps_high_availability::FailoverQueueScope {
        project: project.clone(),
        target_host_id: host.into(),
        app_id: app,
    };
    let queue = if vw_agent_apps::routing::supports_app(app) {
        let queue_key = crate::gateway::agent_apps_high_availability_store::config_key(&scope)
            .map_err(invalid)?;
        GatewaySettingsStore::read_existing(
            db,
            project.tenant_scope(),
            "agent_apps.high_availability",
            &queue_key,
        )
        .map_err(unavailable)?
    } else {
        None
    };
    let mut route = Vec::new();
    for (namespace, key) in [
        ("agent_apps.routing.listener.v1", host.to_owned()),
        ("agent_apps.routing.egress.v1", host.to_owned()),
        (
            "agent_apps.routing.timeouts.v1",
            serde_json::to_string(&(host, project.project_id(), app)).map_err(unavailable)?,
        ),
    ] {
        route.push(
            GatewaySettingsStore::read_existing(db, project.tenant_scope(), namespace, &key)
                .map_err(unavailable)?,
        );
    }
    let direct_restorable = state
        .pointer
        .as_ref()
        .and_then(|pointer| rows.iter().find(|(id, _)| id == &pointer.id))
        .is_some_and(|(id, row)| {
            if row["category"] == "official" || row.get("managedClaudeLogin").is_some() {
                return false;
            }
            match app {
                AppId::Codex => form::editable_profile(id, row).is_some_and(|profile| {
                    !profile.preserve_login
                        && profile.api_key.as_ref().is_some_and(|key| !key.trim().is_empty())
                        && identity(host, id, &profile).is_ok()
                }),
                AppId::ClaudeDesktop => {
                    use vw_api_types::app_providers::claude_desktop::{
                        ClaudeDesktopConfig, ClaudeDesktopProfile,
                    };
                    serde_json::from_value::<ClaudeDesktopProfile>(row["claude_desktop"].clone())
                        .is_ok_and(|profile| {
                            matches!(&profile.config, ClaudeDesktopConfig::ThirdParty { .. })
                                && vw_agent_apps::providers::claude_desktop::settings(&profile)
                                    .is_ok()
                        })
                }
                AppId::Claude => row.get("settingsConfig").is_some_and(|settings| {
                    ClientConfiguration::saved(app, settings, false).is_ok()
                }),
                _ => false,
            }
        });
    let paths: Vec<&str> = match app {
        AppId::Claude => vec![".claude/settings.json", ".claude.json"],
        AppId::Codex => {
            vec![".codex/config.toml", ".codex/auth.json", ".codex/shiju-model-catalog.json"]
        }
        AppId::ClaudeDesktop => vec![
            "Library/Application Support/Claude/claude_desktop_config.json",
            "Library/Application Support/Claude-3p/claude_desktop_config.json",
            "Library/Application Support/Claude-3p/configLibrary/00000000-0000-4000-8000-000000157210.json",
            "Library/Application Support/Claude-3p/configLibrary/_meta.json",
        ],
        _ => return Err(StatusCode::UNPROCESSABLE_ENTITY),
    };
    let mut native = Vec::new();
    for path in paths {
        let bytes = read_native(home, path)?;
        if let Some(bytes) = &bytes {
            if path.ends_with(".toml") {
                let _: toml::Value =
                    std::str::from_utf8(bytes).map_err(invalid)?.parse().map_err(invalid)?;
            } else {
                // Reuse the native owner grammar, including comments/trailing commas.
                vw_agent_apps::providers::live::patch_json::JsonPatch::default()
                    .apply(Some(bytes))
                    .map_err(invalid)?;
            }
        }
        native.push(bytes);
    }
    Ok(Snapshot { rows, catalogs, queue, route, native, direct_restorable })
}

pub(super) fn project(
    app: AppId,
    tenant: &TenantScope,
    state: &AppState,
    snapshot: &Snapshot,
    request: Option<&AggregationPreviewRequest>,
) -> Result<AggregationPreview, StatusCode> {
    use vw_agent_apps::{
        aggregation::codex_catalog::CodexCatalogRow,
        providers::{codex_catalog::form, desktop_mapping},
    };
    let mut configs = Vec::new();
    let mut mappings = Vec::new();
    for (id, row) in &snapshot.rows {
        if app == AppId::Codex {
            if row["category"] == "official" {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            let profile =
                form::editable_profile(id, row).ok_or(StatusCode::UNPROCESSABLE_ENTITY)?;
            configs.push(profile.config.parse().map_err(invalid)?);
        }
        if app == AppId::ClaudeDesktop {
            mappings.push(desktop_mapping::form_view(row).map_err(invalid)?.mapping);
        }
    }
    let providers: Vec<_> = snapshot
        .rows
        .iter()
        .enumerate()
        .map(|(index, (id, row))| native::Provider {
            id,
            name: row["name"].as_str().unwrap_or(id),
            official_service: (row["category"] == "official")
                .then_some(vw_api_types::agent_apps_auth::AuthService::ClaudeOfficial),
            catalog: match app {
                AppId::Claude => native::Catalog::Claude(row),
                AppId::Codex => native::Catalog::Codex(CodexCatalogRow {
                    config: &configs[index],
                    catalog: snapshot.catalogs[index].as_ref(),
                }),
                AppId::ClaudeDesktop => native::Catalog::Desktop(&mappings[index]),
                _ => unreachable!(),
            },
        })
        .collect();
    let queue: crate::gateway::agent_apps_high_availability_store::FailoverQueueState = snapshot
        .queue
        .as_deref()
        .map(serde_json::from_str)
        .transpose()
        .map_err(invalid)?
        .unwrap_or_default();
    let output = native::preview(
        app,
        tenant,
        state,
        native::PreviewInput {
            candidate_default: request.and_then(|r| r.default_provider_id.as_deref()),
            providers: &providers,
            pending_tenant: None,
            failover_enabled: queue.enabled,
            direct_restorable: snapshot.direct_restorable,
            catalog_policy: if request
                .is_some_and(|r| r.catalog_policy == CatalogPolicy::UseManaged)
            {
                native::CatalogPolicy::UseManaged
            } else {
                native::CatalogPolicy::Preserve
            },
        },
    )
    .map_err(|error| match error {
        native::PreviewError::Forbidden => StatusCode::FORBIDDEN,
        native::PreviewError::RecoveryPending => StatusCode::CONFLICT,
        _ => StatusCode::UNPROCESSABLE_ENTITY,
    })?;
    Ok(AggregationPreview {
        revision: String::new(),
        preview_token: None,
        default_provider_id: output.default_provider_id,
        members: output
            .members
            .into_iter()
            .map(|m| AggregationMember {
                provider_id: m.provider_id,
                name: m.name,
                models: m
                    .models
                    .into_iter()
                    .map(|model| AggregationModel {
                        id: model.id,
                        label: model.label,
                        supports_1m: model.supports_1m,
                    })
                    .collect(),
            })
            .collect(),
        issues: output
            .issues
            .into_iter()
            .map(|issue| PreviewIssue {
                code: match issue.code {
                    native::IssueCode::ModelCollision => PreviewIssueCode::ModelCollision,
                    native::IssueCode::InvalidMember => PreviewIssueCode::InvalidMember,
                    native::IssueCode::MissingDefault => PreviewIssueCode::MissingDefault,
                    native::IssueCode::InvalidDefault => PreviewIssueCode::InvalidDefault,
                    native::IssueCode::FailoverEnabled => PreviewIssueCode::FailoverEnabled,
                    native::IssueCode::UnrestorableDirect => PreviewIssueCode::UnrestorableDirect,
                },
                blocking: issue.blocking,
                provider_id: issue.provider_id,
                model_id: issue.model_id,
            })
            .collect(),
        notices: output
            .notices
            .into_iter()
            .map(|notice| match notice {
                native::Notice::ModelReselect => AggregationNotice::ModelReselect,
                native::Notice::NewSession => AggregationNotice::NewSession,
                native::Notice::RestartDesktop => AggregationNotice::RestartDesktop,
                native::Notice::RestartEditor => AggregationNotice::RestartEditor,
                native::Notice::CodexRouteOwnsCatalog => AggregationNotice::CodexRouteOwnsCatalog,
            })
            .collect(),
    })
}

// Per-process signing material never persists or becomes a user configuration.
// Restart invalidates old previews; no key bytes or raw snapshots enter the wire.
static KEY: LazyLock<Result<ring::hmac::Key, StatusCode>> = LazyLock::new(|| {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(unavailable)?;
    Ok(ring::hmac::Key::new(ring::hmac::HMAC_SHA256, &bytes))
});
fn sign(bytes: &[u8]) -> Result<String, StatusCode> {
    let key = KEY.as_ref().map_err(|status| *status)?;
    Ok(hex::encode(ring::hmac::sign(key, bytes).as_ref()))
}
pub(super) fn revision(
    project: &ProjectScope,
    host: &str,
    app: AppId,
    principal: &str,
    state: &AppState,
    snapshot: &Snapshot,
) -> Result<String, StatusCode> {
    sign(
        &serde_json::to_vec(&(
            "aggregation_revision_v1",
            project,
            host,
            app,
            principal,
            state,
            snapshot,
        ))
        .map_err(unavailable)?,
    )
}
pub(super) fn token(
    principal: &str,
    project: &ProjectScope,
    host: &str,
    app: AppId,
    revision: &str,
    request: &AggregationPreviewRequest,
) -> Result<String, StatusCode> {
    use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
    let payload = serde_json::to_vec(&json!({"kind":"aggregation_preview_v1","principal":principal,"scope":project,
        "targetHostId":host,"app":app,"revision":revision,"defaultProviderId":request.default_provider_id,
        "catalogPolicy":request.catalog_policy,"expiresAt":chrono::Utc::now().timestamp()+120})).map_err(unavailable)?;
    Ok(format!("{}.{}", URL_SAFE_NO_PAD.encode(&payload), sign(&payload)?))
}

/// Read fixed native children via retained directory descriptors; reject symlinks,
/// special files and oversized input. Missing files stay absent, without mkdir.
fn read_native(home: &Path, relative: &str) -> Result<Option<Vec<u8>>, StatusCode> {
    use std::{
        fs::{File, OpenOptions},
        io::Read,
        os::{
            fd::{AsRawFd, FromRawFd},
            unix::fs::OpenOptionsExt,
        },
    };
    let mut directory = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_DIRECTORY | libc::O_NOFOLLOW | libc::O_CLOEXEC)
        .open(home)
        .map_err(unavailable)?;
    let components: Vec<_> = relative.split('/').collect();
    for (index, name) in components.iter().enumerate() {
        let last = index + 1 == components.len();
        let name = std::ffi::CString::new(*name).map_err(invalid)?;
        let flags = libc::O_RDONLY
            | libc::O_NOFOLLOW
            | libc::O_CLOEXEC
            | libc::O_NONBLOCK
            | if last { 0 } else { libc::O_DIRECTORY };
        // SAFETY: directory is retained, name is one fixed NUL-terminated child.
        let fd = unsafe { libc::openat(directory.as_raw_fd(), name.as_ptr(), flags) };
        if fd < 0 {
            return if std::io::Error::last_os_error().kind() == std::io::ErrorKind::NotFound {
                Ok(None)
            } else {
                Err(StatusCode::UNPROCESSABLE_ENTITY)
            };
        }
        // SAFETY: openat returned a new owned descriptor consumed once.
        let file = unsafe { File::from_raw_fd(fd) };
        if last {
            let meta = file.metadata().map_err(unavailable)?;
            if !meta.is_file() || meta.len() > 1024 * 1024 {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            let mut bytes = Vec::new();
            file.take(1024 * 1024 + 1).read_to_end(&mut bytes).map_err(unavailable)?;
            if bytes.len() > 1024 * 1024 {
                return Err(StatusCode::UNPROCESSABLE_ENTITY);
            }
            return Ok(Some(bytes));
        }
        directory = file;
    }
    Err(StatusCode::UNPROCESSABLE_ENTITY)
}
