//! Narrow host bindings for authorization, scan scheduling and source discovery.
use std::{collections::HashMap, path::{Path, PathBuf}, sync::Mutex};
use axum::http::{HeaderMap, StatusCode};
use vw_api_transport::{ProjectScope, TenantScope};
use app_usage::{scan_jobs::ScanOwner, scan_schedule::ScanSchedule};
use vw_api_external_session::{ExternalSessionIdentity, SessionProvider};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Intent {
    pub project_id: vw_api_transport::ProjectId,
    pub target_host: String,
    pub active: Option<bool>,
    pub paused: Option<bool>,
}

pub struct Source {
    pub instance_id: String,
    pub provider: SessionProvider,
    pub root: PathBuf,
}
pub struct File {
    pub path: PathBuf,
    pub identity: ExternalSessionIdentity,
}

#[derive(Clone, Copy)]
pub struct Host {
    pub database: fn() -> anyhow::Result<agent_database::GlobalDatabase>,
    pub authorize_read: fn(Option<&Path>, &HeaderMap, &ProjectScope, &str) -> Result<(), StatusCode>,
    pub authorize_events: fn(Option<&Path>, &HeaderMap, &ProjectScope, &str) -> Result<String, StatusCode>,
    pub authorize_maintenance: fn(Option<&Path>, &HeaderMap, &ProjectScope, &str) -> Result<(String, PathBuf, Vec<Source>, PathBuf), StatusCode>,
    pub authorized_owner: fn(Option<&Path>, TenantScope, &HeaderMap, &Intent) -> Result<ScanOwner, StatusCode>,
    pub apps: fn() -> Vec<&'static str>,
    pub schedules: fn() -> &'static Mutex<HashMap<ScanOwner, ScanSchedule>>,
    pub discover: fn(&ProjectScope, &str, SessionProvider, &[(PathBuf, Source)], &str) -> anyhow::Result<Vec<File>>,
}
