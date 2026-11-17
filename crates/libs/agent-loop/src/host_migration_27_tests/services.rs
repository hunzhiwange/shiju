//! Bind scripted external services to the real loop, LLM parser and tool executor.
use super::host::DomainHost;
use crate::core::{llm_round::LlmServices, tool_execution::{CallView, OutcomeView, ToolBatch, ToolExecutionServices}, tool_loop::LoopServices};
use crate::parsing::ParsedToolCall;
use tool_services::scheduler::{PendingToolCall, ScheduledToolBatchMode};
use crate::execution::ToolExecutionOutcome;
use std::{sync::Arc, time::Duration};
use model_types::{ChatMessage, Provider};
use tool_types::Tool;
use agent_observability::Observer;
use agent_approval::ApprovalManager;
use agent_hooks::HookRunner;
use agent_security::SecurityPolicy;
use tokio_util::sync::CancellationToken;
use anyhow::Result;

pub(super) fn services() -> LoopServices<DomainHost, ParsedToolCall> {
    LoopServices {
        wire: crate::core::output::ProgressWire { clear: crate::progress::DRAFT_CLEAR_SENTINEL, progress: crate::progress::DRAFT_PROGRESS_SENTINEL },
        non_cli_approval_context: || crate::approval::TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT.try_with(Clone::clone).ok().flatten(),
        reply_target: || crate::core::turn::TOOL_LOOP_REPLY_TARGET.try_with(Clone::clone).ok().flatten(),
        new_discovery: |specs| specs,
        rebase_discovery: |_, specs| specs,
        exposed_specs: Clone::clone,
        refresh_catalog: |_, _, _| Box::pin(async { anyhow::bail!("catalog refresh is outside scripted loop tests") }),
        new_extractor: || Box::new(NoImageExtractor),
        exposed_instructions: crate::instructions::build_exposed_tool_instructions,
        scrub_credentials: agent_util::credentials::scrub_credentials,
        llm_round: |r| Box::pin(async move {
            crate::core::llm_round::run_llm_round_with_extractor(
                &llm_services(), r.provider, r.history, r.observer, r.provider_name,
                r.model, r.temperature, r.channel_name, r.multimodal_config,
                r.cancellation_token, r.hooks, r.tool_specs, r.use_native_tools,
                r.turn_id, r.iteration, &NoImageExtractor,
            ).await
        }),
        execute_tools: |r| Box::pin(async move {
            crate::core::tool_execution::execute_tool_calls_and_update_history(
                &tool_services(), r.history, r.tool_calls, r.assistant_history_content,
                r.native_tool_calls, r.tools_registry, r.observer, r.channel_name,
                r.channel_reply_target, r.provider_name, r.model, r.turn_id, r.iteration,
                r.tool_use_context, r.on_delta, r.excluded_tools,
                r.bypass_non_cli_approval_for_turn, r.cancellation_token,
                r.seen_tool_signatures, r.use_native_tools,
            ).await
        }),
        stop_hook: |_, _, _, _| Box::pin(async { Ok(Default::default()) }),
        cancel_descendants: |_, _| Ok(0),
        descendant_snapshots: |_, _| Ok(vec![]),
        wait_for_descendants: |_, _, _| Box::pin(async { Ok(session_port::child_agent::ChildAgentWaitOutcome { timed_out: false, tasks: vec![] }) }),
        deliver_model_events: |_, _| Ok(false),
    }
}

fn llm_services() -> LlmServices<ParsedToolCall> {
    LlmServices {
        parse_structured: crate::parsing::parse_structured_tool_calls,
        parse_text: crate::parsing::parse_tool_calls,
        detect_issue: crate::parsing::detect_tool_call_parse_issue,
        call_id_mut: |call| &mut call.tool_call_id,
        history_parsed: super::build_native_assistant_history_from_parsed_calls,
        history_native: crate::core::history::build_native_assistant_history,
        scrub_credentials: agent_util::credentials::scrub_credentials,
        token_usage: plugin_hooks::hook_model::token_usage,
        emit_model: |_, _| Box::pin(async {}),
    }
}

