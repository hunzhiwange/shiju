//! 工具循环主体；实时目录、租约与请求宿主能力按实际调用点绑定。
use std::{sync::Arc, collections::HashSet, path::Path, io::Write as _};
use anyhow::Result;
use model_types::{Provider, ChatMessage, ToolCall};
use agent_approval::ApprovalManager;
use agent_hooks::{HookRunner, TurnEndOutcome};
use agent_security::SecurityPolicy;
use agent_observability::{Observer, runtime_trace};
use agent_util::truncate_with_ellipsis;
use tool_types::{Tool, ToolSpec};
use tool_context::{context::ToolUseContext, host::ContextHost, approval::NonCliApprovalContext};
use tool_assembly::catalog::{ToolCatalog, ToolCatalogSnapshot};
use tokio::{sync::mpsc::Sender};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use agent_multimodal::ocr::ImageTextExtractor;
use super::constants::{DEFAULT_MAX_ITERATIONS, EMPTY_FINAL_RESPONSE_RETRY_PROMPT, MISSING_TOOL_CALL_RETRY_PROMPT};
use super::errors::ToolLoopCancelled;
use super::detection::looks_like_unverified_action_completion_without_tool_call;
use super::output::{ProgressWire, send_retry_progress, stream_final_response, update_thinking_progress, update_tool_call_progress};
use super::llm_round::LlmRoundResult;
use session_port::child_agent::{ChildAgentTaskSnapshot, ChildAgentWaitOutcome};
pub struct RoundRequest<'a> {
    pub provider: &'a dyn Provider,
    pub history: &'a [ChatMessage],
    pub observer: &'a dyn Observer,
    pub provider_name: &'a str,
    pub model: &'a str,
    pub temperature: f64,
    pub channel_name: &'a str,
    pub multimodal_config: &'a config_tools::MultimodalConfig,
    pub cancellation_token: Option<&'a CancellationToken>,
    pub hooks: Option<&'a agent_hooks::HookRunner>,
    pub tool_specs: &'a [ToolSpec],
    pub use_native_tools: bool,
    pub turn_id: &'a str,
    pub iteration: usize,
}
pub struct ToolRoundRequest<'a, C: ContextHost, T> {
    pub history: &'a mut Vec<ChatMessage>,
    pub tool_calls: &'a [T],
    pub assistant_history_content: String,
    pub native_tool_calls: &'a [ToolCall],
    pub tools_registry: &'a [Box<dyn Tool>],
    pub observer: &'a dyn Observer,
    pub channel_name: &'a str,
    pub channel_reply_target: Option<&'a str>,
    pub provider_name: &'a str,
    pub model: &'a str,
    pub turn_id: &'a str,
    pub iteration: usize,
    pub tool_use_context: &'a Arc<ToolUseContext<C>>,
    pub on_delta: Option<&'a Sender<String>>,
    pub excluded_tools: &'a [String],
    pub bypass_non_cli_approval_for_turn: bool,
    pub cancellation_token: Option<&'a CancellationToken>,
    pub seen_tool_signatures: &'a mut HashSet<(String, String)>,
    pub use_native_tools: bool,
}

