//! 提供 vibe-agent 二进制入口。
//! 入口只负责启动共享 CLI 逻辑，保持进程装配与业务处理分离。

use std::future::Future;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    run_cli(vw_cli::run).await
}

async fn run_cli<Run, Fut>(run: Run) -> anyhow::Result<()>
where
    Run: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    run().await
}

#[cfg(test)]
#[path = "vibe-agent_tests.rs"]
mod vibe_agent_tests;
