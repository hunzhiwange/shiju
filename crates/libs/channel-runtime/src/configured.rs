//! 已配置通道与有界入站转发。
use std::sync::Arc;
use anyhow::Result;
use channel_types::{self as traits, Channel};

/// 已配置的通道信息。
///
/// 包装通道实例及其人类可读的显示名称。
#[derive(Clone)]
pub struct ConfiguredChannel {
    /// 稳定实例 ID，独立于显示名称。
    pub instance_id: String,
    /// 用于日志和 UI 显示的名称
    pub display_name: String,
    /// 通道实例
    pub channel: Arc<dyn Channel>,
}

impl ConfiguredChannel {
    pub fn with_instance_id(mut self, instance_id: &str) -> Self {
        self.instance_id = instance_id.to_string();
        self
    }

    pub fn stamp_inbound_message(&self, message: &mut traits::ChannelMessage) {
        message.instance_id.clone_from(&self.instance_id);
    }

    /// 保持有界背压；监听退出前排空已接收消息，取消时不遗留转发任务。
    pub async fn listen(
        &self,
        tx: tokio::sync::mpsc::Sender<traits::ChannelMessage>,
    ) -> Result<()> {
        let (instance_tx, mut instance_rx) = tokio::sync::mpsc::channel(100);
        let listen = self.channel.listen(instance_tx);
        tokio::pin!(listen);
        loop {
            tokio::select! {
                biased;
                result = &mut listen => {
                    while let Ok(mut message) = instance_rx.try_recv() {
                        self.stamp_inbound_message(&mut message);
                        if tx.send(message).await.is_err() { break; }
                    }
                    return result;
                }
                message = instance_rx.recv() => {
                    let Some(mut message) = message else { return listen.await; };
                    self.stamp_inbound_message(&mut message);
                    if tx.send(message).await.is_err() { return Ok(()); }
                }
                _ = tx.closed() => return Ok(()),
            }
        }
    }

    pub fn new(display_name: impl Into<String>, channel: Arc<dyn Channel>) -> Self {
        Self { display_name: display_name.into(), channel, instance_id: "default".into() }
    }
}
