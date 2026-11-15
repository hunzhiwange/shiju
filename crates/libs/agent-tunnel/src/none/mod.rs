//! 提供不创建外部隧道的本地直连实现。
//! 该实现用于显式关闭隧道能力，避免在不需要公网入口时扩大网络暴露面。

use super::Tunnel;
use anyhow::Result;

/// NoneTunnel 表示该模块对外暴露的结构化状态。
pub struct NoneTunnel;

#[async_trait::async_trait]
impl Tunnel for NoneTunnel {
    fn name(&self) -> &str {
        "none"
    }

    async fn start(&self, local_host: &str, local_port: u16) -> Result<String> {
        Ok(format!("http://{local_host}:{local_port}"))
    }

    async fn stop(&self) -> Result<()> {
        Ok(())
    }

    async fn health_check(&self) -> bool {
        true
    }

    fn public_url(&self) -> Option<String> {
        None
    }
}

#[cfg(test)]
mod tests;
