//! Gateway HTTP 服务运行时入口。
//!
//! 本模块负责根据启动选项构造路由、绑定监听地址，并把 Axum 服务放入 Tokio
//! 任务中运行。端口为 `0` 时会优先尝试桌面端约定端口，再回退到系统分配端口。

use std::future::Future;
use std::net::SocketAddr;

use crate::app::agent::gateway::error::ApiError;
use crate::app::agent::gateway::options::ServeOptions;
use crate::app::agent::gateway::router::build_router;

/// 启动 Gateway 服务并等待其结束。
///
/// # 参数
///
/// * `opts` - 服务主机名、端口和 CORS 等启动选项。
///
/// # 返回值
///
/// 服务正常退出后返回实际监听地址。
///
/// # 错误处理
///
/// 绑定失败、服务任务 panic 或 Axum 服务运行失败都会转换为 `ApiError`。
pub async fn serve(opts: ServeOptions) -> Result<SocketAddr, ApiError> {
    serve_until(opts, shutdown_signal()).await
}

async fn serve_until<F>(opts: ServeOptions, shutdown: F) -> Result<SocketAddr, ApiError>
where
    F: Future<Output = ()> + Send + 'static,
{
    let (addr, handle) = start_until(opts, shutdown).await?;
    let res = handle.await.map_err(|e| ApiError::internal(e.to_string()))?;
    res?;
    Ok(addr)
}

/// 启动 Gateway 服务并返回后台任务句柄。
///
/// # 参数
///
/// * `opts` - 服务主机名、端口和 CORS 等启动选项。
///
/// # 返回值
///
/// 返回实际监听地址和服务任务句柄，调用方可自行决定何时等待任务结束。
///
/// # 错误处理
///
/// 路由绑定或监听地址读取失败会返回 `ApiError`。
pub async fn start(
    opts: ServeOptions,
) -> Result<(SocketAddr, tokio::task::JoinHandle<Result<(), ApiError>>), ApiError> {
    start_until(opts, shutdown_signal()).await
}

async fn start_until<F>(
    opts: ServeOptions,
    shutdown: F,
) -> Result<(SocketAddr, tokio::task::JoinHandle<Result<(), ApiError>>), ApiError>
where
    F: Future<Output = ()> + Send + 'static,
{
    let router = build_router(opts.cors);
    let listener = bind_prefer(&opts.hostname, opts.port).await?;
    let addr = listener.local_addr().map_err(|e| ApiError::internal(e.to_string()))?;
    let handle = tokio::spawn(async move {
        // Ctrl-C 时优雅停机，让已接收的请求有机会完成响应。
        axum::serve(listener, router.into_make_service_with_connect_info::<SocketAddr>())
            .with_graceful_shutdown(shutdown)
            .await
            .map_err(|e| ApiError::internal(e.to_string()))
    });
    Ok((addr, handle))
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

/// 绑定首选监听地址。
///
/// # 参数
///
/// * `host` - 监听主机名或 IP。
/// * `port` - 监听端口；非零时表示必须使用该端口。
///
/// # 返回值
///
/// 返回已经绑定好的 TCP listener。
///
/// # 错误处理
///
/// 指定端口绑定失败，或自动端口绑定最终失败时返回 `ApiError`。
pub(crate) async fn bind_prefer(
    host: &str,
    port: u16,
) -> Result<tokio::net::TcpListener, ApiError> {
    if port != 0 {
        return tokio::net::TcpListener::bind((host, port))
            .await
            .map_err(|e| ApiError::internal(e.to_string()));
    }
    // 桌面端默认希望使用稳定端口；被占用时再交给系统分配，提升并行测试可靠性。
    if let Ok(listener) = tokio::net::TcpListener::bind((host, 4099)).await {
        return Ok(listener);
    }
    tokio::net::TcpListener::bind((host, 0)).await.map_err(|e| ApiError::internal(e.to_string()))
}

#[cfg(test)]
#[path = "runtime_tests.rs"]
mod runtime_tests;
