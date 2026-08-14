//! `vibewindow acp` 的纯 stdio Agent 装配。

use tokio::io::{AsyncRead, AsyncWrite};
use vw_acp::server::SessionBackend;

/// 使用进程 stdin/stdout 启动单连接 ACP Agent。
pub(crate) async fn run_stdio() -> anyhow::Result<()> {
    run_with_io(
        vw_agent::acp::VibeWindowBackend::default(),
        tokio::io::stdin(),
        tokio::io::stdout(),
    )
    .await
}

pub(crate) async fn run_with_io<B, R, W>(backend: B, reader: R, writer: W) -> anyhow::Result<()>
where
    B: SessionBackend,
    R: AsyncRead + Unpin + 'static,
    W: AsyncWrite + Unpin + 'static,
{
    vw_acp::server::serve_with_io(backend, reader, writer)
        .await
        .map_err(|error| anyhow::anyhow!("ACP stdio connection failed: {error}"))
}

#[cfg(test)]
#[path = "acp_tests.rs"]
mod tests;
