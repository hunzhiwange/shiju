//! Authenticated model edits use the provider catalog transaction.

use super::{agent_apps::authorize_provider_config_access, model_catalog::registered_instance};
use crate::{
    app_providers::AppProviderStore,
    global_database::GlobalDatabase,
    project::{self, store::ProjectStore},
};
use axum::{
    Extension, Json, Router,
    extract::{
        DefaultBodyLimit, Query,
        rejection::{JsonRejection, QueryRejection},
    },
    http::{HeaderMap, StatusCode},
    routing::post,
};
use serde::{Serialize, de::DeserializeOwned};
use std::path::PathBuf;
use vw_agent_apps::providers;
use vw_api_types::{
    id::TenantScope,
    model_catalog::{
        ModelCatalogDefaultRequest, ModelCatalogEditRequest, ModelCatalogFillCommitRequest,
        ModelCatalogFillPreviewRequest, ModelCatalogFillPreviewResponse, ModelCatalogIdentity,
        ModelCatalogProject, ModelCatalogWriteResponse,
    },
};
use vw_provider_resolver::models::{self, ModelsSnapshot};

#[derive(Clone)]
struct Policy(Option<PathBuf>);

pub(super) fn router_with_policy(policy: Option<PathBuf>) -> Router {
    Router::new()
        .route("/model-catalog/edits", post(handle::<ModelCatalogEditRequest>))
        .route("/model-catalog/defaults", post(handle::<ModelCatalogDefaultRequest>))
        .route("/model-catalog/fill-preview", post(handle::<ModelCatalogFillPreviewRequest>))
        .route("/model-catalog/fill-commit", post(handle::<ModelCatalogFillCommitRequest>))
        .layer(DefaultBodyLimit::max(256 * 1024))
        .layer(Extension(Policy(policy)))
}

trait CatalogOperation: DeserializeOwned + Send + 'static {
    type Output: Serialize + Send + 'static;

    const WRITE: bool = true;
    const USES_SNAPSHOT: bool = false;

    fn target(&self) -> (&ModelCatalogIdentity, Option<&TenantScope>);

    fn apply(
        self,
        store: &AppProviderStore,
        tenant: &TenantScope,
        snapshot: Option<&ModelsSnapshot>,
    ) -> Result<Self::Output, String>;
}

async fn handle<T: CatalogOperation>(
    Extension(policy): Extension<Policy>,
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
    headers: HeaderMap,
    query: Result<Query<ModelCatalogProject>, QueryRejection>,
    body: Result<Json<T>, JsonRejection>,
) -> Result<Json<T::Output>, StatusCode> {
    let Query(project) = query.map_err(|_| StatusCode::BAD_REQUEST)?;
    let Json(input) = body.map_err(|error| match error.status() {
        StatusCode::PAYLOAD_TOO_LARGE => StatusCode::PAYLOAD_TOO_LARGE,
        _ => StatusCode::BAD_REQUEST,
    })?;
    let (deployment, scope) = input.target();
    let deployment = deployment.clone();
    let scope_matches = scope.is_none_or(|scope| scope == &tenant);
    let owner = tenant.clone();
    let store = tokio::task::spawn_blocking(move || {
        authorize_provider_config_access(
            policy.0.as_deref(),
            &headers,
            &owner,
            &deployment.target_host_id,
            T::WRITE,
        )?;
        if !scope_matches {
            return Err(StatusCode::FORBIDDEN);
        }
        providers::validate_identity(&deployment.target_host_id)
            .and_then(|_| providers::validate_identity(&deployment.provider_id))
            .and_then(|_| providers::validation::endpoint(&deployment.endpoint))
            .map_err(|_| StatusCode::BAD_REQUEST)?;
        let (database, projects) = match database {
            Some(Extension(database)) => {
                let projects = ProjectStore::new(database.clone())
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
                (database, projects)
            }
            None => (
                GlobalDatabase::for_home(&crate::global::paths().home)
                    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
                project::runtime_store().map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?,
            ),
        };
        registered_instance(&projects, &owner, &project.project_id, &project.directory)?;
        AppProviderStore::new(database).map_err(|_| StatusCode::SERVICE_UNAVAILABLE)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)??;

    let snapshot = if T::USES_SNAPSHOT { Some(models::snapshot().await) } else { None };
    tokio::task::spawn_blocking(move || {
        input.apply(&store, &tenant, snapshot.as_deref()).map(Json).map_err(status)
    })
    .await
    .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
}

fn status(error: String) -> StatusCode {
    match error.as_str() {
        "model catalog changed" => StatusCode::CONFLICT,
        "application provider not found" | "model not found in deployment" => StatusCode::NOT_FOUND,
        "model fill target changed" => StatusCode::FORBIDDEN,
        "invalid model ID"
        | "conflicting edits for one model"
        | "invalid display alias"
        | "invalid model limits"
        | "invalid model modalities"
        | "invalid model cost"
        | "cannot remove default model"
        | "default model not found in deployment" => StatusCode::UNPROCESSABLE_ENTITY,
        _ => StatusCode::SERVICE_UNAVAILABLE,
    }
}

impl CatalogOperation for ModelCatalogEditRequest {
    type Output = ModelCatalogWriteResponse;

    fn target(&self) -> (&ModelCatalogIdentity, Option<&TenantScope>) {
        (&self.deployment, None)
    }

    fn apply(
        self,
        store: &AppProviderStore,
        tenant: &TenantScope,
        _: Option<&ModelsSnapshot>,
    ) -> Result<Self::Output, String> {
        store.edit_model_catalog(tenant, &self)
    }
}

impl CatalogOperation for ModelCatalogDefaultRequest {
    type Output = ModelCatalogWriteResponse;

    fn target(&self) -> (&ModelCatalogIdentity, Option<&TenantScope>) {
        (&self.deployment, None)
    }

    fn apply(
        self,
        store: &AppProviderStore,
        tenant: &TenantScope,
        _: Option<&ModelsSnapshot>,
    ) -> Result<Self::Output, String> {
        store.edit_model_catalog(
            tenant,
            &ModelCatalogEditRequest {
                deployment: self.deployment,
                expected_revision: self.expected_revision,
                edits: Vec::new(),
                default_change: Some(self.change),
            },
        )
    }
}

impl CatalogOperation for ModelCatalogFillPreviewRequest {
    type Output = ModelCatalogFillPreviewResponse;
    const WRITE: bool = false;
    const USES_SNAPSHOT: bool = true;

    fn target(&self) -> (&ModelCatalogIdentity, Option<&TenantScope>) {
        (&self.identity.deployment, Some(&self.identity.scope))
    }

    fn apply(
        self,
        store: &AppProviderStore,
        tenant: &TenantScope,
        snapshot: Option<&ModelsSnapshot>,
    ) -> Result<Self::Output, String> {
        store.preview_model_catalog_fill(tenant, &self, snapshot)
    }
}

impl CatalogOperation for ModelCatalogFillCommitRequest {
    type Output = ModelCatalogWriteResponse;
    const USES_SNAPSHOT: bool = true;

    fn target(&self) -> (&ModelCatalogIdentity, Option<&TenantScope>) {
        (&self.identity.deployment, Some(&self.identity.scope))
    }

    fn apply(
        self,
        store: &AppProviderStore,
        tenant: &TenantScope,
        snapshot: Option<&ModelsSnapshot>,
    ) -> Result<Self::Output, String> {
        store.commit_model_catalog_fill(tenant, &self, snapshot)
    }
}
