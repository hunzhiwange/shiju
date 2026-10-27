//! Budgeted context summaries with bounded overflow recovery and the normal request lifecycle.

use crate::config::ReliabilityConfig;
use crate::provider::provider::ModelLimit;

use crate::session::processor::llm_runner;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

#[path = "compaction_summary_input.rs"]
mod request_input;

const MAX_OVERFLOW_RECOVERIES: usize = 6;

const SUMMARY_PROMPT: &str = "Create a concise context checkpoint for another model to continue this conversation. Preserve progress, key decisions, user constraints, unfinished work, next steps, and essential references. Return only the handoff summary. Do not execute tools or continue the task.";

pub(crate) fn summarize(
    session_id: &str,
    history: &[Value],
    model: &str,
    reliability: &ReliabilityConfig,
    cancellation: Option<&CancellationToken>,
    limit: &ModelLimit,
) -> Result<String, String> {
    if cancellation.is_some_and(CancellationToken::is_cancelled) {
        return Err("执行已取消".into());
    }
    if history.is_empty() {
        return Err("internal compaction has no history to summarize".into());
    }
    let mut input = history.to_vec();
    input.push(json!({"role": "user", "content": SUMMARY_PROMPT}));
    // Some legacy providers have no catalog limit. Preserve their request and
    // learn by shrinking after an actual overflow instead of inventing a window.
    let mut budget = if limit.context == 0 && limit.input.is_none_or(|input| input == 0) {
        request_input::estimated_tokens(&input)
    } else {
        request_input::input_budget(limit)
    };
    let mut omitted = request_input::fit(&mut input, budget)?;
    let mut recoveries = 0;
    loop {
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            return Err("执行已取消".into());
        }
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
                return Ok(if omitted {
                    format!(
                        "[部分历史或工具输出为适配输入限制已省略，原始内容仍保留在会话记录中]\n{summary}"
                    )
                } else {
                    summary.to_string()
                });
            }
            Err(error) if llm_runner::is_request_too_long_error(&error, &input) => {
                if recoveries >= MAX_OVERFLOW_RECOVERIES {
                    return Err(format!(
                        "压缩超限恢复次数已用尽；原始会话和已有检查点已保留：{error}"
                    ));
                }
                // Shrink geometrically, including a single giant message. Never resend
                // the same rejected payload or mutate the checkpoint source.
                budget = budget.min(request_input::estimated_tokens(&input)) / 2;
                omitted |= request_input::recover(&mut input, budget)?;
                recoveries += 1;
            }
            Err(error) => return Err(summary_error(error)),
        }
    }
}

/// The shared runner can wrap empty output to forbid replay after streamed
/// whitespace/reasoning. Translate the domain error without losing that metadata.
fn summary_error(error: String) -> String {
    const EMPTY_SUMMARY: &str = "internal compaction generated an empty summary";
    if error == "模型未返回内容" {
        return EMPTY_SUMMARY.into();
    }
    let Ok(mut structured) =
        serde_json::from_str::<crate::session::message::AssistantError>(&error)
    else {
        return error;
    };
    match &mut structured {
        crate::session::message::AssistantError::APIError { message, .. }
        | crate::session::message::AssistantError::Unknown { message }
            if message == "模型未返回内容" =>
        {
            *message = EMPTY_SUMMARY.into();
        }
        _ => return error,
    }
    serde_json::to_string(&structured).unwrap_or(error)
}

/// Adapt async callers without blocking the executor. Dropping the caller cancels
/// the shared runner, including stream collection and retry backoff.
pub(crate) async fn summarize_async(
    session_id: String,
    history: Vec<Value>,
    model: String,
    reliability: ReliabilityConfig,
    limit: ModelLimit,
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
                summarize(&session_id, &history, &model, &reliability, Some(&cancellation), &limit)
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
