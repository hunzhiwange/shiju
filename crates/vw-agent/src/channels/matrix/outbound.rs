//! Matrix 通道的出站消息发送实现。
//!
//! 本模块负责在发送前解析目标房间、同步一次缺失房间状态，并确保只向已加入
//! 且受支持的房间发送消息。

use super::MatrixChannel;
use crate::app::agent::channels::traits::SendMessage;
use async_trait::async_trait;
use matrix_sdk::{
    Room, RoomState, config::SyncSettings, ruma::OwnedRoomId,
    ruma::events::room::message::RoomMessageEventContent,
};
use serde_json::Value;
use std::sync::atomic::Ordering;
use vw_api_types::agent_scheduler::{
    AGENT_SCHEDULER_MATRIX_DISPATCH_EVENT_TYPE, AgentSchedulerMatrixDispatchEventDto,
};
use vw_api_types::task::{
    TASK_POOL_MATRIX_EVENT_TYPE, TaskPoolMatrixEventDto, TaskPoolMatrixEventKindDto, TaskPoolStatus,
};

#[async_trait]
pub(super) trait MatrixRawEventSender {
    async fn send_raw_event(
        &self,
        event_type: &'static str,
        content: Value,
    ) -> anyhow::Result<String>;

    async fn send_notice(&self, message: String) -> anyhow::Result<()>;
}

#[async_trait]
impl MatrixRawEventSender for Room {
    async fn send_raw_event(
        &self,
        event_type: &'static str,
        content: Value,
    ) -> anyhow::Result<String> {
        let response = self.send_raw(event_type, content).await?;
        Ok(response.event_id.to_string())
    }

    async fn send_notice(&self, message: String) -> anyhow::Result<()> {
        self.send(RoomMessageEventContent::notice_plain(message)).await?;
        Ok(())
    }
}

impl MatrixChannel {
    async fn joined_target_room(&self) -> anyhow::Result<Room> {
        if self.otk_conflict_detected.load(Ordering::Relaxed) {
            anyhow::bail!("{}", self.otk_conflict_recovery_message());
        }

        let client = self.matrix_client().await?;
        let target_room_id = self.target_room_id().await?;
        let target_room: OwnedRoomId = target_room_id.parse()?;

        let mut room = client.get_room(&target_room);
        if room.is_none() {
            // 本地 store 可能尚未有房间快照，发送前同步一次可以避免误报房间不存在。
            let _ = client.sync_once(SyncSettings::new()).await;
            room = client.get_room(&target_room);
        }

        let Some(room) = room else {
            anyhow::bail!("{}", matrix_join_required_message(&target_room_id));
        };

        if room.state() != RoomState::Joined {
            anyhow::bail!("{}", matrix_join_required_message(&target_room_id));
        }

        Ok(room)
    }

    /// 向配置的 Matrix 目标房间发送消息。
    ///
    /// 参数：`message` 是运行时标准消息，当前实现发送其 Markdown 文本正文。
    ///
    /// 返回值：发送成功返回 `Ok(())`。
    ///
    /// 错误处理：检测到 OTK 冲突、房间 ID 无法解析、目标房间不存在、未加入房间
    /// 或 Matrix SDK 发送失败时返回错误。
    pub(super) async fn send_impl(&self, message: &SendMessage) -> anyhow::Result<()> {
        let room = self.joined_target_room().await?;
        // 使用 Markdown 内容保留代理回复中的列表、代码块等格式。
        room.send(RoomMessageEventContent::text_markdown(&message.content)).await?;
        Ok(())
    }

    pub(crate) async fn send_task_pool_matrix_event(
        &self,
        event: &TaskPoolMatrixEventDto,
    ) -> anyhow::Result<String> {
        event.validate_for_matrix_room_event().map_err(anyhow::Error::msg)?;
        let room = self.joined_target_room().await?;
        Self::send_task_pool_matrix_event_with_sender(&room, event).await
    }

    pub(crate) async fn send_agent_scheduler_dispatch_matrix_event(
        &self,
        event: &AgentSchedulerMatrixDispatchEventDto,
    ) -> anyhow::Result<String> {
        event.validate_for_matrix_room_event().map_err(anyhow::Error::msg)?;
        let room = self.joined_target_room().await?;
        Self::send_agent_scheduler_dispatch_event_with_sender(&room, event).await
    }

    async fn send_task_pool_matrix_event_with_sender(
        sender: &impl MatrixRawEventSender,
        event: &TaskPoolMatrixEventDto,
    ) -> anyhow::Result<String> {
        event.validate_for_matrix_room_event().map_err(anyhow::Error::msg)?;
        let content = serde_json::to_value(event)?;
        let matrix_event_id = sender.send_raw_event(TASK_POOL_MATRIX_EVENT_TYPE, content).await?;
        sender.send_notice(task_pool_matrix_notice(event)).await?;
        Ok(matrix_event_id)
    }

    async fn send_agent_scheduler_dispatch_event_with_sender(
        sender: &impl MatrixRawEventSender,
        event: &AgentSchedulerMatrixDispatchEventDto,
    ) -> anyhow::Result<String> {
        event.validate_for_matrix_room_event().map_err(anyhow::Error::msg)?;
        let content = serde_json::to_value(event)?;
        let matrix_event_id =
            sender.send_raw_event(AGENT_SCHEDULER_MATRIX_DISPATCH_EVENT_TYPE, content).await?;
        Ok(matrix_event_id)
    }
}

fn matrix_join_required_message(target_room_id: &str) -> String {
    format!(
        "Matrix room '{target_room_id}' is not joined by the configured Matrix account. \
        Check channels.matrix.room_id and invite or join the bot account to that room before publishing task-pool events."
    )
}

fn task_pool_matrix_notice(event: &TaskPoolMatrixEventDto) -> String {
    let actor = event
        .actor
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(event.actor.matrix_user_id.as_str());
    let detail = match &event.kind {
        TaskPoolMatrixEventKindDto::TaskStatusChanged { from_status, to_status } => {
            match from_status {
                Some(from_status) => {
                    format!(
                        "状态 {} -> {}",
                        task_pool_status_label(*from_status),
                        task_pool_status_label(*to_status)
                    )
                }
                None => format!("状态 -> {}", task_pool_status_label(*to_status)),
            }
        }
        TaskPoolMatrixEventKindDto::TaskClaimed { assignee } => {
            format!("认领给 {assignee}")
        }
        TaskPoolMatrixEventKindDto::TaskNote { message } => {
            format!("备注：{}", message.trim())
        }
        TaskPoolMatrixEventKindDto::TaskBlocked { reason } => {
            format!("阻塞：{}", reason.trim())
        }
    };
    format!("VibeWindow 任务池事件 · {} · {} · {}", event.task_id, detail, actor)
}

fn task_pool_status_label(status: TaskPoolStatus) -> &'static str {
    match status {
        TaskPoolStatus::Pool => "pool",
        TaskPoolStatus::Pending => "pending",
        TaskPoolStatus::Planning => "planning",
        TaskPoolStatus::Running => "running",
        TaskPoolStatus::Failed => "failed",
        TaskPoolStatus::Paused => "paused",
        TaskPoolStatus::CodeComplete => "code_complete",
        TaskPoolStatus::CodeReview => "code_review",
        TaskPoolStatus::PrSubmitted => "pr_submitted",
        TaskPoolStatus::Completed => "completed",
        TaskPoolStatus::Archived => "archived",
    }
}

#[cfg(test)]
#[path = "outbound_tests.rs"]
mod outbound_tests;