fn tool_services() -> ToolExecutionServices<DomainHost, ParsedToolCall, PendingToolCall, ToolExecutionOutcome, ()> {
    ToolExecutionServices {
        call_view: |call| CallView { name: &call.name, arguments: &call.arguments, tool_call_id: call.tool_call_id.as_deref() },
        pending_view: |call| CallView { name: &call.name, arguments: &call.arguments, tool_call_id: call.tool_call_id.as_deref() },
        pending_call: |name, arguments, tool_call_id| PendingToolCall { name, arguments, tool_call_id },
        schedule: |calls, tools| tool_services::scheduler::schedule_tool_calls(calls, tools).into_iter().map(|batch| ToolBatch { parallel: batch.mode == ScheduledToolBatchMode::Parallel, calls: batch.calls }).collect(),
        outcome_view: |o| OutcomeView { tool_name: &o.tool_name, output: &o.output, success: o.success, error_reason: o.error_reason.as_deref(), result_dto: o.result_dto.as_ref(), duration: o.duration },
        take_output: |o| o.output,
        failure: |tool_name, output| ToolExecutionOutcome { tool_name, error_reason: Some(output.clone()), output, success: false, result_dto: None, duration: Duration::ZERO },
        execute_parallel: |calls, tools, observer, context, token| Box::pin(crate::execution::execute_tools_parallel(calls, tools, observer, context, token)),
        execute_sequential: |calls, tools, observer, context, token| Box::pin(crate::execution::execute_tools_sequential(calls, tools, observer, context, token)),
        inject_cron_delivery: crate::cron::maybe_inject_cron_add_delivery,
        call_signature: crate::parsing::json::tool_call_signature,
        scrub_credentials: agent_util::credentials::scrub_credentials,
        progress_actions: crate::progress::tool_progress_actions,
        progress_label: crate::progress::tool_progress_label,
        progress_sentinel: crate::progress::DRAFT_PROGRESS_SENTINEL,
        ws_event_sentinel: crate::progress::DRAFT_WS_EVENT_SENTINEL,
        serialize_native: |_| anyhow::bail!("native search is outside scripted loop tests"),
        #[cfg(not(target_arch = "wasm32"))]
        native_search_history: |_, _| anyhow::bail!("native search is outside scripted loop tests"),
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_tool_call_loop(
    provider: &dyn Provider,
    history: &mut Vec<ChatMessage>,
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    silent: bool,
    approval: Option<Arc<ApprovalManager>>,
    channel_name: &str,
    multimodal_config: &config_tools::MultimodalConfig,
    max_iterations: usize,
    cancellation_token: Option<CancellationToken>,
    on_delta: Option<tokio::sync::mpsc::Sender<String>>,
    hooks: Option<Arc<HookRunner>>,
    security: Option<Arc<SecurityPolicy>>,
    excluded_tools: &[String],
) -> Result<String> {
    crate::core::tool_loop::run_tool_call_loop_with_skills::<DomainHost, _, _>(
        &services(),
        provider,
        history,
        tools_registry,
        observer,
        provider_name,
        model,
        temperature,
        silent,
        approval,
        channel_name,
        multimodal_config,
        max_iterations,
        cancellation_token,
        on_delta,
        hooks,
        security,
        excluded_tools,
        None,
    )
    .await
}


#[allow(clippy::too_many_arguments)]
pub(super) async fn agent_turn(
    provider: &dyn Provider,
    history: &mut Vec<ChatMessage>,
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    silent: bool,
    multimodal_config: &config_tools::MultimodalConfig,
    max_iterations: usize,
    hooks: Option<Arc<HookRunner>>,
    skills: Option<&mut crate::core::skills::CoreSkillSession<DomainHost>>,
) -> Result<String> {
    crate::core::turn::agent_turn::<DomainHost, _, _>(
        &services(), provider, history, tools_registry, observer,
        provider_name, model, temperature, silent, multimodal_config,
        max_iterations, hooks, skills,
    ).await
}


use crate::approval::NonCliApprovalContext;
#[allow(clippy::too_many_arguments)]
pub(super) async fn run_tool_call_loop_with_non_cli_approval_context(
    provider: &dyn Provider,
    history: &mut Vec<ChatMessage>,
    tools_registry: &[Box<dyn Tool>],
    observer: &dyn Observer,
    provider_name: &str,
    model: &str,
    temperature: f64,
    silent: bool,
    approval: Option<Arc<ApprovalManager>>,
    channel_name: &str,
    non_cli_approval_context: Option<NonCliApprovalContext>,
    multimodal_config: &config_tools::MultimodalConfig,
    max_iterations: usize,
    cancellation_token: Option<CancellationToken>,
    on_delta: Option<tokio::sync::mpsc::Sender<String>>,
    hooks: Option<Arc<HookRunner>>,
    security: Option<Arc<SecurityPolicy>>,
    excluded_tools: &[String],
) -> Result<String> {
    agent_loop::core::turn::run_tool_call_loop_with_non_cli_approval_context::<DomainHost, _, _>(
        &services(), provider, history, tools_registry, observer,
        provider_name, model, temperature, silent, approval, channel_name, non_cli_approval_context,
        multimodal_config, max_iterations, cancellation_token, on_delta, hooks, security,
        excluded_tools,
    ).await
}

pub(crate) struct NoImageExtractor;
#[async_trait::async_trait]
impl agent_multimodal::ocr::ImageTextExtractor for NoImageExtractor {
 async fn supports_image_input(&self, _: &str) -> anyhow::Result<bool> { anyhow::bail!("scripted loop has no image model catalog") }
 async fn extract_text(&self, _: &str, _: &str) -> anyhow::Result<String> { anyhow::bail!("scripted loop has no OCR service") }
}
