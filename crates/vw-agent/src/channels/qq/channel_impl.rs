use async_trait::async_trait;

use super::QQChannel;

/// Channel trait 实现
///
/// 为 `QQChannel` 实现标准的 `Channel` trait，提供消息发送、监听和健康检查能力。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl channel_types::Channel for QQChannel {
    /// 获取通道名称
    fn name(&self) -> &str {
        "qq"
    }

    /// 发送消息到指定接收者
    async fn send(
        &self,
        message: &channel_types::SendMessage,
    ) -> anyhow::Result<()> {
        self.send_message(message).await
    }

    /// 启动 WebSocket 监听循环
    async fn listen(
        &self,
        tx: tokio::sync::mpsc::Sender<channel_types::ChannelMessage>,
    ) -> anyhow::Result<()> {
        self.listen_gateway(tx).await
    }

    /// 执行健康检查
    async fn health_check(&self) -> bool {
        self.fetch_access_token().await.is_ok()
    }
}

#[cfg(test)]
#[path = "channel_impl_tests.rs"]
mod channel_impl_tests;
