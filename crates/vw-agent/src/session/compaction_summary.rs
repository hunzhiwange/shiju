//! Single-request context checkpoint summaries using the normal model request lifecycle.

use crate::config::ReliabilityConfig;
use crate::session::processor::llm_runner;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

const SUMMARY_PROMPT: &str = "Create a concise context checkpoint for another model to continue this conversation. Preserve progress, key decisions, user constraints, unfinished work, next steps, and essential references. Return only the handoff summary. Do not execute tools or continue the task.";

pub(crate) fn summarize(
    session_id: &str,
    history: &[Value],
    model: &str,
    reliability: &ReliabilityConfig,
    cancellation: Option<&CancellationToken>,
) -> Result<String, String> {
    let mut input = history.to_vec();
    input.push(json!({"role": "user", "content": SUMMARY_PROMPT}));
    loop {
        let result = llm_runner::run_llm_step_with_retry(
            session_id,
            &input,
            &[],
            Some(model.to_string()),
            &json!({"disable_tools": true}),
            &[],
            &Default::default(),
            &mut |_| true,
            reliability.provider_retries,
            reliability.provider_backoff_ms,
            cancellation,
        );
        match result {
            Ok(output) => {
                if !output.tool_calls.is_empty() {
                    return Err("internal compaction rejected a model tool call".into());
                }
                if output.finish_reason.as_deref() == Some("length") {
                    return Err("internal compaction summary was truncated by the model".into());
                }
                let summary = output.text.trim();
                if summary.is_empty() {
                    return Err("internal compaction generated an empty summary".into());
                }
                return Ok(summary.to_string());
            }
            Err(error) if llm_runner::is_prompt_too_long_error(&error) && input.len() > 2 => {
                // Like Codex, trim only the request copy after an actual overflow. Never
                // delete persisted history or leave orphan tool results at the new head.
                input.remove(0);
                while input.len() > 1 && input[0]["role"] == "tool" {
                    input.remove(0);
                }
                if input.len() == 1 {
                    return Err(error);
                }
            }
            Err(error) if error == "模型未返回内容" => {
                return Err("internal compaction generated an empty summary".into());
            }
            Err(error) => return Err(error),
        }
    }
}

/// Adapt async callers without blocking the executor. Dropping the caller cancels
/// the shared runner, including stream collection and retry backoff.
pub(crate) async fn summarize_async(
    session_id: String,
    history: Vec<Value>,
    model: String,
    reliability: ReliabilityConfig,
) -> Result<String, String> {
    let cancellation = CancellationToken::new();
    let _cancel_on_drop = cancellation.clone().drop_guard();
    let tenant = crate::session::ui_store::current_tenant_scope();
    let project =
        crate::project::instance::project().map(|project| crate::project::instance::Context {
            directory: crate::project::instance::directory(),
            worktree: crate::project::instance::worktree(),
            project,
        });
    tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| format!("compaction runtime failed: {error}"))?;
        runtime.block_on(crate::session::ui_store::with_tenant_scope(tenant, async {
            let summarize = async {
                summarize(&session_id, &history, &model, &reliability, Some(&cancellation))
            };
            match project {
                Some(context) => {
                    crate::project::instance::provide_context(context, summarize).await
                }
                None => summarize.await,
            }
        }))
    })
    .await
    .map_err(|error| format!("compaction worker failed: {error}"))?
}

#[cfg(test)]
#[path = "compaction_summary_tests.rs"]
mod tests;
