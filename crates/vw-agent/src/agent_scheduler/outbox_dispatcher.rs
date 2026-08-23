//! 从事务发件箱认领待发消息，经通道适配器投递并记录成功或可重试失败。

use anyhow::{Context, Result, bail};
use vw_api_types::common::TimestampMs;

use super::adapters::matrix::{MatrixDispatchAdapter, MatrixDispatchRequest};
use super::store::outbox::StoredOutboxMessage;
use super::store::{AgentSchedulerStorageContext, AgentSchedulerStore};

pub const DEFAULT_OUTBOX_RETRY_DELAY_MS: i64 = 60_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutboxDrainSummary {
    pub claimed: usize,
    pub delivered: usize,
    pub failed: usize,
}

pub struct OutboxDispatcher<'a, A>
where
    A: MatrixDispatchAdapter + ?Sized,
{
    store: &'a AgentSchedulerStore,
    matrix_adapter: &'a A,
}

impl<'a, A> OutboxDispatcher<'a, A>
where
    A: MatrixDispatchAdapter + ?Sized,
{
    pub fn new(store: &'a AgentSchedulerStore, matrix_adapter: &'a A) -> Self {
        Self { store, matrix_adapter }
    }

    pub async fn drain_once(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        limit: usize,
    ) -> Result<OutboxDrainSummary> {
        self.drain_messages_once(project_path, now_ms, limit)
            .await
            .map(|(summary, _messages)| summary)
    }

    pub async fn drain_messages_once(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        limit: usize,
    ) -> Result<(OutboxDrainSummary, Vec<StoredOutboxMessage>)> {
        tracing::info!(project_path = %project_path.storage_project_path().display(), now_ms = now_ms.0, limit, "agent scheduler outbox drain started");
        let messages = self.store.claim_outbox_batch(project_path, now_ms, limit)?;
        let mut summary = OutboxDrainSummary { claimed: messages.len(), delivered: 0, failed: 0 };
        tracing::info!(project_path = %project_path.storage_project_path().display(), claimed = summary.claimed, "agent scheduler outbox drain claimed messages");

        for message in messages.clone() {
            match self.dispatch_message(project_path, now_ms, message).await? {
                DispatchOutcome::Delivered => summary.delivered += 1,
                DispatchOutcome::Failed => summary.failed += 1,
            }
        }

        tracing::info!(project_path = %project_path.storage_project_path().display(), claimed = summary.claimed, delivered = summary.delivered, failed = summary.failed, "agent scheduler outbox drain finished");
        Ok((summary, messages))
    }

    async fn dispatch_message(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        message: StoredOutboxMessage,
    ) -> Result<DispatchOutcome> {
        tracing::info!(project_path = %project_path.storage_project_path().display(), outbox_message_id = message.id.as_str(), channel = message.channel.as_str(), target = message.target.as_str(), attempts = message.attempts, "agent scheduler outbox dispatch message started");
        let target = match outbox_dispatch_target(&message) {
            Ok(target) => target,
            Err(error) => {
                tracing::warn!(project_path = %project_path.storage_project_path().display(), outbox_message_id = message.id.as_str(), error = %error, "agent scheduler outbox dispatch target parse failed");
                self.mark_failed(project_path, now_ms, &message.id, error.to_string())?;
                return Ok(DispatchOutcome::Failed);
            }
        };

        let OutboxDispatchTarget::Matrix(request) = target;
        tracing::info!(project_path = %project_path.storage_project_path().display(), outbox_message_id = message.id.as_str(), target = request.target.as_str(), recipient_id = ?request.recipient_id.as_deref(), "agent scheduler sending Matrix outbox message");
        if let Err(error) = self.matrix_adapter.send_message(request).await {
            tracing::warn!(project_path = %project_path.storage_project_path().display(), outbox_message_id = message.id.as_str(), error = %error, "agent scheduler Matrix outbox send failed");
            self.mark_failed(project_path, now_ms, &message.id, error.to_string())?;
            return Ok(DispatchOutcome::Failed);
        }

        self.store
            .mark_outbox_delivered(project_path, &message.id, now_ms)?
            .with_context(|| format!("claimed outbox message was not delivered: {}", message.id))?;
        tracing::info!(project_path = %project_path.storage_project_path().display(), outbox_message_id = message.id.as_str(), "agent scheduler Matrix outbox delivered");
        Ok(DispatchOutcome::Delivered)
    }

    fn mark_failed(
        &self,
        project_path: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        message_id: &str,
        error_message: String,
    ) -> Result<()> {
        let error_message = non_empty_failure_message(error_message);
        let next_attempt_at_ms = retry_after(now_ms);
        tracing::warn!(project_path = %project_path.storage_project_path().display(), outbox_message_id = message_id, error = %error_message, next_attempt_at_ms = next_attempt_at_ms.0, "agent scheduler outbox message marked failed");
        self.store
            .mark_outbox_failed(
                project_path,
                message_id,
                error_message,
                now_ms,
                next_attempt_at_ms,
            )?
            .with_context(|| format!("claimed outbox message was not failed: {message_id}"))?;
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispatchOutcome {
    Delivered,
    Failed,
}

#[derive(Debug, Clone, PartialEq)]
enum OutboxDispatchTarget {
    Matrix(MatrixDispatchRequest),
}

fn outbox_dispatch_target(message: &StoredOutboxMessage) -> Result<OutboxDispatchTarget> {
    match message.channel.as_str() {
        "matrix" => Ok(OutboxDispatchTarget::Matrix(MatrixDispatchRequest {
            channel: message.channel.clone(),
            target: message.target.clone(),
            recipient_id: message.recipient_id.clone(),
            body: message.body.clone(),
            payload_json: message.payload.clone(),
        })),
        other => bail!("unsupported outbox channel for Matrix dispatcher: {other}"),
    }
}

fn retry_after(now_ms: TimestampMs) -> TimestampMs {
    TimestampMs(now_ms.0.saturating_add(DEFAULT_OUTBOX_RETRY_DELAY_MS))
}

fn non_empty_failure_message(error_message: String) -> String {
    let error_message = error_message.trim();
    if error_message.is_empty() {
        "matrix dispatch failed".to_string()
    } else {
        error_message.into()
    }
}
