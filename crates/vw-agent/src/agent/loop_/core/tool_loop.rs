//! # 工具调用循环核心模块
//!
//! 本模块保留工具循环的对外入口，并将 LLM 轮次处理、进度输出、
//! 工具执行和完成声明检测拆分到 `tool_loop/` 子模块，降低单文件职责密度。

mod detection;
mod llm_round;
mod output;
mod tool_execution;

#[cfg(test)]
#[path = "tool_loop/discovery_tests.rs"]
mod discovery_tests;

#[cfg(test)]
#[path = "tool_loop_tests.rs"]
mod tool_loop_tests;

pub(crate) use detection::looks_like_unverified_action_completion_without_tool_call;

use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::hooks::HookRunner;
use crate::app::agent::observability::{Observer, runtime_trace};
use crate::app::agent::providers::{ChatMessage, Provider};
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::{Tool, ToolUseContext};
use crate::app::agent::util::truncate_with_ellipsis;
use crate::tools::tool_search::ToolDiscoverySession;
use anyhow::Result;
use std::collections::HashSet;
use std::io::Write as _;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use super::super::approval::TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT;
use super::super::utils::scrub_credentials;
use super::TOOL_LOOP_REPLY_TARGET;
use super::constants::{
    DEFAULT_MAX_ITERATIONS, EMPTY_FINAL_RESPONSE_RETRY_PROMPT, MISSING_TOOL_CALL_RETRY_PROMPT,
};
use super::errors::ToolLoopCancelled;
use llm_round::run_llm_round;
use output::{
    send_retry_progress, stream_final_response, update_thinking_progress, update_tool_call_progress,
};
use tool_execution::execute_tool_calls_and_update_history;

