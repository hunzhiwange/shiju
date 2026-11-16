//! Budgeted context summaries with bounded overflow recovery and the normal request lifecycle.

use crate::config::ReliabilityConfig;
use crate::provider::provider::ModelLimit;

use crate::session::processor::llm_runner;
use serde_json::Value;
#[cfg(test)]
use serde_json::json;
use tokio_util::sync::CancellationToken;

// Request fitting is owned by session-engine.
#[cfg(test)]
use session_engine::compaction_summary::{request_input, MAX_OVERFLOW_RECOVERIES};

struct SummaryHost;
impl session_engine::compaction_summary::SummaryHost for SummaryHost {
    fn run_llm_step(
        session_id: &str, messages: &[Value], system: &[String], model: Option<String>,
        options: &Value, available_tools: &[crate::tools::ToolSpec],
        allowed_tools: &std::collections::HashSet<String>,
        on_event: &mut dyn FnMut(crate::session::processor::StreamEvent) -> bool,
        cancellation: Option<&CancellationToken>,
    ) -> Result<llm_runner::LlmStep, String> {
        llm_runner::run_llm_step(session_id, messages, system, model, options,
            available_tools, allowed_tools, on_event, cancellation)
    }
}
pub(crate) fn summarize(
    session_id: &str, history: &[Value], model: &str,
    reliability: &ReliabilityConfig, cancellation: Option<&CancellationToken>, limit: &ModelLimit,
) -> Result<String, String> {
    session_engine::compaction_summary::summarize::<SummaryHost>(
        session_id, history, model, reliability, cancellation, limit,
    )
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
