//! `shiju acp` 的纯 stdio Agent 装配。

use acp_server::server::SessionBackend;
use tokio::io::{AsyncRead, AsyncWrite};

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
    acp_server::server::serve_with_io(backend, reader, writer)
        .await
        .map_err(|error| anyhow::anyhow!("ACP stdio connection failed: {error}"))
}

#[cfg(test)]
#[path = "acp_tests.rs"]
mod tests;

pub(crate) async fn run_employee_stdio(
    employee_id: String,
    kind: crate::cli::EmployeeAcpKind,
    state_dir: std::path::PathBuf,
    account_scope: String,
) -> anyhow::Result<()> {
    let credential = std::env::var("SHIJU_EMPLOYEE_ACP_CREDENTIAL").ok();
    run_employee_with_io(
        employee_id,
        kind,
        state_dir,
        account_scope,
        credential.as_deref(),
        tokio::io::stdin(),
        tokio::io::stdout(),
    )
    .await
}

async fn run_employee_with_io<R, W>(
    employee_id: String,
    kind: crate::cli::EmployeeAcpKind,
    state_dir: std::path::PathBuf,
    account_scope: String,
    credential: Option<&str>,
    reader: R,
    writer: W,
) -> anyhow::Result<()>
where
    R: AsyncRead + Unpin + 'static,
    W: AsyncWrite + Unpin + 'static,
{
    use acp_server::employee::{
        EmployeeAgent, EmployeeConversationClient, EmployeeKind, EmployeeTarget, ShijuCredential,
    };

    let credential = credential
        .and_then(|value| ShijuCredential::new(value).ok())
        .ok_or_else(|| anyhow::anyhow!("SHIJU_EMPLOYEE_ACP_CREDENTIAL is missing or invalid"))?;
    let kind = match kind {
        crate::cli::EmployeeAcpKind::Private => EmployeeKind::Private,
        crate::cli::EmployeeAcpKind::Shared => EmployeeKind::Shared,
    };
    let client = EmployeeConversationClient::production(&credential)?;
    let agent = EmployeeAgent::new(
        client,
        EmployeeTarget { employee_id, kind },
        &state_dir,
        &account_scope,
    )?;
    agent.serve_with_io(reader, writer).await?;
    Ok(())
}
