//! Task Pool 运维命令。

use std::io::{Read, Write};

use anyhow::{Context, Result, bail};
use clap::Subcommand;
use serde_json::json;
use vw_gateway_client::vw_api_types::task::{
    TaskPoolWorktreeAcquireRequest, TaskPoolWorktreeMaintenanceRequest,
    TaskPoolWorktreeMergeLockRequest, TaskPoolWorktreeMergeUnlockRequest,
    TaskPoolWorktreeRecycleRequest, TaskPoolWorktreeReleaseRequest,
    TaskPoolWorktreeSlotRecycleRequest, TaskPoolWorktreeSnapshotRequest,
};

use super::context::ApiContext;
use super::input::read_json_input;
use super::output;

#[derive(Subcommand, Debug)]
pub(super) enum TaskPoolCommands {
    Snapshot { #[arg(long, value_parser = required)] input: String },
    Maintain { #[arg(long, value_parser = required)] input: String },
    Acquire { #[arg(long, value_parser = required)] input: String },
    Release { #[arg(long, value_parser = required)] input: String },
    Recycle { #[arg(long, value_parser = required)] input: String },
    SlotRecycle { #[arg(long, value_parser = required)] input: String },
    MergeLock { #[arg(long, value_parser = required)] input: String },
    MergeUnlock { #[arg(long, value_parser = required)] input: String },
    Events { #[arg(long)] after_cursor: Option<u64> },
    /// Matrix publish is reserved for authenticated internal producers.
    Publish,
}

impl TaskPoolCommands {
    pub(super) fn dangerous_operation(&self) -> Option<String> {
        match self {
            Self::Maintain { .. } => Some("maintain task-pool worktrees".to_string()),
            Self::Acquire { .. } => Some("acquire task-pool worktree lease".to_string()),
            Self::Release { .. } => Some("release task-pool worktree lease".to_string()),
            Self::Recycle { .. } => Some("recycle task-pool worktree".to_string()),
            Self::SlotRecycle { .. } => Some("recycle task-pool worktree slot".to_string()),
            Self::MergeLock { .. } => Some("lock task-pool merge target".to_string()),
            Self::MergeUnlock { .. } => Some("unlock task-pool merge target".to_string()),
            Self::Snapshot { .. } | Self::Events { .. } | Self::Publish => None,
        }
    }
}

pub(super) async fn run(
    command: TaskPoolCommands,
    context: &ApiContext,
    stdin: &mut impl Read,
    writer: &mut impl Write,
) -> Result<()> {
    let client = context.client();
    let response = match command {
        TaskPoolCommands::Snapshot { input } => {
            let body = parse::<TaskPoolWorktreeSnapshotRequest>(&input, stdin, "snapshot")?;
            json!(client.task_pool_worktree_snapshot(&body).await.map_err(anyhow::Error::msg)?)
        }
        TaskPoolCommands::Maintain { input } => {
            let body = parse::<TaskPoolWorktreeMaintenanceRequest>(&input, stdin, "maintain")?;
            json!(client.task_pool_worktree_maintain(&body).await.map_err(anyhow::Error::msg)?)
        }
        TaskPoolCommands::Acquire { input } => {
            let body = parse::<TaskPoolWorktreeAcquireRequest>(&input, stdin, "acquire")?;
            json!(client.task_pool_worktree_acquire(&body).await.map_err(anyhow::Error::msg)?)
        }
        TaskPoolCommands::Release { input } => {
            let body = parse::<TaskPoolWorktreeReleaseRequest>(&input, stdin, "release")?;
            json!(client.task_pool_worktree_release(&body).await.map_err(anyhow::Error::msg)?)
        }
        TaskPoolCommands::Recycle { input } => {
            let body = parse::<TaskPoolWorktreeRecycleRequest>(&input, stdin, "recycle")?;
            json!(client.task_pool_worktree_recycle(&body).await.map_err(anyhow::Error::msg)?)
        }
        TaskPoolCommands::SlotRecycle { input } => {
            let body = parse::<TaskPoolWorktreeSlotRecycleRequest>(&input, stdin, "slot recycle")?;
            json!(client.task_pool_worktree_slot_recycle(&body).await.map_err(anyhow::Error::msg)?)
        }
        TaskPoolCommands::MergeLock { input } => {
            let body = parse::<TaskPoolWorktreeMergeLockRequest>(&input, stdin, "merge lock")?;
            json!(client.task_pool_worktree_merge_lock(&body).await.map_err(anyhow::Error::msg)?)
        }
        TaskPoolCommands::MergeUnlock { input } => {
            let body = parse::<TaskPoolWorktreeMergeUnlockRequest>(&input, stdin, "merge unlock")?;
            json!(client.task_pool_worktree_merge_unlock(&body).await.map_err(anyhow::Error::msg)?)
        }
        TaskPoolCommands::Events { after_cursor } => {
            json!(client.task_pool_matrix_events(after_cursor).await.map_err(anyhow::Error::msg)?)
        }
        TaskPoolCommands::Publish => {
            bail!("task-pool Matrix publish is an internal producer callback and is not supported by shiju")
        }
    };
    output::write_json(writer, &response, context.output_format())
}

fn parse<T: serde::de::DeserializeOwned>(source: &str, stdin: &mut impl Read, operation: &str) -> Result<T> {
    serde_json::from_value(read_json_input(source, stdin)?).with_context(|| format!("invalid task-pool {operation} JSON input"))
}

fn required(value: &str) -> Result<String, String> {
    let value = value.trim();
    if value.is_empty() { return Err("value must not be empty".to_string()); }
    Ok(value.to_string())
}
