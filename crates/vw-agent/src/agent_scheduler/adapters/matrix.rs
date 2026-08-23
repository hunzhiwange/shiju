//! 提供 Matrix 消息派发契约，并在未配置或缺少对应功能时明确拒绝发送。

use anyhow::Result;
use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
/// 已解析的 Matrix 房间与接收者路由。
pub struct MatrixDispatchTarget {
    pub room_id: String,
    pub recipient_id: String,
}

#[derive(Debug, Clone, PartialEq)]
/// Matrix 适配器投递消息所需的通道、路由、正文和结构化载荷。
pub struct MatrixDispatchRequest {
    pub channel: String,
    pub target: String,
    pub recipient_id: Option<String>,
    pub body: String,
    pub payload_json: Value,
}

#[async_trait]
/// 发件箱面向 Matrix 的异步投递边界。
pub trait MatrixDispatchAdapter: Send + Sync {
    async fn send_message(&self, request: MatrixDispatchRequest) -> Result<()>;
}

#[derive(Debug, Clone, Copy, Default)]
/// 未配置 Matrix 时使用的拒绝型适配器。
pub struct NoMatrixDispatchAdapter;

#[async_trait]
impl MatrixDispatchAdapter for NoMatrixDispatchAdapter {
    async fn send_message(&self, _request: MatrixDispatchRequest) -> Result<()> {
        anyhow::bail!(
            "Matrix outbox dispatch is not configured; configure a Matrix dispatch adapter to deliver Matrix outbox messages"
        )
    }
}

#[derive(Debug, Clone, Copy, Default)]
/// 将请求转交给当前构建所启用 Matrix 客户端的适配器。
pub struct ConfiguredMatrixDispatchAdapter;

#[async_trait]
impl MatrixDispatchAdapter for ConfiguredMatrixDispatchAdapter {
    async fn send_message(&self, request: MatrixDispatchRequest) -> Result<()> {
        send_configured_matrix_message(request).await
    }
}

#[cfg(feature = "channel-matrix")]
async fn send_configured_matrix_message(request: MatrixDispatchRequest) -> Result<()> {
    use anyhow::Context;

    use crate::app::agent::channels::matrix::MatrixChannel;
    use crate::app::agent::channels::traits::{Channel, SendMessage};
    use crate::app::agent::config::schema::{Config, ConfigExt};
    use vw_api_types::agent_scheduler::AgentSchedulerMatrixDispatchEventDto;

    let config = Config::load_or_init().await.context("load Matrix dispatch configuration")?;
    if !config.channels_config.matrix_enabled {
        anyhow::bail!("Matrix outbox dispatch is disabled");
    }
    let matrix = config
        .channels_config
        .matrix
        .clone()
        .context("Matrix outbox dispatch requires channels.matrix configuration")?;
    let room_id = matrix_dispatch_room_id(&request.target, &matrix.room_id)?;
    let config_dir = config.config_path.parent().map(std::path::PathBuf::from);
    let channel = MatrixChannel::new_with_session_hint_instance_id_and_vibewindow_dir(
        matrix.homeserver,
        matrix.access_token,
        room_id,
        matrix.allowed_users,
        matrix.user_id,
        matrix.device_id,
        None,
        config_dir,
    )
    .with_mention_only(matrix.mention_only);
    let dispatch_event: AgentSchedulerMatrixDispatchEventDto =
        serde_json::from_value(request.payload_json.clone())
            .context("parse Matrix scheduler dispatch event payload")?;
    channel
        .send_agent_scheduler_dispatch_matrix_event(&dispatch_event)
        .await
        .context("send Matrix scheduler dispatch event")?;
    let recipient = request.recipient_id.clone().unwrap_or_else(|| request.target.clone());
    channel
        .send(&SendMessage::new(request.body, recipient))
        .await
        .context("send Matrix outbox message")?;
    Ok(())
}

#[cfg(feature = "channel-matrix")]
fn matrix_dispatch_room_id(target: &str, fallback_room_id: &str) -> Result<String> {
    let Some(room_id) = target.strip_prefix("room:") else {
        let fallback = fallback_room_id.trim();
        if fallback.is_empty() {
            anyhow::bail!("Matrix outbox target must start with room:");
        }
        return Ok(fallback.to_string());
    };
    let room_id = room_id.trim();
    if room_id.is_empty() {
        anyhow::bail!("Matrix outbox room target must not be empty");
    }
    Ok(room_id.to_string())
}

#[cfg(not(feature = "channel-matrix"))]
async fn send_configured_matrix_message(_request: MatrixDispatchRequest) -> Result<()> {
    anyhow::bail!("Matrix outbox dispatch requires the channel-matrix feature")
}

#[cfg(test)]
mod fake {
    use std::collections::VecDeque;
    use std::sync::{Arc, Mutex};

    use anyhow::{Result, bail};

    use super::{MatrixDispatchAdapter, MatrixDispatchRequest};

    #[derive(Debug, Clone, Default)]
    pub struct FakeMatrixDispatchAdapter {
        requests: Arc<Mutex<Vec<MatrixDispatchRequest>>>,
        outcomes: Arc<Mutex<VecDeque<std::result::Result<(), String>>>>,
    }

    impl FakeMatrixDispatchAdapter {
        pub fn new() -> Self {
            Self::default()
        }

        pub fn succeed_next(&self) {
            self.outcomes.lock().expect("fake matrix outcomes lock poisoned").push_back(Ok(()));
        }

        pub fn fail_next(&self, error_message: impl Into<String>) {
            self.outcomes
                .lock()
                .expect("fake matrix outcomes lock poisoned")
                .push_back(Err(error_message.into()));
        }

        pub fn recorded_requests(&self) -> Vec<MatrixDispatchRequest> {
            self.requests.lock().expect("fake matrix requests lock poisoned").clone()
        }
    }

    #[async_trait::async_trait]
    impl MatrixDispatchAdapter for FakeMatrixDispatchAdapter {
        async fn send_message(&self, request: MatrixDispatchRequest) -> Result<()> {
            self.requests.lock().expect("fake matrix requests lock poisoned").push(request);

            match self.outcomes.lock().expect("fake matrix outcomes lock poisoned").pop_front() {
                Some(Ok(())) | None => Ok(()),
                Some(Err(error_message)) => bail!("{error_message}"),
            }
        }
    }
}

#[cfg(test)]
pub use fake::FakeMatrixDispatchAdapter;

#[cfg(test)]
#[path = "matrix_tests.rs"]
mod matrix_tests;
