//! Gateway HTTP 服务运行时入口。
//!
//! 宿主只初始化数据库、同步生命周期与实际路由，服务运行交给 agent-gateway。

use std::net::SocketAddr;
use gateway_types::{ApiError, ServeOptions};

async fn runtime_router(cors: Vec<String>) -> Result<(axum::Router, device_sync::lifecycle::SyncLifecycle), ApiError> {
    let database = tokio::task::spawn_blocking(|| {
        crate::global_database::GlobalDatabase::for_home(&crate::global::paths().home)
    }).await.map_err(|_| ApiError::internal("settings backend unavailable"))?
        .map_err(|_| ApiError::internal("settings backend unavailable"))?;
    let sync = device_sync::lifecycle::SyncLifecycle::default();
    let router = super::router::build_router(cors).layer(axum::Extension(database))
        .layer(axum::Extension(sync.clone()));
    Ok((router, sync))
}

pub async fn serve(opts: ServeOptions) -> Result<SocketAddr, ApiError> {
    let (router, sync) = runtime_router(opts.cors.clone()).await?;
    agent_gateway::runtime::serve(opts, router, sync).await
}

pub async fn start(opts: ServeOptions)
    -> Result<(SocketAddr, tokio::task::JoinHandle<Result<(), ApiError>>), ApiError>
{
    let (router, sync) = runtime_router(opts.cors.clone()).await?;
    agent_gateway::runtime::start(opts, router, sync).await
}

#[cfg(test)]
#[path = "../../../libs/agent-gateway/src/runtime_tests.rs"]
mod runtime_tests;
