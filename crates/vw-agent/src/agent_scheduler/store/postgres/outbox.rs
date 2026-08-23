//! 实现 PostgreSQL 事务发件箱的幂等入队、并发认领及投递状态更新。
//! 认领使用行锁跳过其它消费者占用的消息，使多个派发器可安全并行排空。

use ::postgres::{GenericClient, Row};
use anyhow::{Context, Result, bail};
use vw_api_types::agent_scheduler::AgentSchedulerScope;
use vw_api_types::common::TimestampMs;

use super::PostgresAgentSchedulerStore;
use crate::agent_scheduler::store::AgentSchedulerStorageContext;
use crate::agent_scheduler::store::outbox::{
    EnqueueOutboxMessageRequest, OutboxMessageStatus, StoredOutboxMessage,
};

const OUTBOX_COLUMNS: &str = "id, target_kind, target_id, recipient_id, body, status, \
idempotency_key, attempts, next_attempt_at_ms, delivered_at_ms, last_error, payload_json, \
created_at_ms, updated_at_ms";

impl PostgresAgentSchedulerStore {
    pub(in crate::agent_scheduler::store) fn enqueue_outbox_message(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: EnqueueOutboxMessageRequest,
    ) -> Result<StoredOutboxMessage> {
        crate::global_database::run_sync_database(|| {
            let payload_json =
                serde_json::to_string(&request.payload).context("serialize outbox payload")?;
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL outbox enqueue")?;
            transaction
                .execute(
                    "INSERT INTO scheduler_outbox_messages (
                     platform_id, company_id, project_id, id, target_kind, target_id, recipient_id,
                     body, status, idempotency_key, attempts, next_attempt_at_ms, payload_json,
                     created_at_ms, updated_at_ms
                 ) VALUES (
                     $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, 0, $11, $12, $13, $13
                 )
                 ON CONFLICT (platform_id, company_id, project_id, idempotency_key) DO NOTHING",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &request.message_id,
                        &request.channel,
                        &request.target,
                        &request.recipient_id,
                        &request.body,
                        &OutboxMessageStatus::Pending.as_str(),
                        &request.idempotency_key,
                        &request.next_attempt_at_ms.map(|value| value.0),
                        &payload_json,
                        &request.created_at_ms.0,
                    ],
                )
                .context("insert scoped PostgreSQL outbox message")?;
            let stored = load_outbox_message_by_idempotency_key(
                &mut transaction,
                &scope,
                &request.idempotency_key,
            )?
            .with_context(|| {
                format!(
                    "PostgreSQL outbox message disappeared after enqueue: {}",
                    request.idempotency_key
                )
            })?;
            transaction.commit().context("commit PostgreSQL outbox enqueue")?;
            Ok(stored)
        })
    }

    pub(in crate::agent_scheduler::store) fn claim_outbox_batch(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        limit: i64,
    ) -> Result<Vec<StoredOutboxMessage>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL outbox claim")?;
            let message_ids = transaction
                .query(
                    "SELECT id
                 FROM scheduler_outbox_messages
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND status IN ($4, $5)
                   AND (next_attempt_at_ms IS NULL OR next_attempt_at_ms <= $6)
                 ORDER BY COALESCE(next_attempt_at_ms, created_at_ms), created_at_ms, id
                 LIMIT $7
                 FOR UPDATE SKIP LOCKED",
                    &[
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &OutboxMessageStatus::Pending.as_str(),
                        &OutboxMessageStatus::Failed.as_str(),
                        &now_ms.0,
                        &limit,
                    ],
                )
                .context("lock claimable scoped PostgreSQL outbox messages")?
                .iter()
                .map(|row| row.try_get::<_, String>(0).context("read PostgreSQL outbox message id"))
                .collect::<Result<Vec<_>>>()?;

            let mut messages = Vec::with_capacity(message_ids.len());
            for message_id in message_ids {
                let changed = transaction
                    .execute(
                        "UPDATE scheduler_outbox_messages
                     SET status = $1,
                         attempts = attempts + 1,
                         next_attempt_at_ms = NULL,
                         updated_at_ms = $2
                     WHERE platform_id = $3 AND company_id = $4 AND project_id = $5 AND id = $6
                       AND status IN ($7, $8)
                       AND (next_attempt_at_ms IS NULL OR next_attempt_at_ms <= $2)",
                        &[
                            &OutboxMessageStatus::Claimed.as_str(),
                            &now_ms.0,
                            &project.platform_id().as_ref(),
                            &project.company_id().as_ref(),
                            &project.project_id().as_ref(),
                            &message_id,
                            &OutboxMessageStatus::Pending.as_str(),
                            &OutboxMessageStatus::Failed.as_str(),
                        ],
                    )
                    .context("claim scoped PostgreSQL outbox message")?;
                if changed != 1 {
                    bail!("PostgreSQL outbox claim affected {changed} rows: {message_id}");
                }
                messages.push(
                    load_outbox_message_by_id(&mut transaction, &scope, &message_id)?
                        .with_context(|| {
                            format!(
                                "PostgreSQL outbox message disappeared after claim: {message_id}"
                            )
                        })?,
                );
            }
            transaction.commit().context("commit PostgreSQL outbox claim")?;
            Ok(messages)
        })
    }

    pub(in crate::agent_scheduler::store) fn mark_outbox_delivered(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        message_id: &str,
        delivered_at_ms: TimestampMs,
    ) -> Result<Option<StoredOutboxMessage>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL outbox delivery")?;
            let changed = transaction
                .execute(
                    "UPDATE scheduler_outbox_messages
                 SET status = $1, next_attempt_at_ms = NULL, delivered_at_ms = $2,
                     last_error = NULL, updated_at_ms = $2
                 WHERE platform_id = $3 AND company_id = $4 AND project_id = $5 AND id = $6
                   AND status = $7",
                    &[
                        &OutboxMessageStatus::Delivered.as_str(),
                        &delivered_at_ms.0,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &message_id,
                        &OutboxMessageStatus::Claimed.as_str(),
                    ],
                )
                .context("deliver scoped PostgreSQL outbox message")?;
            match changed {
                0 => return Ok(None),
                1 => {}
                _ => bail!("PostgreSQL outbox delivery affected {changed} rows: {message_id}"),
            }
            let stored = load_outbox_message_by_id(&mut transaction, &scope, message_id)?
                .with_context(|| {
                    format!("PostgreSQL outbox message disappeared after delivery: {message_id}")
                })?;
            transaction.commit().context("commit PostgreSQL outbox delivery")?;
            Ok(Some(stored))
        })
    }

    pub(in crate::agent_scheduler::store) fn mark_outbox_failed(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        message_id: &str,
        error_message: &str,
        failed_at_ms: TimestampMs,
        next_attempt_at_ms: TimestampMs,
    ) -> Result<Option<StoredOutboxMessage>> {
        crate::global_database::run_sync_database(|| {
            let scope = context.storage_scope()?;
            let project = scope.project_scope();
            let mut connection = self.database.connect_postgres()?;
            let mut transaction =
                connection.transaction().context("begin PostgreSQL outbox failure")?;
            let changed = transaction
                .execute(
                    "UPDATE scheduler_outbox_messages
                 SET status = $1, next_attempt_at_ms = $2, last_error = $3, updated_at_ms = $4
                 WHERE platform_id = $5 AND company_id = $6 AND project_id = $7 AND id = $8
                   AND status = $9",
                    &[
                        &OutboxMessageStatus::Failed.as_str(),
                        &next_attempt_at_ms.0,
                        &error_message,
                        &failed_at_ms.0,
                        &project.platform_id().as_ref(),
                        &project.company_id().as_ref(),
                        &project.project_id().as_ref(),
                        &message_id,
                        &OutboxMessageStatus::Claimed.as_str(),
                    ],
                )
                .context("fail scoped PostgreSQL outbox message")?;
            match changed {
                0 => return Ok(None),
                1 => {}
                _ => bail!("PostgreSQL outbox failure affected {changed} rows: {message_id}"),
            }
            let stored = load_outbox_message_by_id(&mut transaction, &scope, message_id)?
                .with_context(|| {
                    format!("PostgreSQL outbox message disappeared after failure: {message_id}")
                })?;
            transaction.commit().context("commit PostgreSQL outbox failure")?;
            Ok(Some(stored))
        })
    }
}