#[cfg(not(target_arch = "wasm32"))]
pub type LoopFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + Send + 'a>>;
#[cfg(target_arch = "wasm32")]
pub type LoopFuture<'a, T> = std::pin::Pin<Box<dyn std::future::Future<Output = T> + 'a>>;
/// 每项函数均对应一个已有真实动作，不持有副本状态或调度整轮业务。
pub struct LoopServices<C: ContextHost, T> {
    pub wire: ProgressWire,
    pub non_cli_approval_context: fn() -> Option<NonCliApprovalContext>,
    pub reply_target: fn() -> Option<String>,
    pub new_discovery: fn(Vec<ToolSpec>) -> C::ToolDiscoverySession,
    pub rebase_discovery: fn(&C::ToolDiscoverySession, Vec<ToolSpec>) -> C::ToolDiscoverySession,
    pub exposed_specs: fn(&C::ToolDiscoverySession) -> Vec<ToolSpec>,
    pub refresh_catalog: for<'a> fn(&'a str, &'a Path, &'a Arc<ToolCatalog>) -> LoopFuture<'a, Result<()>>,
    pub new_extractor: fn() -> Box<dyn ImageTextExtractor>,
    pub exposed_instructions: fn(&[ToolSpec]) -> String,
    pub scrub_credentials: fn(&str) -> String,
    pub llm_round: for<'a> fn(RoundRequest<'a>) -> LoopFuture<'a, Result<LlmRoundResult<T>>>,
    pub execute_tools: for<'a> fn(ToolRoundRequest<'a, C, T>) -> LoopFuture<'a, Result<()>>,
    pub stop_hook: for<'a> fn(Arc<ToolUseContext<C>>, bool, bool, Option<&'a str>) -> LoopFuture<'a, Result<TurnEndOutcome>>,
    pub cancel_descendants: fn(&str, &[String]) -> Result<usize>,
    pub descendant_snapshots: fn(&str, &[String]) -> Result<Vec<ChildAgentTaskSnapshot>>,
    pub wait_for_descendants: for<'a> fn(&'a str, &'a [String], std::time::Duration) -> LoopFuture<'a, Result<ChildAgentWaitOutcome>>,
    pub deliver_model_events: fn(&ToolUseContext<C>, &mut Vec<serde_json::Value>) -> Result<bool>,
}
pub fn consume<C: ContextHost, T>(services: &LoopServices<C,T>, context: &ToolUseContext<C>, history: &mut Vec<ChatMessage>) -> Result<bool> {
    let mut events = Vec::new();
    let delivered = (services.deliver_model_events)(context, &mut events)?;
    history.extend(events.into_iter().map(|event| ChatMessage::user(event["content"].as_str().unwrap_or_default())));
    Ok(delivered)
}
pub async fn integrate<C: ContextHost,T>(services: &LoopServices<C,T>, context: &ToolUseContext<C>, history: &mut Vec<ChatMessage>, cancellation: Option<&CancellationToken>, budget_exhausted: bool) -> Result<bool> {
    loop {
        if cancellation.is_some_and(CancellationToken::is_cancelled) {
            (services.cancel_descendants)(context.session(), &[])?;
            return Err(ToolLoopCancelled.into());
        }
        if budget_exhausted && (services.descendant_snapshots)(context.session(), &[])?.iter().any(|task| !task.state.is_terminal()) {
            anyhow::bail!("Maximum tool iterations reached: child tasks remain unfinished");
        }
        let outcome = (services.wait_for_descendants)(context.session(), &[], std::time::Duration::from_millis(100)).await?;
        if !outcome.timed_out { break; }
    }
    consume(services, context, history)
}
pub async fn run_tool_call_loop_with_skills<H, C, T>(
    services: &LoopServices<C, T>,
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
    mut skills: Option<&mut super::skills::CoreSkillSession<H>>,
) -> Result<String>
where
    H: super::skills::CoreSkillHost<Context = C>,
    C: ContextHost<ApprovalManager = ApprovalManager, HookRunner = HookRunner,
        SecurityPolicy = SecurityPolicy, AsyncHooks = plugin_hooks::hook_async::AsyncHooks,
        ToolCatalog = ToolCatalog, ToolCatalogSnapshot = ToolCatalogSnapshot,
        ImageAssetId = agent_multimodal::assets::ImageAssetId,
        ImageAssetScope = agent_multimodal::assets::ImageAssetScope>,
{
    let non_cli_approval_context =
        (services.non_cli_approval_context)();
    let channel_reply_target = (services.reply_target)()
        .or_else(|| non_cli_approval_context.as_ref().map(|ctx| ctx.reply_target.clone()));
    let max_iterations = if max_iterations == 0 { DEFAULT_MAX_ITERATIONS } else { max_iterations };
    let initial_history_len = history.len();
    let mut authorized_specs: Vec<ToolSpec> = tools_registry
        .iter()
        .filter(|tool| {
            let tool_id = tool.spec().id;
            !excluded_tools.iter().any(|excluded| excluded == &tool_id)
        })
        .map(|tool| tool.spec())
        .collect();
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

    let inherited = tool_context::context::current_tool_use_context::<C>();
    let session_id = skills
        .as_ref()
        .and_then(|skills| skills.session_id.clone())
        .or_else(|| inherited.as_ref().map(|context| context.session().to_string()))
        .unwrap_or_else(|| format!("tool-loop-{turn_id}"));
    let workspace_dir = security
        .as_ref()
        .map(|policy| policy.workspace_dir.clone())
        .or_else(|| {
            skills
                .as_ref()
                .and_then(|skills| skills.config.as_ref())
                .map(|config| config.workspace_dir.clone())
        })
        .or_else(|| inherited.as_ref().and_then(|context| context.workspace_root()))
        .map_or_else(std::env::current_dir, Ok)?;
    #[cfg(not(target_arch = "wasm32"))]
    let _worktree_use = C::use_lease(&workspace_dir)?;
    let mut tool_use_context =
        ToolUseContext::<C>::new(session_id, Some(workspace_dir.to_string_lossy().into_owned()))
            .inherit_delegated_authority()
            .with_channel(channel_name.to_string())
            .with_provider(provider_name.to_string())
            .with_model(model.to_string())
            .with_turn_id(turn_id.clone())
            .with_bypass_non_cli_approval_for_turn(bypass_non_cli_approval_for_turn);
    let _async_hook_turn = tool_use_context.async_hooks.turn_guard();
    let security = security.or_else(|| {
        skills.as_ref().and_then(|skills| skills.config.as_ref()).map(|config| {
            Arc::new(SecurityPolicy::from_configs(
                &config.autonomy,
                &config.security,
                &config.workspace_dir,
            ))
        })
    });
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
    let live_catalog = inherited.as_ref().and_then(|context| context.live_tool_catalog());
    let mut catalog_snapshot = None;
    let mut discovery = Arc::new((services.new_discovery)(authorized_specs));
    tool_use_context = tool_use_context.with_tool_discovery(discovery.clone());
    let mut image_turn = prepare_legacy_image_turn(history, model, multimodal_config, (services.new_extractor)().as_ref()).await?;
    if let Some(ctx) = image_turn.as_mut() {
        tool_use_context.set_image_asset_scope(&ctx.scope);
        ctx.rewrite_chat_messages(history);
    }
    let tool_use_context = Arc::new(tool_use_context);

    let mut stop_hook_active = false;
    let result = async {
    for iteration in 0..max_iterations {
        if cancellation_token.as_ref().is_some_and(CancellationToken::is_cancelled) {
            return Err(ToolLoopCancelled.into());
        }

        if let Some(retry_prompt) = missing_tool_call_retry_prompt.take() {
            history.push(ChatMessage::user(retry_prompt));
        }

        consume(services, &tool_use_context, history)?;
        update_thinking_progress(&services.wire, on_delta.as_ref(), iteration).await;

        // Capture specifications and executors together, before the model can wait.
        if let Some(catalog) = &live_catalog {
            if let Err(error) = (services.refresh_catalog)(tool_use_context.session(), &workspace_dir, catalog).await {
                tracing::warn!("MCP catalog refresh remains pending: {error}");
            }
            let snapshot = catalog.snapshot();
            if catalog_snapshot.as_ref().is_none_or(|previous| !Arc::ptr_eq(previous, &snapshot)) {
                let specs = snapshot.specs().into_iter().filter(|spec| {
                    !excluded_tools.contains(&spec.id)
                        && tool_use_context.delegated_tool_is_allowed(&spec.id)
                }).collect();
                discovery = Arc::new((services.rebase_discovery)(&discovery, specs));
                catalog_snapshot = Some(snapshot);
            }
        }
        // Search can change the next request, never another call in this round.
        let tool_specs = (services.exposed_specs)(&discovery);
        let use_native_tools = provider.supports_native_tools() && !tool_specs.is_empty();
        let mut request_context = tool_use_context.as_ref().clone().with_tool_discovery(discovery.clone());
        if let Some(snapshot) = &catalog_snapshot {
            request_context = request_context.with_tool_catalog(snapshot.clone());
        }
        let round_tool_use_context = Arc::new(
            request_context
                .with_tool_exposure_snapshot(tool_specs.iter().map(|spec| spec.id.clone())),
        );

        let pending_reloads = tool_use_context.peek_pending_image_reloads();
        if let Some(ctx) = image_turn.as_ref() {
            ctx.attach_pending_reloads_to_chat(history, &pending_reloads);
        }

        let mut skill_messages = match skills.as_deref_mut() {
            Some(skills) => skills.messages(tool_specs.iter().any(|spec| spec.id == "skill"))?,
            None => Vec::new(),
        };
        skill_messages
            .extend(tool_use_context.take_hook_contexts().into_iter().map(ChatMessage::user));
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
                        (services.exposed_instructions)(&tool_specs);
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
        let round = (services.llm_round)(RoundRequest {
    provider, history: request_history.as_deref().unwrap_or(history.as_slice()), observer,
    provider_name, model, temperature, channel_name, multimodal_config,
    cancellation_token: cancellation_token.as_ref(), hooks: hooks.as_deref(),
    tool_specs: &tool_specs, use_native_tools, turn_id: &turn_id, iteration,
})
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

        update_tool_call_progress(&services.wire, on_delta.as_ref(), round.tool_calls.len(), round.duration_secs)
            .await;

        if round.tool_calls.is_empty() {
            if integrate(services, &tool_use_context, history, cancellation_token.as_ref(), iteration + 1 == max_iterations).await? {
                if iteration + 1 == max_iterations {
                    anyhow::bail!("Maximum tool iterations reached: child results remain unintegrated");
                }
                continue;
            }
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
                            &(services.scrub_credentials)(&round.display_text),
                            240
                        ),
                    }),
                );
                send_retry_progress(&services.wire, on_delta.as_ref()).await;
                continue;
            }

            // A malformed payload is never a final answer, including when no tools
            // are available or the last iteration leaves no room for recovery.
            if round.parse_issue_detected {
                anyhow::bail!("Model emitted an unparseable tool call instead of a final response");
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
                            &(services.scrub_credentials)(&round.display_text),
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
                    "text": (services.scrub_credentials)(&round.display_text),
                }),
            );
            let stop = (services.stop_hook)(tool_use_context.clone(), false, stop_hook_active, Some(&round.display_text)).await?;
            if cancellation_token.as_ref().is_some_and(CancellationToken::is_cancelled) {
                return Err(ToolLoopCancelled.into());
            }
            if !stop.contexts.is_empty() {
                history.push(ChatMessage::assistant(round.response_text));
                history.extend(stop.contexts.into_iter().map(ChatMessage::user));
                stop_hook_active = true;
                continue;
            }
            stream_final_response(&services.wire,
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

        (services.execute_tools)(ToolRoundRequest {
    history, tool_calls: &round.tool_calls, assistant_history_content: round.assistant_history_content,
    native_tool_calls: &round.native_tool_calls, tools_registry, observer, channel_name,
    channel_reply_target: channel_reply_target.as_deref(), provider_name, model,
    turn_id: &turn_id, iteration, tool_use_context: &round_tool_use_context,
    on_delta: on_delta.as_ref(), excluded_tools, bypass_non_cli_approval_for_turn,
    cancellation_token: cancellation_token.as_ref(), seen_tool_signatures: &mut seen_tool_signatures,
    use_native_tools,
})
        .await?;
        consume(services, &tool_use_context, history)?;
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
    }.await;
    if result.is_err() {
        (services.cancel_descendants)(tool_use_context.session(), &[])?;
    }
    if cancellation_token.as_ref().is_some_and(CancellationToken::is_cancelled) {
        (services.stop_hook)(tool_use_context, true, false, None).await?;
    }
    result
}

async fn prepare_legacy_image_turn(
    history: &[ChatMessage],
    model: &str,
    multimodal_config: &config_tools::MultimodalConfig,
    extractor: &dyn ImageTextExtractor,
) -> Result<Option<agent_multimodal::context::ImageTurnContext>> {
    use agent_multimodal::assets::{ImageAssetScope, ImageAssetStore};
    use agent_multimodal::context::ImageTurnContext;
    use agent_multimodal::parse_image_markers;

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
    let Some(multimodal_config) = agent_multimodal::ocr::image_preprocessing_config(
        multimodal_config,
        model,
        extractor,
    )
    .await?
    else {
        return Ok(None);
    };
    let ctx = ImageTurnContext::build(
        &store,
        scope,
        &prior,
        current.as_deref(),
        false,
        &multimodal_config,
        Some(model),
        extractor,
    )
    .await?;
    if ctx.images.is_empty() { Ok(None) } else { Ok(Some(ctx)) }
}

fn is_transient_protocol_message(message: &ChatMessage) -> bool {
    message.content.starts_with("<system-reminder>")
}
