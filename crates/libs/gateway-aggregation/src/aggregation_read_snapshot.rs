//! Executor-only snapshot and integrity for aggregation reads. Never persists state.
use app_settings_store::settings_store::GatewaySettingsStore;
use agent_database::GlobalDatabase;
use axum::http::StatusCode;
use serde::Serialize;
use serde_json::Value;
use std::{path::Path, sync::LazyLock};
use vw_agent_apps::providers::live::commit::AppState;
use vw_api_app_identity::AppId;
use vw_api_transport::id::ProjectScope;
use vw_api_model_catalog::SavedCodexCatalog;

#[derive(Serialize)]
pub struct Snapshot {
    rows: Vec<(String, Value)>,
    catalogs: Vec<Option<SavedCodexCatalog>>,
    queue: Option<String>,
    route: Vec<Option<String>>,
    native: Vec<Option<Vec<u8>>>,
}
fn unavailable<T>(_: T) -> StatusCode {
    StatusCode::SERVICE_UNAVAILABLE
}
fn invalid<T>(_: T) -> StatusCode {
    StatusCode::UNPROCESSABLE_ENTITY
}

pub fn load(
    db: &GlobalDatabase,
    project: &ProjectScope,
    host: &str,
    app: AppId,
    home: &Path,
    state: &AppState,
    candidate: Option<&str>,
) -> Result<Snapshot, StatusCode> {
    use app_provider_services::AppProviderStore;
    use vw_agent_apps::providers::{
        codex_catalog::{form, identity},
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
            model_catalog::ModelCatalogStore::read_existing(
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
    let scope = vw_api_app_failover::FailoverQueueScope {
        project: project.clone(),
        target_host_id: host.into(),
        app_id: app,
    };
    let queue = if vw_agent_apps::routing::supports_app(app) {
        let queue_key = app_settings_store::agent_apps_high_availability_store::config_key(&scope)
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
                vw_agent_apps::providers::live::patch_toml::TomlPatch::default()
                    .apply(Some(bytes))
                    .map_err(invalid)?;
            } else {
                // Reuse the native owner grammar, including comments/trailing commas.
                vw_agent_apps::providers::live::patch_json::JsonPatch::default()
                    .apply(Some(bytes))
                    .map_err(invalid)?;
            }
        }
        native.push(bytes);
    }
    Ok(Snapshot { rows, catalogs, queue, route, native })
}

static KEY: LazyLock<Result<ring::hmac::Key, StatusCode>> = LazyLock::new(|| {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(unavailable)?;
    Ok(ring::hmac::Key::new(ring::hmac::HMAC_SHA256, &bytes))
});
pub fn sign(bytes: &[u8]) -> Result<String, StatusCode> {
    let key = KEY.as_ref().map_err(|status| *status)?;
    Ok(hex::encode(ring::hmac::sign(key, bytes).as_ref()))
}
pub fn revision(
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
