//! Await the host's live projection owners after the receive has committed.
use std::{future::Future, path::Path, pin::Pin, sync::Arc};
use agent_database::GlobalDatabase;
use agent_app_services::shared_snapshot::SnapshotSelection;
use axum::http::HeaderMap;
use vw_api_app_operations::DataManagementTarget;

/// Host composition action retaining the live routing authority and domain owners.
pub trait ProjectionHost: Send + Sync {
    fn refresh<'a>(&'a self, database: &'a GlobalDatabase, policy: Option<&'a Path>,
        headers: &'a HeaderMap, target: &'a DataManagementTarget, selection: &'a SnapshotSelection,
    ) -> Pin<Box<dyn Future<Output = Result<(), ()>> + Send + 'a>>;
}
#[derive(Clone)]
pub struct DownloadRouting(pub Arc<dyn ProjectionHost>);

pub(super) async fn refresh(database: &GlobalDatabase, policy: Option<&Path>, headers: &HeaderMap,
    target: &DataManagementTarget, selection: &SnapshotSelection, control: Option<&DownloadRouting>,
) -> Result<(), ()> {
    control.ok_or(())?.0.refresh(database, policy, headers, target, selection).await
}
