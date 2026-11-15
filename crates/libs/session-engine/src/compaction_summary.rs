//! Budgeted one-shot compaction summaries using the original model retry lifecycle.
use agent_config::runtime::ReliabilityConfig;
use model_types::ModelLimit;
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
#[path = "compaction_summary_input.rs"]
#[doc(hidden)]
pub mod request_input;

pub trait SummaryHost {
    fn run_llm_step(
        session_id: &str, messages: &[Value], system: &[String], model: Option<String>,
        options: &Value, available_tools: &[tool_types::ToolSpec],
        allowed_tools: &std::collections::HashSet<String>,
        on_event: &mut dyn FnMut(crate::processor_events::StreamEvent) -> bool,
        cancellation: Option<&CancellationToken>,
    ) -> Result<crate::llm_step::LlmStep, String>;
}
fn run_llm_step_with_retry<H: SummaryHost>(
    session_id: &str, messages: &[Value], system: &[String], model: Option<String>,
    options: &Value, available_tools: &[tool_types::ToolSpec],
    allowed_tools: &std::collections::HashSet<String>,
    on_event: &mut dyn FnMut(crate::processor_events::StreamEvent) -> bool,
    additional_retries: u32, initial_backoff_ms: u64,
    cancellation: Option<&CancellationToken>,
) -> Result<crate::llm_step::LlmStep, String> {
    crate::recovery::RetryBudget::new(additional_retries, initial_backoff_ms).run(
        false, cancellation, on_event, |sink| H::run_llm_step(
            session_id, messages, system, model.clone(), options, available_tools,
            allowed_tools, sink, cancellation,
        ),
    )
}

#[doc(hidden)]
pub const MAX_OVERFLOW_RECOVERIES: usize = 6;

const SUMMARY_PROMPT: &str = "Create a concise context checkpoint for another model to continue this conversation. Preserve progress, key decisions, user constraints, unfinished work, next steps, and essential references. Return only the handoff summary. Do not execute tools or continue the task.";

pub fn summarize<H: SummaryHost>(
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
    // The fallback is a local scheduling budget. Actual provider overflows still
    // shrink only this request copy, including when a custom model is smaller.
    let mut budget = request_input::input_budget(limit);
    let output_budget = crate::compaction_budget::summary_output_budget(limit);
    let mut omitted = request_input::fit(&mut input, budget)?;
    let mut recoveries = 0;
    loop {
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            return Err("执行已取消".into());
        }
        let result = run_llm_step_with_retry::<H>(
            session_id,
            &input,
            &[],
            Some(model.to_string()),
            &json!({"disable_tools": true, "max_output_tokens": output_budget}),
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
            Err(error) if crate::request_overflow::is_request_too_long_error(&error, &input) => {
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
#[doc(hidden)]
pub fn summary_error(error: String) -> String {
    const EMPTY_SUMMARY: &str = "internal compaction generated an empty summary";
    if error == "模型未返回内容" {
        return EMPTY_SUMMARY.into();
    }
    let Ok(mut structured) =
        serde_json::from_str::<message_types::AssistantError>(&error)
    else {
        return error;
    };
    match &mut structured {
        message_types::AssistantError::APIError { message, .. }
        | message_types::AssistantError::Unknown { message }
            if message == "模型未返回内容" =>
        {
            *message = EMPTY_SUMMARY.into();
        }
        _ => return error,
    }
    serde_json::to_string(&structured).unwrap_or(error)
}
