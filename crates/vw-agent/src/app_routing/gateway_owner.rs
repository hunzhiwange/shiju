use super::RoutingListener;
use anyhow::Result;
use std::{future::Future, sync::Arc};

pub(crate) async fn run_with_routing(
    serve: impl AsyncFnOnce(&Arc<RoutingListener>) -> Result<()>,
    shutdown: impl Future<Output = Result<()>>,
) -> Result<()> {
    let database = crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home)?;
    app_routing_services::gateway_owner::run_with_routing(database, serve, shutdown).await
}
