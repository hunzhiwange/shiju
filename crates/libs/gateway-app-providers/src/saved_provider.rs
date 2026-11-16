//! Reapply saved selections using each provider's existing authorization and journal.
#![cfg(target_os = "macos")]
use agent_database::GlobalDatabase;
use axum::http::HeaderMap;
use std::path::PathBuf;
use vw_api_app_identity::AppId;
use vw_api_app_operations::DataManagementTarget;

/// Never activates an inactive record or consumes remote JSON.
pub async fn sync_saved_provider(
    database: GlobalDatabase,
    policy: Option<PathBuf>,
    headers: HeaderMap,
    target: &DataManagementTarget,
    app: AppId,
    id: &str,
    publication: crate::PublicationControl,
) -> Result<(), ()> {
    match app {
        AppId::Claude => crate::claude::sync_saved(database, policy, headers, target, id).await,
        AppId::Codex => {
            crate::codex_catalog::sync_saved(database, policy, headers, target, id, publication).await
        }
        _ => Err(()),
    }
}
