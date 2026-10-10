use anyhow::{Result, bail};
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteDispatchTarget {
    pub room_id: String,
    pub recipient_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RemoteDispatchRequest {
    pub channel: String,
    pub target: String,
    pub recipient_id: Option<String>,
    pub body: String,
    pub payload_json: Value,
}

#[async_trait]
pub trait RemoteDispatchAdapter: Send + Sync {
    async fn send_message(&self, request: RemoteDispatchRequest) -> Result<()>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct LocalOnlyDispatchAdapter;

#[async_trait]
impl RemoteDispatchAdapter for LocalOnlyDispatchAdapter {
    async fn send_message(&self, _request: RemoteDispatchRequest) -> Result<()> {
        bail!("remote outbox dispatch is not supported")
    }
}
