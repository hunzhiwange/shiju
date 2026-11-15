use super::RoutingListener;
use anyhow::{Context, Result};
use std::{future::Future, sync::Arc};

/// Keep the single routing listener alive for the gateway's serving lifetime.
pub(crate) async fn run_with_routing(
    serve: impl AsyncFnOnce(&Arc<RoutingListener>) -> Result<()>,
    shutdown: impl Future<Output = Result<()>>,
) -> Result<()> {
    let database = crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home)?;
    let routing = Arc::new(RoutingListener::with_database(database));
    let result = tokio::select! {
        biased;
        result = shutdown => result,
        result = serve(&routing) => result,
    };
    routing.shutdown().await.context("gateway could not stop application routing")?;
    result
}
