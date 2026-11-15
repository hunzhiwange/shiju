//! Tool hook result mapping, independent of the host hook registry.
use crate::{host::ContextHost, PermissionDecision, ToolCallError};
use serde_json::Value;
use std::{marker::PhantomData, time::Duration};
use tool_types::{ToolCallResult, ToolResult};

pub enum PreHookOutcome {
    Continue((String, Value)),
    Cancel(String),
}
pub struct PreToolHook<H>(PhantomData<H>);
impl<H: ContextHost> PreToolHook<H> {
    pub async fn run(runner: Option<&H::HookRunner>, name: String, input: Value) -> Result<(String, Value), ToolCallError> {
        let Some(runner) = runner else { return Ok((name, input)); };
        match H::pre_hook(runner, name, input).await {
            PreHookOutcome::Continue(value) => Ok(value),
            PreHookOutcome::Cancel(reason) => Err(ToolCallError::denied(format!("Cancelled by hook: {reason}"))),
        }
    }
}
pub struct PermissionHook;
impl PermissionHook {
    pub fn adapt(decision: PermissionDecision) -> PermissionDecision { decision }
}
pub struct PostToolHook<H>(PhantomData<H>);
impl<H: ContextHost> PostToolHook<H> {
    pub async fn run(runner: Option<&H::HookRunner>, name: &str, result: &ToolCallResult, duration: Duration) {
        let legacy = ToolResult { success: result.is_success(), output: result.model_text(), error: result.error_text() };
        Self::run_legacy(runner, name, &legacy, duration).await;
    }
    pub async fn run_legacy(runner: Option<&H::HookRunner>, name: &str, result: &ToolResult, duration: Duration) {
        if let Some(runner) = runner { H::post_hook(runner, name, result, duration).await; }
    }
}
