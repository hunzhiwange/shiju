//! 提供 vw-cli 主入口。
//! 入口负责进程级初始化并委托 CLI 模块处理具体命令。

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
#[path = "main_tests.rs"]
mod main_tests;
