use anyhow::{Context, Result, bail};
use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, TransactionBehavior, params};
use serde_json::Value;
use vw_api_types::common::TimestampMs;

use super::{
    AgentSchedulerStorageContext, AgentSchedulerStore, open_locked_initialized_connection,
    postgres_provider, scope_params,
};

const OUTBOX_COLUMNS: &str = "id, target_kind, target_id, recipient_id, body, status, \
idempotency_key, attempts, next_attempt_at_ms, delivered_at_ms, last_error, payload_json, \
created_at_ms, updated_at_ms";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutboxMessageStatus {
    Pending,
    Claimed,
    Failed,
    Delivered,
}

impl OutboxMessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Claimed => "claimed",
            Self::Failed => "failed",
            Self::Delivered => "delivered",
        }
    }

    pub(super) fn parse(value: &str) -> Result<Self> {
        match value {
            "pending" => Ok(Self::Pending),
            "claimed" => Ok(Self::Claimed),
            "failed" => Ok(Self::Failed),
            "delivered" => Ok(Self::Delivered),
            other => bail!("unsupported outbox message status: {other}"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnqueueOutboxMessageRequest {
    pub message_id: String,
    pub channel: String,
    pub target: String,
    pub recipient_id: Option<String>,
    pub body: String,
    pub idempotency_key: String,
    pub payload: Value,
    pub created_at_ms: TimestampMs,
    pub next_attempt_at_ms: Option<TimestampMs>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StoredOutboxMessage {
    pub id: String,
    pub channel: String,
    pub target: String,
    pub recipient_id: Option<String>,
    pub body: String,
    pub status: OutboxMessageStatus,
    pub idempotency_key: String,
    pub attempts: u32,
    pub next_attempt_at_ms: Option<TimestampMs>,
    pub delivered_at_ms: Option<TimestampMs>,
    pub last_error: Option<String>,
    pub payload: Value,
    pub created_at_ms: TimestampMs,
    pub updated_at_ms: TimestampMs,
}

impl AgentSchedulerStore {
    pub fn enqueue_outbox_message(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        request: EnqueueOutboxMessageRequest,
    ) -> Result<StoredOutboxMessage> {
        validate_enqueue_request(&request)?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.enqueue_outbox_message(context, request);
        }
        let payload_json =
            serde_json::to_string(&request.payload).context("serialize outbox payload")?;
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        if let Some(existing) =
            load_outbox_message_by_idempotency_key(&tx, context, &request.idempotency_key)?
        {
            tx.commit()?;
            return Ok(existing);
        }

        tx.execute(
            "INSERT INTO scheduler_outbox_messages (
                 platform_id, company_id, project_id, id, target_kind, target_id, recipient_id,
                 body, status, idempotency_key, attempts, next_attempt_at_ms, payload_json,
                 created_at_ms, updated_at_ms
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, 0, ?11, ?12, ?13, ?13)",
            params![
                platform_id,
                company_id,
                project_id,
                request.message_id,
                request.channel,
                request.target,
                request.recipient_id,
                request.body,
                OutboxMessageStatus::Pending.as_str(),
                request.idempotency_key,
                request.next_attempt_at_ms.map(|value| value.0),
                payload_json,
                request.created_at_ms.0,
            ],
        )?;
        let stored =
            load_outbox_message_by_id(&tx, context, &request.message_id)?.with_context(|| {
                format!("outbox message disappeared after enqueue: {}", request.message_id)
            })?;
        tx.commit()?;
        Ok(stored)
    }

    pub fn claim_outbox_batch(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        now_ms: TimestampMs,
        limit: usize,
    ) -> Result<Vec<StoredOutboxMessage>> {
        if limit == 0 {
            return Ok(Vec::new());
        }
        let limit = i64::try_from(limit).context("outbox claim limit out of range")?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.claim_outbox_batch(context, now_ms, limit);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let message_ids = claimable_outbox_message_ids(&tx, context, now_ms, limit)?;

        for message_id in &message_ids {
            tx.execute(
                "UPDATE scheduler_outbox_messages
                 SET status = ?1,
                     attempts = attempts + 1,
                     next_attempt_at_ms = NULL,
                     updated_at_ms = ?2
                 WHERE platform_id = ?3 AND company_id = ?4 AND project_id = ?5 AND id = ?6
                   AND status IN (?7, ?8)
                   AND (next_attempt_at_ms IS NULL OR next_attempt_at_ms <= ?2)",
                params![
                    OutboxMessageStatus::Claimed.as_str(),
                    now_ms.0,
                    platform_id,
                    company_id,
                    project_id,
                    message_id,
                    OutboxMessageStatus::Pending.as_str(),
                    OutboxMessageStatus::Failed.as_str(),
                ],
            )?;
        }

        let mut messages = Vec::with_capacity(message_ids.len());
        for message_id in message_ids {
            if let Some(message) = load_outbox_message_by_id(&tx, context, &message_id)? {
                messages.push(message);
            }
        }
        tx.commit()?;
        Ok(messages)
    }

    pub fn mark_outbox_delivered(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        message_id: &str,
        delivered_at_ms: TimestampMs,
    ) -> Result<Option<StoredOutboxMessage>> {
        let message_id = non_empty(message_id.to_string(), "outbox message id")?;
        if let Some(store) = postgres_provider(self, context)? {
            return store.mark_outbox_delivered(context, &message_id, delivered_at_ms);
        }
        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let updated = tx.execute(
            "UPDATE scheduler_outbox_messages
             SET status = ?1,
                 next_attempt_at_ms = NULL,
                 delivered_at_ms = ?2,
                 last_error = NULL,
                 updated_at_ms = ?2
             WHERE platform_id = ?3 AND company_id = ?4 AND project_id = ?5 AND id = ?6
               AND status = ?7",
            params![
                OutboxMessageStatus::Delivered.as_str(),
                delivered_at_ms.0,
                platform_id,
                company_id,
                project_id,
                message_id,
                OutboxMessageStatus::Claimed.as_str(),
            ],
        )?;
        if updated == 0 {
            tx.commit()?;
            return Ok(None);
        }
        let stored = load_outbox_message_by_id(&tx, context, &message_id)?
            .with_context(|| format!("outbox message disappeared after delivery: {message_id}"))?;
        tx.commit()?;
        Ok(Some(stored))
    }

    pub fn mark_outbox_failed(
        &self,
        context: &(impl AgentSchedulerStorageContext + ?Sized),
        message_id: &str,
        error_message: String,
        failed_at_ms: TimestampMs,
        next_attempt_at_ms: TimestampMs,
    ) -> Result<Option<StoredOutboxMessage>> {
        let message_id = non_empty(message_id.to_string(), "outbox message id")?;
        let error_message = non_empty(error_message, "outbox failure message")?;
        if next_attempt_at_ms < failed_at_ms {
            bail!("outbox next attempt time must not be before failure time");
        }
        if let Some(store) = postgres_provider(self, context)? {
            return store.mark_outbox_failed(
                context,
                &message_id,
                &error_message,
                failed_at_ms,
                next_attempt_at_ms,
            );
        }

        let scope = context.storage_scope()?;
        let (platform_id, company_id, project_id) = scope_params(&scope);
        let (_guard, mut conn) = open_locked_initialized_connection(self, context)?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let updated = tx.execute(
            "UPDATE scheduler_outbox_messages
             SET status = ?1,
                 next_attempt_at_ms = ?2,
                 last_error = ?3,
                 updated_at_ms = ?4
             WHERE platform_id = ?5 AND company_id = ?6 AND project_id = ?7 AND id = ?8
               AND status = ?9",
            params![
                OutboxMessageStatus::Failed.as_str(),
                next_attempt_at_ms.0,
                error_message,
                failed_at_ms.0,
                platform_id,
                company_id,
                project_id,
                message_id,
                OutboxMessageStatus::Claimed.as_str(),
            ],
        )?;
        if updated == 0 {
            tx.commit()?;
            return Ok(None);
        }
        let stored = load_outbox_message_by_id(&tx, context, &message_id)?
            .with_context(|| format!("outbox message disappeared after failure: {message_id}"))?;
        tx.commit()?;
        Ok(Some(stored))
    }
}

fn validate_enqueue_request(request: &EnqueueOutboxMessageRequest) -> Result<()> {
    non_empty(request.message_id.clone(), "outbox message id")?;
    non_empty(request.channel.clone(), "outbox channel")?;
    non_empty(request.target.clone(), "outbox target")?;
    if let Some(recipient_id) = &request.recipient_id {
        non_empty(recipient_id.clone(), "outbox recipient id")?;
    }
    non_empty(request.body.clone(), "outbox body")?;
    non_empty(request.idempotency_key.clone(), "outbox idempotency key")?;
    if let Some(next_attempt_at_ms) = request.next_attempt_at_ms
        && next_attempt_at_ms < request.created_at_ms
    {
        bail!("outbox next attempt time must not be before creation time");
    }
    Ok(())
}

fn claimable_outbox_message_ids(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    now_ms: TimestampMs,
    limit: i64,
) -> Result<Vec<String>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(
        "SELECT id
         FROM scheduler_outbox_messages
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3
           AND status IN (?4, ?5)
           AND (next_attempt_at_ms IS NULL OR next_attempt_at_ms <= ?6)
         ORDER BY COALESCE(next_attempt_at_ms, created_at_ms), created_at_ms, id
         LIMIT ?7",
    )?;
    let rows = stmt.query_map(
        params![
            platform_id,
            company_id,
            project_id,
            OutboxMessageStatus::Pending.as_str(),
            OutboxMessageStatus::Failed.as_str(),
            now_ms.0,
            limit,
        ],
        |row| row.get::<_, String>(0),
    )?;
    rows.collect::<rusqlite::Result<Vec<_>>>().context("list claimable outbox messages")
}

fn load_outbox_message_by_id(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    message_id: &str,
) -> Result<Option<StoredOutboxMessage>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(&format!(
        "SELECT {OUTBOX_COLUMNS} FROM scheduler_outbox_messages
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND id = ?4"
    ))?;
    stmt.query_row(params![platform_id, company_id, project_id, message_id], map_outbox_message_row)
        .optional()
        .with_context(|| format!("load outbox message: {message_id}"))
}

