//! Read-only tenant icon library API.
use super::agent_apps_error::AgentAppsApiError;
use crate::{agent_apps::store::IconLibraryStore, global_database::GlobalDatabase};
use axum::{Extension, Json, Router, routing::get};
use vw_api_types::{agent_apps::CustomIconEntry, id::TenantScope};

pub(crate) fn router() -> Router {
    Router::new().route("/agent-apps/custom-icons", get(read_icons))
}

async fn read_icons(
    Extension(tenant): Extension<TenantScope>,
    database: Option<Extension<GlobalDatabase>>,
) -> Result<Json<Vec<CustomIconEntry>>, AgentAppsApiError> {
    tokio::task::spawn_blocking(move || {
        let database = match database {
            Some(Extension(database)) => database,
            None => GlobalDatabase::for_home(&crate::global::paths().home)
                .map_err(AgentAppsApiError::storage_unavailable)?,
        };
        let store =
            IconLibraryStore::new(database).map_err(AgentAppsApiError::storage_unavailable)?;
        store.read_icons(&tenant).map(Json).map_err(AgentAppsApiError::storage_unavailable)
    })
    .await
    .map_err(|error| AgentAppsApiError::storage_unavailable(error.into()))?
}