/// 执行代理工具调用循环的核心函数。
#[allow(clippy::too_many_arguments)]
pub async fn run_tool_call_loop(
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
    multimodal_config: &crate::app::agent::config::MultimodalConfig,
    max_iterations: usize,
    cancellation_token: Option<CancellationToken>,
    on_delta: Option<tokio::sync::mpsc::Sender<String>>,
    hooks: Option<Arc<HookRunner>>,
    security: Option<Arc<SecurityPolicy>>,
    excluded_tools: &[String],
) -> Result<String> {
    run_tool_call_loop_with_skills(
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
pub(crate) async fn run_tool_call_loop_with_skills(
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
    multimodal_config: &crate::app::agent::config::MultimodalConfig,
    max_iterations: usize,
    cancellation_token: Option<CancellationToken>,
    on_delta: Option<tokio::sync::mpsc::Sender<String>>,
    hooks: Option<Arc<HookRunner>>,
    security: Option<Arc<SecurityPolicy>>,
    excluded_tools: &[String],
    mut skills: Option<&mut super::skills::CoreSkillSession>,
) -> Result<String> {
    let non_cli_approval_context =
        TOOL_LOOP_NON_CLI_APPROVAL_CONTEXT.try_with(Clone::clone).ok().flatten();
    let channel_reply_target = TOOL_LOOP_REPLY_TARGET
        .try_with(Clone::clone)
        .ok()
        .flatten()
        .or_else(|| non_cli_approval_context.as_ref().map(|ctx| ctx.reply_target.clone()));
    let max_iterations = if max_iterations == 0 { DEFAULT_MAX_ITERATIONS } else { max_iterations };
    let initial_history_len = history.len();
    let mut authorized_specs: Vec<crate::app::agent::tools::ToolSpec> = tools_registry
        .iter()
        .filter(|tool| {
            let tool_id = tool.spec().id;
            !excluded_tools.iter().any(|excluded| excluded == &tool_id)
        })
        .map(|tool| tool.spec())
        .collect();
    let use_native_tools = provider.supports_native_tools() && !authorized_specs.is_empty();
    let turn_id = Uuid::new_v4().to_string();
    let mut seen_tool_signatures: HashSet<(String, String)> = HashSet::new();
    let mut missing_tool_call_retry_used = false;
    let mut missing_tool_call_retry_prompt: Option<String> = None;
    let bypass_non_cli_approval_for_turn = approval
        .as_ref()
        .is_some_and(|mgr| channel_name != "cli" && mgr.consume_non_cli_allow_all_once());
    if bypass_non_cli_approval_for_turn {
        runtime_trace::record_event(
            "approval_bypass_one_time_all_tools_consumed",
            Some(channel_name),
            Some(provider_name),
            Some(model),
            Some(&turn_id),
            Some(true),
            Some("consumed one-time non-cli allow-all approval token"),
            serde_json::json!({}),
        );
    }

    let mut tool_use_context = ToolUseContext::new(format!("tool-loop-{turn_id}"), None)
        .with_channel(channel_name.to_string())
        .with_provider(provider_name.to_string())
        .with_model(model.to_string())
        .with_turn_id(turn_id.clone())
        .with_bypass_non_cli_approval_for_turn(bypass_non_cli_approval_for_turn);
    if let Some(security) = security.clone() {
        tool_use_context = tool_use_context.with_security(security);
    }
    if let Some(approval) = approval.clone() {
        tool_use_context = tool_use_context.with_approval(approval);
    }
    if let Some(hook_runner) = hooks.clone() {
        tool_use_context = tool_use_context.with_hook_runner(hook_runner);
    }
    if let Some(progress_tx) = on_delta.clone() {
        tool_use_context = tool_use_context.with_progress_tx(progress_tx);
    }
    if let Some(token) = cancellation_token.clone() {
        tool_use_context = tool_use_context.with_abort_token(token);
    }
    if let Some(non_cli) = non_cli_approval_context.clone() {
        tool_use_context = tool_use_context.with_non_cli_approval_context(non_cli);
    }
    if let Some(skills) = skills.as_deref_mut() {
        tool_use_context = Box::pin(skills.configure(tool_use_context)).await?;
        skills.refresh_skill_spec(&mut authorized_specs);
    }
    let discovery = Arc::new(ToolDiscoverySession::new(authorized_specs));
    tool_use_context = tool_use_context.with_tool_discovery(discovery.clone());
    let mut image_turn = prepare_legacy_image_turn(history, model, multimodal_config).await?;
    if let Some(ctx) = image_turn.as_mut() {
        tool_use_context.set_image_asset_scope(&ctx.scope);
        ctx.rewrite_chat_messages(history);
    }
    let tool_use_context = Arc::new(tool_use_context);

    for iteration in 0..max_iterations {
        if cancellation_token.as_ref().is_some_and(CancellationToken::is_cancelled) {
            return Err(ToolLoopCancelled.into());
        }

        if let Some(retry_prompt) = missing_tool_call_retry_prompt.take() {
            history.push(ChatMessage::user(retry_prompt));
        }

        update_thinking_progress(on_delta.as_ref(), iteration).await;

        // Search can change the next request, never another call in this round.
        let tool_specs = discovery.exposed_specs();
        let round_tool_use_context = Arc::new(
            tool_use_context
                .as_ref()
                .clone()
                .with_tool_exposure_snapshot(tool_specs.iter().map(|spec| spec.id.clone())),
        );

        let pending_reloads = tool_use_context.peek_pending_image_reloads();
        if let Some(ctx) = image_turn.as_ref() {
            ctx.attach_pending_reloads_to_chat(history, &pending_reloads);
        }

        let skill_messages = match skills.as_deref_mut() {
            Some(skills) => skills.messages(tool_specs.iter().any(|spec| spec.id == "skill"))?,
            None => Vec::new(),
        };
        let request_history =
            if !skill_messages.is_empty() || (!use_native_tools && !tool_specs.is_empty()) {
                let mut request_history = history.clone();
                // Keep the entry point's system reminder first; skill messages remain
                // request-local and never enter history or tool-result persistence.
                let offset =
                    usize::from(request_history.first().is_some_and(is_transient_protocol_message));
                request_history.splice(offset..offset, skill_messages);
                if !use_native_tools && !tool_specs.is_empty() {
                    let instructions =
                        super::super::instructions::build_exposed_tool_instructions(&tool_specs);
                    request_history.insert(
                        0,
                        ChatMessage::user(format!(
                            "<system-reminder>\n{instructions}\n</system-reminder>"
                        )),
                    );
                }
                Some(request_history)
            } else {
                None
            };
        let round = run_llm_round(
            provider,
            request_history.as_deref().unwrap_or(history.as_slice()),
            observer,
            provider_name,
            model,
            temperature,
            channel_name,
            multimodal_config,
            cancellation_token.as_ref(),
            hooks.as_deref(),
            &tool_specs,
            use_native_tools,
            &turn_id,
            iteration,
        )
        .await?;

        if let Some(ctx) = image_turn.as_mut() {
            if !pending_reloads.is_empty() {
                tool_use_context.clear_pending_image_reloads();
            }
            if !ctx.has_first_success() || !pending_reloads.is_empty() {
                ctx.mark_first_success();
                ctx.rewrite_chat_messages(history);
            }
        }

        update_tool_call_progress(on_delta.as_ref(), round.tool_calls.len(), round.duration_secs)
            .await;

        if round.tool_calls.is_empty() {
            if round.display_text.trim().is_empty() {
                runtime_trace::record_event(
                    "empty_final_response_retry",
                    Some(channel_name),
                    Some(provider_name),
                    Some(model),
                    Some(&turn_id),
                    Some(false),
                    Some("model returned no tool calls or final text"),
                    serde_json::json!({
                        "iteration": iteration + 1,
                        "will_retry": iteration + 1 < max_iterations,
                    }),
                );
                if iteration + 1 < max_iterations {
                    history.push(ChatMessage::user(EMPTY_FINAL_RESPONSE_RETRY_PROMPT));
                    continue;
                }
                anyhow::bail!(
                    "Agent did not produce a non-empty final response within {max_iterations} iterations"
                );
            }

            let completion_claim_signal =
                looks_like_unverified_action_completion_without_tool_call(&round.display_text);
            let missing_tool_call_signal = round.parse_issue_detected || completion_claim_signal;
            let should_retry = !missing_tool_call_retry_used
                && iteration + 1 < max_iterations
                && !tool_specs.is_empty()
                && missing_tool_call_signal;

            if should_retry {
                missing_tool_call_retry_used = true;
                missing_tool_call_retry_prompt = Some(MISSING_TOOL_CALL_RETRY_PROMPT.to_string());
                let retry_reason = if round.parse_issue_detected {
                    "parse_issue_detected"
                } else {
                    "completion_claim_text_detected"
                };
                runtime_trace::record_event(
                    "tool_call_followthrough_retry",
                    Some(channel_name),
                    Some(provider_name),
                    Some(model),
                    Some(&turn_id),
                    Some(false),
                    Some(retry_reason),
                    serde_json::json!({
                        "iteration": iteration + 1,
                        "response_excerpt": truncate_with_ellipsis(
                            &scrub_credentials(&round.display_text),
                            240
                        ),
                    }),
                );
                send_retry_progress(on_delta.as_ref()).await;
                continue;
            }

            if missing_tool_call_signal && missing_tool_call_retry_used {
                runtime_trace::record_event(
                    "tool_call_followthrough_failed",
                    Some(channel_name),
                    Some(provider_name),
                    Some(model),
                    Some(&turn_id),
                    Some(false),
                    Some("model repeated deferred action without tool call"),
                    serde_json::json!({
                        "iteration": iteration + 1,
                        "response_excerpt": truncate_with_ellipsis(
                            &scrub_credentials(&round.display_text),
                            240
                        ),
                    }),
                );
                anyhow::bail!("Model repeatedly deferred action without emitting a tool call");
            }

            runtime_trace::record_event(
                "turn_final_response",
                Some(channel_name),
                Some(provider_name),
                Some(model),
                Some(&turn_id),
                Some(true),
                None,
                serde_json::json!({
                    "iteration": iteration + 1,
                    "text": scrub_credentials(&round.display_text),
                }),
            );
            stream_final_response(
                &round.display_text,
                on_delta.as_ref(),
                cancellation_token.as_ref(),
            )
            .await?;
            history.push(ChatMessage::assistant(round.response_text));
            return Ok(round.display_text);
        }

        if !silent && !round.display_text.is_empty() {
            print!("{}", round.display_text);
            let _ = std::io::stdout().flush();
        }

        execute_tool_calls_and_update_history(
            history,
            &round.tool_calls,
            round.assistant_history_content,
            &round.native_tool_calls,
            tools_registry,
            observer,
            channel_name,
            channel_reply_target.as_deref(),
            provider_name,
            model,
            &turn_id,
            iteration,
            &round_tool_use_context,
            on_delta.as_ref(),
            excluded_tools,
            bypass_non_cli_approval_for_turn,
            cancellation_token.as_ref(),
            &mut seen_tool_signatures,
            use_native_tools,
        )
        .await?;
        if let Some(skills) = skills.as_deref_mut() {
            Box::pin(skills.consume(tool_use_context.as_ref(), history, initial_history_len))
                .await?;
        }
    }

    runtime_trace::record_event(
        "tool_loop_exhausted",
        Some(channel_name),
        Some(provider_name),
        Some(model),
        Some(&turn_id),
        Some(false),
        Some("agent exceeded maximum tool iterations"),
        serde_json::json!({
            "max_iterations": max_iterations,
        }),
    );
    anyhow::bail!("Agent exceeded maximum tool iterations ({max_iterations})")
}

async fn prepare_legacy_image_turn(
    history: &[ChatMessage],
    model: &str,
    multimodal_config: &crate::app::agent::config::MultimodalConfig,
) -> Result<Option<crate::app::agent::multimodal::context::ImageTurnContext>> {
    use crate::app::agent::multimodal::assets::{ImageAssetScope, ImageAssetStore};
    use crate::app::agent::multimodal::context::ImageTurnContext;
    use crate::app::agent::multimodal::ocr::SessionImageTextExtractor;
    use crate::app::agent::multimodal::parse_image_markers;

    if !history.iter().filter(|message| !is_transient_protocol_message(message)).any(|message| {
        message.role == "user" && !parse_image_markers(&message.content).1.is_empty()
    }) {
        return Ok(None);
    }

    let mut prior = history.to_vec();
    let current = prior
        .iter()
        .rposition(|message| {
            !is_transient_protocol_message(message)
                && message.role == "user"
                && !parse_image_markers(&message.content).1.is_empty()
        })
        .map(|index| prior.remove(index).content);

    let scope = ImageAssetScope::from_request_context(None, "legacy-tool-loop")?;
    let store = ImageAssetStore::from_global_paths();
    let extractor = SessionImageTextExtractor::new();
    let ctx = ImageTurnContext::build(
        &store,
        scope,
        &prior,
        current.as_deref(),
        false,
        multimodal_config,
        Some(model),
        &extractor,
    )
    .await?;
    if ctx.images.is_empty() { Ok(None) } else { Ok(Some(ctx)) }
}

fn is_transient_protocol_message(message: &ChatMessage) -> bool {
    message.content.starts_with("<system-reminder>")
}
