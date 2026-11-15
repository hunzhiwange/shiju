//! 从环境变量解析队列所有者运行参数。

use std::collections::HashMap;
use std::env;

use serde_json::Value;
use thiserror::Error;

use crate::mcp_servers::parse_optional_mcp_servers;
use acp_owner_process::{
    QUEUE_OWNER_PAYLOAD_ENV, QueueOwnerRuntimeOptions,
};
use crate::session_runtime::runtime::{SessionRuntimeError, run_session_queue_owner};
use crate::types::{AuthPolicy, NonInteractivePermissionPolicy, PermissionMode};

#[derive(Debug, Error)]
pub enum QueueOwnerPayloadError {
    #[error("vwacp queue owner payload is missing")]
    MissingPayload,
    #[error("{0}")]
    InvalidPayload(String),
}

#[derive(Debug, Error)]
pub enum QueueOwnerRunFromEnvError {
    #[error(transparent)]
    Payload(#[from] QueueOwnerPayloadError),
    #[error(transparent)]
    Runtime(#[from] Box<SessionRuntimeError>),
}

impl From<SessionRuntimeError> for QueueOwnerRunFromEnvError {
    fn from(err: SessionRuntimeError) -> Self {
        Self::Runtime(Box::new(err))
    }
}

pub fn parse_queue_owner_payload(
    payload: &str,
) -> Result<QueueOwnerRuntimeOptions, QueueOwnerPayloadError> {
    let parsed = serde_json::from_str::<Value>(payload)
        .map_err(|error| invalid_payload(error.to_string()))?;
    let record = parsed
        .as_object()
        .ok_or_else(|| invalid_payload("queue owner payload must be an object"))?;

    let session_id = record
        .get("sessionId")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| invalid_payload("queue owner payload missing sessionId"))?
        .to_string();

    let permission_mode = match record.get("permissionMode").and_then(Value::as_str) {
        Some("approve-all") => PermissionMode::ApproveAll,
        Some("approve-reads") => PermissionMode::ApproveReads,
        Some("deny-all") => PermissionMode::DenyAll,
        _ => {
            return Err(invalid_payload("queue owner payload has invalid permissionMode"));
        }
    };

    let mut options = QueueOwnerRuntimeOptions {
        session_id,
        mcp_servers: None,
        permission_mode,
        non_interactive_permissions: None,
        auth_credentials: None,
        auth_policy: None,
        suppress_sdk_console_errors: None,
        verbose: None,
        ttl_ms: None,
        max_queue_depth: None,
        prompt_retries: None,
    };

    if let Some(mcp_servers) = parse_optional_mcp_servers(
        record.get("mcpServers").filter(|value| !value.is_null()),
        "queue owner payload",
    )
    .map_err(|error| invalid_payload(error.to_string()))?
    {
        options.mcp_servers = Some(mcp_servers);
    }

    options.non_interactive_permissions =
        parse_non_interactive_permissions(record.get("nonInteractivePermissions"));
    options.auth_credentials = parse_auth_credentials(record.get("authCredentials"));
    options.auth_policy = parse_auth_policy(record.get("authPolicy"));
    options.suppress_sdk_console_errors =
        record.get("suppressSdkConsoleErrors").and_then(Value::as_bool);
    options.verbose = record.get("verbose").and_then(Value::as_bool);
    options.ttl_ms = parse_ttl_ms(record.get("ttlMs"));
    options.max_queue_depth = parse_max_queue_depth(record.get("maxQueueDepth"));
    options.prompt_retries = parse_prompt_retries(record.get("promptRetries"));

    Ok(options)
}

pub fn queue_owner_runtime_options_from_env()
-> Result<Option<QueueOwnerRuntimeOptions>, QueueOwnerPayloadError> {
    let Some(payload) = env::var(QUEUE_OWNER_PAYLOAD_ENV).ok() else {
        return Ok(None);
    };
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return Err(QueueOwnerPayloadError::MissingPayload);
    }
    parse_queue_owner_payload(trimmed).map(Some)
}

pub async fn run_queue_owner_from_env() -> Result<(), QueueOwnerRunFromEnvError> {
    let payload =
        env::var(QUEUE_OWNER_PAYLOAD_ENV).map_err(|_| QueueOwnerPayloadError::MissingPayload)?;
    let trimmed = payload.trim();
    if trimmed.is_empty() {
        return Err(QueueOwnerPayloadError::MissingPayload.into());
    }
    let options = parse_queue_owner_payload(trimmed)?;
    run_session_queue_owner(options).await?;
    Ok(())
}

fn invalid_payload(message: impl Into<String>) -> QueueOwnerPayloadError {
    QueueOwnerPayloadError::InvalidPayload(message.into())
}

fn parse_non_interactive_permissions(
    value: Option<&Value>,
) -> Option<NonInteractivePermissionPolicy> {
    match value.and_then(Value::as_str) {
        Some("deny") => Some(NonInteractivePermissionPolicy::Deny),
        Some("fail") => Some(NonInteractivePermissionPolicy::Fail),
        _ => None,
    }
}

fn parse_auth_credentials(value: Option<&Value>) -> Option<HashMap<String, String>> {
    let Value::Object(entries) = value? else {
        return None;
    };

    Some(
        entries
            .iter()
            .filter_map(|(key, value)| value.as_str().map(|value| (key.clone(), value.to_string())))
            .collect(),
    )
}

fn parse_auth_policy(value: Option<&Value>) -> Option<AuthPolicy> {
    match value.and_then(Value::as_str) {
        Some("skip") => Some(AuthPolicy::Skip),
        Some("fail") => Some(AuthPolicy::Fail),
        _ => None,
    }
}

fn parse_ttl_ms(value: Option<&Value>) -> Option<u64> {
    let number = value.and_then(Value::as_f64)?;
    if !number.is_finite() || number < 0.0 || number > u64::MAX as f64 || number.fract() != 0.0 {
        return None;
    }
    Some(number as u64)
}

fn parse_max_queue_depth(value: Option<&Value>) -> Option<usize> {
    let number = value.and_then(Value::as_f64)?;
    if !number.is_finite() {
        return None;
    }
    Some(number.round().max(1.0).min(usize::MAX as f64) as usize)
}

fn parse_prompt_retries(value: Option<&Value>) -> Option<u64> {
    let number = value.and_then(Value::as_f64)?;
    if !number.is_finite() {
        return None;
    }
    Some(number.round().max(0.0).min(u64::MAX as f64) as u64)
}
