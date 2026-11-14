//! Read-only migration from administrator-selected executor sources. No client paths.
use super::super::agent_apps::{authenticate_data_migration, authorize_data_migration};
use crate::{app_providers::AppProviderStore, global_database::GlobalDatabase};
use axum::{
    Extension, Json, Router,
    extract::Request,
    http::{StatusCode, header},
    routing::post,
};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use vw_agent_apps::data_management::{
    preview::Difference,
    providers::{self, ProviderPreview, ProviderRow},
    source::{SourceFormat, SourceScope, SourceSnapshot},
};
use vw_api_types::{
    agent_apps::{AppId, data_management::*},
    id::TenantScope,
};

const TTL: Duration = Duration::from_secs(300);
// Source staging itself enforces 256 MiB; bound both admission and retained data.
const MAX_SOURCES: usize = 2;
const MAX_PREVIEWS: usize = 16;
#[derive(Clone)]
struct State {
    policy: Option<PathBuf>,
    cache: Arc<Mutex<Cache>>,
}
#[derive(Default)]
struct Cache {
    sources: HashMap<String, Staged>,
    previews: HashMap<String, HeldPreview>,
}
struct Staged {
    created: Instant,
    target: DataManagementTarget,
    source_id: String,
    snapshot: SourceSnapshot,
    summary: DataSourceSummary,
}
struct HeldPreview {
    created: Instant,
    // Keep normalized values and their handles together for a later executor;
    // issuance does not grant write permission and no execute route is added.
    _target: DataManagementTarget,
    _transfer: String,
    _preview: ProviderPreview,
    _response: DataImportPreview,
}
impl Cache {
    fn expire(&mut self) {
        self.sources.retain(|_, s| s.created.elapsed() < TTL);
        self.previews
            .retain(|_, p| p.created.elapsed() < TTL && self.sources.contains_key(&p._transfer));
    }
}
pub(crate) fn router() -> Router {
    router_with_policy(std::env::var_os("SHIJU_AGENT_APPS_ACCESS_FILE").map(PathBuf::from))
}
pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/agent-apps/data/sources/identify", post(identify))
        .route("/agent-apps/data/import/preview", post(preview))
        .layer(Extension(State { policy, cache: Arc::new(Mutex::new(Cache::default())) }))
        .layer(axum::middleware::map_response(async |mut r: axum::response::Response| {
            r.headers_mut()
                .insert(header::CACHE_CONTROL, header::HeaderValue::from_static("no-store"));
            r
        }))
}
// Request is deliberately last: authentication runs before body decoding. Errors
// contain only status codes, never JSON rejection details or source contents.
async fn identify(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    request: Request,
) -> Result<Json<DataSourceSummary>, StatusCode> {
    authenticate_data_migration(state.policy.as_deref(), request.headers(), &tenant)?;
    let headers = request.headers().clone();
    let bytes = axum::body::to_bytes(request.into_body(), 16 * 1024)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
    let input: IdentifyDataSourceRequest =
        serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        if input.target.scope.tenant_scope() != &tenant {
            return Err(StatusCode::FORBIDDEN);
        }
        let (root, selected) = authorize_data_migration(
            state.policy.as_deref(),
            &headers,
            &input.target,
            Some(&input.transfer_id.0),
        )?
        .ok_or(StatusCode::FORBIDDEN)?;
        let scope = source_scope(&input.target)?;
        let mut cache = state.cache.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        cache.expire();
        if cache.sources.len() >= MAX_SOURCES {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        // Hold admission lock through parsing, so concurrent requests cannot
        // allocate an unbounded number of snapshots before admission.
        let snapshot =
            vw_agent_apps::data_management::source::read_selected_source(&scope, &root, &selected)
                .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let source = snapshot.summary(&scope).map_err(|_| StatusCode::FORBIDDEN)?;
        let format = match source.format {
            SourceFormat::Sqlite => DataArchiveFormat::Sqlite,
            SourceFormat::Json => DataArchiveFormat::Json,
            SourceFormat::Sql => DataArchiveFormat::Sql,
        };
        use sha2::{Digest, Sha256};
        let mut digest = Sha256::new();
        digest.update(source.main_sha256);
        if let Some(wal) = source.wal_sha256 {
            digest.update(wal);
        }
        let id = uuid::Uuid::new_v4().to_string();
        let summary = DataSourceSummary {
            transfer_id: DataTransferId(id.clone()),
            kind: DataSourceKind::CcSwitch,
            // These formats do not carry a trustworthy application release.
            application_version: "unknown".into(),
            schema_version: if source.format == SourceFormat::Sqlite { source.version } else { 0 },
            format_version: if source.format == SourceFormat::Json { source.version } else { 0 },
            format,
            digest_sha256: hex::encode(digest.finalize()),
        };
        cache.sources.insert(
            id,
            Staged {
                created: Instant::now(),
                target: input.target,
                source_id: input.transfer_id.0,
                snapshot,
                summary: summary.clone(),
            },
        );
        Ok(Json(summary))
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
async fn preview(
    Extension(state): Extension<State>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    request: Request,
) -> Result<Json<DataImportPreview>, StatusCode> {
    authenticate_data_migration(state.policy.as_deref(), request.headers(), &tenant)?;
    let headers = request.headers().clone();
    let bytes = axum::body::to_bytes(request.into_body(), 16 * 1024)
        .await
        .map_err(|_| StatusCode::PAYLOAD_TOO_LARGE)?;
    let input: PreviewDataImportRequest =
        serde_json::from_slice(&bytes).map_err(|_| StatusCode::BAD_REQUEST)?;
    tokio::task::spawn_blocking(move || {
        if input.target.scope.tenant_scope() != &tenant {
            return Err(StatusCode::FORBIDDEN);
        }
        authorize_data_migration(state.policy.as_deref(), &headers, &input.target, None)?;
        // Each category needs its real owner normalization and target observation.
        if input.selections.len() != 1 || input.selections[0].category != DataCategory::Providers {
            return Err(StatusCode::NOT_IMPLEMENTED);
        }
        if !matches!(input.selections[0].entries, DataEntrySelection::All) {
            return Err(StatusCode::BAD_REQUEST);
        }
        super::super::agent_apps::authorize_provider_config_access(
            state.policy.as_deref(),
            &headers,
            &tenant,
            &input.target.target_host_id,
            false,
        )?;
        let mut cache = state.cache.lock().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        cache.expire();
        if cache.previews.len() >= MAX_PREVIEWS {
            return Err(StatusCode::TOO_MANY_REQUESTS);
        }
        let staged = cache.sources.get(&input.transfer_id.0).ok_or(StatusCode::NOT_FOUND)?;
        if staged.target != input.target {
            return Err(StatusCode::FORBIDDEN);
        }
        // Revoked source grants also invalidate already staged references.
        authorize_data_migration(
            state.policy.as_deref(),
            &headers,
            &input.target,
            Some(&staged.source_id),
        )?;
        let scope = source_scope(&input.target)?;
        let db = match database {
            Some(Extension(db)) => db,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
        };
        let store =
            AppProviderStore::new(db.clone()).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        let mut rows = Vec::new();
        for app in [
            AppId::Claude,
            AppId::ClaudeDesktop,
            AppId::Codex,
            AppId::Gemini,
            AppId::GrokBuild,
            AppId::OpenCode,
            AppId::OpenClaw,
            AppId::Hermes,
            AppId::Pi,
            AppId::Mcode,
            AppId::Kimi,
            AppId::Dsh,
            AppId::Agy,
            AppId::Omp,
        ] {
            for item in store.list(&tenant, app).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)? {
                rows.push(ProviderRow {
                    app,
                    value: AppProviderStore::preview_record(&db, &tenant, app, &item.id)
                        .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
                    id: item.id,
                });
            }
        }
        let normalized = providers::prepare(&scope, &staged.snapshot, &rows)
            .map_err(|_| StatusCode::UNPROCESSABLE_ENTITY)?;
        let mut conflicts = Vec::new();
        let mut eligible = 0;
        let mut excluded = 0;
        for entry in normalized.entries() {
            excluded += entry.excluded_fields as u64;
            if entry.difference != Difference::InvalidReference {
                eligible += 1;
            }
            if matches!(entry.difference, Difference::Different | Difference::InvalidReference) {
                conflicts.push(DataConflictSummary {
                    conflict_id: entry.id.to_string(),
                    category: DataCategory::Providers,
                    entry_id: entry.id.to_string(),
                    allowed_decisions: if entry.difference == Difference::InvalidReference {
                        vec![DataConflictDecision::Skip]
                    } else {
                        vec![DataConflictDecision::Skip, DataConflictDecision::Replace]
                    },
                });
            }
        }
        let id = uuid::Uuid::new_v4().to_string();
        let result = DataImportPreview {
            target: input.target.clone(),
            preview_id: DataPreviewId(id.clone()),
            // Unpredictable server reference binds the exact retained source,
            // normalized selection, decisions and target observations.
            revision: uuid::Uuid::new_v4().to_string(),
            source: staged.summary.clone(),
            categories: vec![DataCategorySummary {
                category: DataCategory::Providers,
                selected: normalized.entries().len() as u64,
                eligible,
                excluded,
            }],
            conflicts,
            exclusions: if excluded == 0 {
                vec![]
            } else {
                vec![DataExclusionSummary {
                    category: DataCategory::Providers,
                    reason: DataExclusionReason::UnsupportedField,
                    count: excluded,
                }]
            },
        };
        cache.previews.insert(
            id,
            HeldPreview {
                created: Instant::now(),
                _target: input.target,
                _transfer: input.transfer_id.0,
                _preview: normalized,
                _response: result.clone(),
            },
        );
        Ok(Json(result))
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}
fn source_scope(target: &DataManagementTarget) -> Result<SourceScope, StatusCode> {
    SourceScope::new(
        target.scope.clone(),
        target.target_host_id.clone(),
        target.principal_id.clone(),
    )
    .map_err(|_| StatusCode::BAD_REQUEST)
}