fn load_outbox_message_by_idempotency_key(
    conn: &Connection,
    context: &(impl AgentSchedulerStorageContext + ?Sized),
    idempotency_key: &str,
) -> Result<Option<StoredOutboxMessage>> {
    let scope = context.storage_scope()?;
    let (platform_id, company_id, project_id) = scope_params(&scope);
    let mut stmt = conn.prepare(&format!(
        "SELECT {OUTBOX_COLUMNS} FROM scheduler_outbox_messages
         WHERE platform_id = ?1 AND company_id = ?2 AND project_id = ?3 AND idempotency_key = ?4"
    ))?;
    stmt.query_row(
        params![platform_id, company_id, project_id, idempotency_key],
        map_outbox_message_row,
    )
    .optional()
    .with_context(|| format!("load outbox message by idempotency key: {idempotency_key}"))
}

fn map_outbox_message_row(row: &Row<'_>) -> rusqlite::Result<StoredOutboxMessage> {
    let status: String = row.get(5)?;
    let attempts: i64 = row.get(7)?;
    let payload_json: String = row.get(11)?;

    Ok(StoredOutboxMessage {
        id: row.get(0)?,
        channel: row.get(1)?,
        target: row.get(2)?,
        recipient_id: row.get(3)?,
        body: row.get(4)?,
        status: OutboxMessageStatus::parse(&status).map_err(|error| conversion_error(5, error))?,
        idempotency_key: row.get(6)?,
        attempts: u32::try_from(attempts).map_err(|error| {
            conversion_error(7, anyhow::anyhow!("outbox attempts out of range: {error}"))
        })?,
        next_attempt_at_ms: row.get::<_, Option<i64>>(8)?.map(TimestampMs),
        delivered_at_ms: row.get::<_, Option<i64>>(9)?.map(TimestampMs),
        last_error: row.get(10)?,
        payload: serde_json::from_str(&payload_json)
            .map_err(|error| conversion_error(11, anyhow::Error::new(error)))?,
        created_at_ms: TimestampMs(row.get(12)?),
        updated_at_ms: TimestampMs(row.get(13)?),
    })
}

fn non_empty(value: String, label: &str) -> Result<String> {
    if value.trim().is_empty() {
        bail!("{label} must not be empty");
    }
    Ok(value)
}

fn conversion_error(column: usize, error: anyhow::Error) -> rusqlite::Error {
    let error = std::io::Error::new(std::io::ErrorKind::InvalidData, error.to_string());
    rusqlite::Error::FromSqlConversionFailure(column, Type::Text, Box::new(error))
}