fn load_outbox_message_by_id(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    message_id: &str,
) -> Result<Option<StoredOutboxMessage>> {
    load_outbox_message(client, scope, "id", message_id)
}

fn load_outbox_message_by_idempotency_key(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    idempotency_key: &str,
) -> Result<Option<StoredOutboxMessage>> {
    load_outbox_message(client, scope, "idempotency_key", idempotency_key)
}

fn load_outbox_message(
    client: &mut impl GenericClient,
    scope: &AgentSchedulerScope,
    key_column: &str,
    key: &str,
) -> Result<Option<StoredOutboxMessage>> {
    debug_assert!(matches!(key_column, "id" | "idempotency_key"));
    let project = scope.project_scope();
    client
        .query_opt(
            &format!(
                "SELECT {OUTBOX_COLUMNS} FROM scheduler_outbox_messages
                 WHERE platform_id = $1 AND company_id = $2 AND project_id = $3
                   AND {key_column} = $4"
            ),
            &[
                &project.platform_id().as_ref(),
                &project.company_id().as_ref(),
                &project.project_id().as_ref(),
                &key,
            ],
        )
        .with_context(|| format!("load scoped PostgreSQL outbox message by {key_column}: {key}"))?
        .as_ref()
        .map(map_outbox_message_row)
        .transpose()
}

fn map_outbox_message_row(row: &Row) -> Result<StoredOutboxMessage> {
    let status: String = row.try_get(5).context("read PostgreSQL outbox status")?;
    let attempts: i64 = row.try_get(7).context("read PostgreSQL outbox attempts")?;
    let payload_json: String = row.try_get(11).context("read PostgreSQL outbox payload")?;
    Ok(StoredOutboxMessage {
        id: row.try_get(0).context("read PostgreSQL outbox id")?,
        channel: row.try_get(1).context("read PostgreSQL outbox target kind")?,
        target: row.try_get(2).context("read PostgreSQL outbox target id")?,
        recipient_id: row.try_get(3).context("read PostgreSQL outbox recipient")?,
        body: row.try_get(4).context("read PostgreSQL outbox body")?,
        status: OutboxMessageStatus::parse(&status).context("parse PostgreSQL outbox status")?,
        idempotency_key: row.try_get(6).context("read PostgreSQL outbox idempotency key")?,
        attempts: u32::try_from(attempts)
            .with_context(|| format!("PostgreSQL outbox attempts out of range: {attempts}"))?,
        next_attempt_at_ms: row.try_get::<_, Option<i64>>(8)?.map(TimestampMs),
        delivered_at_ms: row.try_get::<_, Option<i64>>(9)?.map(TimestampMs),
        last_error: row.try_get(10).context("read PostgreSQL outbox last error")?,
        payload: serde_json::from_str(&payload_json).context("parse PostgreSQL outbox payload")?,
        created_at_ms: TimestampMs(row.try_get(12).context("read PostgreSQL outbox creation")?),
        updated_at_ms: TimestampMs(row.try_get(13).context("read PostgreSQL outbox update")?),
    })
}

#[cfg(test)]
#[path = "outbox_tests.rs"]
mod tests;
