//! Gateway HTTP 服务运行时入口。
//!
//! 宿主只初始化数据库、同步生命周期与实际路由，服务运行交给 agent-gateway。

use std::net::SocketAddr;
#[cfg(test)]
use std::future::Future;
use gateway_types::{ApiError, ServeOptions};
#[cfg(test)]
use agent_gateway::runtime::bind_prefer;

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
async fn serve_until<F>(opts: ServeOptions, shutdown: F) -> Result<SocketAddr, ApiError>
where F: Future<Output = ()> + Send + 'static,
{
    let (router, sync) = runtime_router(opts.cors.clone()).await?;
    agent_gateway::runtime::serve_until(opts, router, sync, shutdown).await
}

#[cfg(test)]
async fn start_until<F>(opts: ServeOptions, shutdown: F)
    -> Result<(SocketAddr, tokio::task::JoinHandle<Result<(), ApiError>>), ApiError>
where F: Future<Output = ()> + Send + 'static,
{
    let (router, sync) = runtime_router(opts.cors.clone()).await?;
    agent_gateway::runtime::start_until(opts, router, sync, shutdown).await
}

#[cfg(test)]
#[path = "../../../libs/agent-gateway/src/runtime_tests.rs"]
mod runtime_tests;
