//! Session turn orchestration. Host adapters supply concrete IO and contexts.
use crate::helpers::{allowed_tool_ids_for_request_from_specs, is_acp_request, response_preview};
use crate::prefetch as prompt;
use crate::processor_events::StreamEvent;
use crate::runner_host::{RunnerHost, RunnerPrompt};
use crate::runner_protocol::{
    DesktopSessionProtocol, compact_transient_history_for_request,
    delegated_skill_policy_resolution, option_string_values, prepare_context_before_llm_step,
    provider_request_options,
};
use crate::runner_types::Request as RequestData;
use crate::runner_types::{ResolvedRequestModel, ToolSessionState};
use crate::session_memory::{Message, Role, Session};
use crate::turn_policy::ChatTurnPolicy;
use crate::{context_governance, llm_messages, llm_runner};
use serde_json::{Value, json};
use session_port::ui_types as models;
use std::collections::HashSet;
use std::path::PathBuf;
use tool_assembly::runtime::ToolRuntimeContext;
use tool_context::context::ToolUseContext;
pub type Request<H> = RequestData<
    <<H as RunnerHost>::CtxHost as tool_context::host::ContextHost>::ApprovalManager,
    <<H as RunnerHost>::CtxHost as tool_context::host::ContextHost>::SessionSkillSources,
>;
pub const EMPTY_RESPONSE_ERR: &str = "模型未返回内容";
pub const EMPTY_RESPONSE_RETRY_PROMPT: &str =
    "上一步模型输出为空，已自动重试。请继续当前任务并返回非空答复。";
pub const EMPTY_RESPONSE_RETRY_LIMIT: usize = 4;
#[path = "runner_model.rs"]
mod model;
pub use model::*;
#[path = "runner_tools.rs"]
mod tools;
pub use tools::*;
#[path = "runner_response.rs"]
mod response;
pub use response::*;
#[path = "runner_text.rs"]
mod text;
pub use text::*;
#[path = "runner_images.rs"]
mod images;
pub use images::*;

pub fn run<H: RunnerHost>(
    mut req: Request<H>,
    persisted_history: Option<Vec<Value>>,
    mut on_event: impl FnMut(StreamEvent) -> bool + Send + 'static,
) {
    if let Err(message) = ensure_persistent_request_message_id::<H>(&mut req) {
        on_event(StreamEvent::Error(message));
        return;
    }
    let mut session = Session::new(req.session.clone());
    let mut raw_rounds = Vec::new();
    let is_acp = is_acp_request(&req.options);
    let app_session_scope = req.app_session_scope.as_deref();

    let agent_key = req.options.get("agent").and_then(Value::as_str).map(ToOwned::to_owned);
    let mut config = match H::config() {
        Ok(config) => config,
        Err(error) => {
            on_event(StreamEvent::Error(format!("Failed to load request config: {error}")));
            return;
        }
    };
    if let Err(error) = H::apply_binding(&req.session, &mut config) {
        on_event(StreamEvent::Error(error.to_string()));
        return;
    }
    let native_defaults = if !is_acp {
        match H::native_defaults(
            &config,
            agent_key.as_deref().unwrap_or("main"),
            req.model.as_deref(),
            std::path::Path::new(req.root.as_deref().unwrap_or(".")),
        ) {
            Ok(defaults) => defaults,
            Err(error) => {
                on_event(StreamEvent::Error(error.to_string()));
                return;
            }
        }
    } else {
        None
    };
    if let Some(defaults) = &native_defaults {
        if let Some(model) = &defaults.effective.model {
            req.model = Some(format!("{}/{}", model.provider, model.model));
        }
        if defaults.skills_fixed
            || !defaults.effective.skills.is_empty()
            || defaults.skill_policy == config_agent::DelegateCapabilityPolicy::Strict
        {
            let inherited_strict =
                req.options.get("delegate_skill_policy").and_then(Value::as_str) == Some("strict");
            let inherited = if req.options.get("delegate_allowed_skills").is_some() {
                option_string_values(&req.options, "delegate_allowed_skills")
            } else {
                option_string_values(&req.options, "delegate_default_allowed_skills")
            };
            let boundary = defaults
                .allowed_skills
                .iter()
                .filter(|id| !inherited_strict || inherited.contains(id))
                .cloned()
                .collect::<Vec<_>>();
            req.options["delegate_allowed_skills"] = serde_json::json!(boundary);
            if inherited_strict
                && defaults.effective.skills.iter().any(|id| !inherited.contains(id))
            {
                on_event(StreamEvent::Error(
                    "Default skill is outside the inherited strict boundary".into(),
                ));
                return;
            }
            req.options["delegate_default_allowed_skills"] =
                serde_json::json!(defaults.effective.skills);
            if !inherited_strict {
                req.options["delegate_skill_policy"] = serde_json::json!(defaults.skill_policy);
            }
        }
    }
    let model_selected_by_auto = !is_acp && is_auto_model_request(req.model.as_deref());
    let (provider_retries, provider_backoff_ms) = request_provider_retry_policy::<H>(&req);
    let mut initial_budget =
        crate::recovery::RetryBudget::new(provider_retries, provider_backoff_ms);
    let selected_model =
        match initial_budget.run(is_acp, req.abort_token.as_ref(), &mut on_event, |_| {
            resolve_request_model::<H>(&req, &config)
        }) {
            Ok(model) => model,
            Err(message) => {
                on_event(StreamEvent::Error(message));
                return;
            }
        };
    req.model = Some(selected_model.model.clone());

    session.messages = req
        .history
        .iter()
        .map(|message| Message {
            role: match message.role {
                models::ChatRole::User => Role::User,
                models::ChatRole::Assistant => Role::Assistant,
                models::ChatRole::System => Role::System,
                models::ChatRole::Developer => Role::Developer,
                models::ChatRole::Tool => Role::Tool,
            },
            content: message.content.clone(),
        })
        .collect();

    let persisted_history_len = persisted_history.as_ref().map_or(0, |_| session.messages.len());
    let checkpoint_source = persisted_history
        .clone()
        .unwrap_or_else(|| llm_messages::session_messages_to_llm_messages(&session));
    let restored_context = match H::restore_context(&req.session, &checkpoint_source) {
        Ok(history) => history,
        Err(error) => {
            on_event(StreamEvent::Error(error));
            return;
        }
    };
    let skill_workspace = req
        .root
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let Some((admitted_ctx, admission)) = H::admit_context(
        &req,
        &config,
        native_defaults,
        &selected_model,
        model_selected_by_auto,
        &skill_workspace,
        &mut on_event,
    ) else {
        return;
    };
    // Preserve the original drop order: interrupt, runtime context, async hooks, plugin turn.
    let _admission = admission;
    let mut ctx = admitted_ctx;
    let _interrupt = InterruptGuard::<H>(ctx.tool_use_context());
    let mut tool_state = ToolSessionState::default();
    let skills = tool_context::context::scope_tool_use_context_sync::<H::CtxHost, _, _>(
        ctx.tool_use_context().clone(),
        || H::load_skills(&skill_workspace, &config, &req.session_skills, false),
    );
    let skills = match skills {
        Ok(skills) => skills,
        Err(error) => {
            on_event(StreamEvent::Error(format!("Session skill sources unavailable: {error}")));
            return;
        }
    };
    let skill_policy_resolution = delegated_skill_policy_resolution(&skills, &req.options);
    let skills = if let Some(resolution) = skill_policy_resolution.as_ref() {
        skills
            .into_iter()
            .filter(|skill| {
                resolution.visible_ids.contains(&skill.invocation_name().trim().to_lowercase())
            })
            .collect()
    } else {
        skills
    };
    let mut session_protocol = DesktopSessionProtocol::new(
        skills,
        agent_key.as_deref().unwrap_or("main"),
        usize::try_from(selected_model.limit.context).unwrap_or(usize::MAX),
    );
    session_protocol.persisted_history = persisted_history.unwrap_or_default();
    session_protocol.persisted_history_len = persisted_history_len;
    #[cfg(not(target_arch = "wasm32"))]
    for message in &checkpoint_source {
        if message["role"] == "native"
            && let Some(content) = message["content"].as_str()
            && let Ok(crate::native_history::NativeHistoryItem::ToolSearchCall(call)) =
                serde_json::from_str(content)
            && let Some(id) = call.call_id
        {
            session_protocol.native_call_ids.insert(id);
        }
    }

    if should_handle_docs_request_locally(&req.query) {
        handle_docs_request::<H>(&mut session, &ctx, &mut on_event);
        return;
    }

    let turn_policy = ChatTurnPolicy::from_config(&config, agent_key.as_deref());
    let enabled_identity_files = enabled_identity_files_for_agent(&config, agent_key.as_deref());
    let mut base_system = H::system_prompt(
        selected_model.step_event_model(),
        req.root.as_deref(),
        agent_key.as_deref(),
        &enabled_identity_files,
    );
    let agents_referenced_skills = H::referenced_skills(
        &session_protocol.skills,
        base_system.instruction_context().iter().map(|instruction| instruction.content.as_str()),
    );
    let user_referenced_skills =
        H::referenced_skills(&session_protocol.skills, std::iter::once(req.query.as_str()));
    let mut explicitly_requested_skills = agents_referenced_skills.clone();
    explicitly_requested_skills.extend(user_referenced_skills.iter().cloned());
    if H::explicit_skill_request(&req.query) {
        explicitly_requested_skills.extend(H::invocable_skills(&session_protocol.skills));
    }
    let tool_use_context = ctx
        .tool_use_context()
        .as_ref()
        .clone()
        .with_explicitly_requested_skills(explicitly_requested_skills);
    ctx = ctx.with_tool_use_context(tool_use_context);
    #[cfg(not(target_arch = "wasm32"))]
    {
        session_protocol.skills = H::accessible_skills(
            std::mem::take(&mut session_protocol.skills),
            Some(ctx.tool_use_context().as_ref()),
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    if H::run_video_command(&req, &ctx, &session_protocol.skills, &mut on_event)
        || H::run_bound_command(&req, &ctx, &mut on_event)
    {
        return;
    }
    if let Err(message) = session_protocol.prepare_native_skill_context::<H>(
        &req.session,
        &req.options,
        &checkpoint_source,
        skill_policy_resolution.as_ref(),
        &agents_referenced_skills,
        &user_referenced_skills,
    ) {
        on_event(StreamEvent::Error(message));
        return;
    }
    // AGENTS.md 和用户显式点名的技能必须先进入可信工具上下文，再生成工具目录。
    // 否则 deny 规则会提前从 skill 工具描述中移除这些技能，模型只能看到一个
    // 无法执行的协议要求。
    let live_catalog = tool_context::context::current_tool_use_context::<H::CtxHost>()
        .and_then(|context| context.live_tool_catalog());
    let live_catalog = if let Some(catalog) = live_catalog {
        Some(catalog)
    } else {
        let tools = H::execution_tools(&ctx);
        let catalog = std::sync::Arc::new(tool_catalog::catalog::ToolCatalog::default());
        let ids = tools.iter().map(|tool| tool.spec().id).collect::<Vec<_>>();
        if let Err(error) =
            catalog.replace(tools.into_iter().map(std::sync::Arc::from).collect(), ids)
        {
            on_event(StreamEvent::Error(format!(
                "failed to initialize request tool catalog: {error}"
            )));
            return;
        }
        Some(catalog)
    };
    let mcp_connections = H::mcp_connections(&ctx);
    let mut catalog_snapshot = live_catalog.as_ref().map(|catalog| catalog.snapshot());
    #[cfg(not(target_arch = "wasm32"))]
    if let Some(snapshot) = catalog_snapshot.take() {
        match H::request_catalog(&ctx.tool_use_context(), snapshot) {
            Ok(snapshot) => catalog_snapshot = Some(snapshot),
            Err(error) => {
                on_event(StreamEvent::Error(error.to_string()));
                return;
            }
        }
    }
    if let Some(snapshot) = &catalog_snapshot {
        ctx = H::with_request_tool_catalog(&ctx, &req.session_tools, snapshot);
    }
    let available_tool_specs = catalog_snapshot
        .as_ref()
        .map(|snapshot| snapshot.specs())
        .unwrap_or_else(|| H::tool_specs(&ctx))
        .into_iter()
        // Exclude worker-ineligible tools before required-tool expansion.
        .filter(|spec| agent_key.as_deref().unwrap_or("main") == "main" || spec.id != "AgentTool")
        .filter(|spec| ctx.tool_use_context().delegated_tool_is_allowed(&spec.id))
        .collect::<Vec<_>>();
    let mut allowed_tools =
        allowed_tool_ids_for_request_from_specs(&available_tool_specs, &req.options);
    let mut discovery = std::sync::Arc::new(H::new_discovery(
        available_tool_specs.into_iter().filter(|spec| allowed_tools.contains(&spec.id)).collect(),
    ));
    ctx = ctx.clone().with_tool_use_context(
        ctx.tool_use_context().as_ref().clone().with_tool_discovery(discovery.clone()),
    );
    let prompt_tool_ids = allowed_tools.iter().cloned().collect::<Vec<_>>();
    base_system = H::runtime_prompt(
        base_system,
        selected_model.step_event_model(),
        req.root.as_deref(),
        &prompt_tool_ids,
        &req.options,
    );
    if is_acp {
        req.options["acp_allowed_tools"] =
            serde_json::json!(allowed_tools.iter().cloned().collect::<Vec<_>>());
    }
    if !base_system.visible_instruction_paths().is_empty()
        && !on_event(StreamEvent::ContextLoaded {
            paths: base_system.visible_instruction_paths().to_vec(),
        })
    {
        return;
    }

    let prefetch_start = session.messages.len();
    crate::prefetch::prefetch_seed_context::<H::Prefetch>(
        &mut session,
        &req.query,
        &ctx,
        &allowed_tools,
        &mut tool_state,
    );
    session_protocol.hidden_prefetch = prefetch_start..session.messages.len();
    crate::runner_ingest::ingest_user_query::<H>(
        &mut session,
        &req.query,
        &ctx,
        &allowed_tools,
        &mut on_event,
        &mut tool_state,
    );

    let primary_system_prompt = req
        .options
        .get("chat_system_prompt")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned);
    let mut llm_messages = restored_context;
    let history_len = checkpoint_source.len();
    let mut current_messages = checkpoint_source;
    llm_messages::extend_llm_messages_from_session_range(
        &mut current_messages,
        &session,
        prefetch_start,
    );
    llm_messages.extend_from_slice(&current_messages[history_len..]);
    if let Err(error) = session_protocol
        .persist_checkpoint::<H>(&req.session, &session_protocol.checkpoint_history(&session))
    {
        on_event(StreamEvent::Error(error));
        return;
    }
    let checkpoint_source = current_messages;
    if let Some(source) = llm_messages::input_source_from_options(&req.options) {
        llm_messages::annotate_current_user_source(&mut llm_messages, source);
    }
    let mut image_turn =
        match build_image_turn_context_for_request::<H>(&req, &session, &config, &mut on_event) {
            Ok(ctx) => ctx,
            Err(message) => {
                tracing::warn!(
                    target: "vw_agent",
                    session_id = %req.session,
                    error = %message,
                    "failed to build image turn context"
                );
                session.push(Role::System, message.clone());
                on_event(StreamEvent::Error(message));
                return;
            }
        };
    if let Some(ctx_images) = image_turn.as_mut() {
        ctx.tool_use_context().set_image_asset_scope(&ctx_images.scope);
        ctx_images.rewrite_json_messages(&mut llm_messages);
        let store = agent_multimodal::assets::ImageAssetStore::from_global_paths();
        if let Err(error) = ctx_images.register_session_refs(&store, &req.session) {
            tracing::warn!(
                target: "vw_agent",
                session_id = %req.session,
                error = %error,
                "failed to register image asset session refs"
            );
        }
    }

    let mut stop_hook_active = false;
    let mut initial_budget = Some(initial_budget);
    let mut step = 0usize;
    let mut step_index: u32 = 0;
    let mut total_usage = models::TokenUsage::default();
    let mut empty_response_retries = 0usize;
    let model_limit = selected_model.limit.clone();
    let mut observed_input_tokens: Option<i64> = None;
    let mut internal_compaction_attempted = false;

    loop {
        if let Err(error) =
            H::deliver_model_events(ctx.tool_use_context().as_ref(), &mut llm_messages)
        {
            on_event(StreamEvent::Error(error.to_string()));
            return;
        }
        // Definitions and executors change together, only before the next model request.
        if let Some(catalog) = &live_catalog {
            if let Err(error) = H::refresh_catalog(&mcp_connections, catalog) {
                tracing::warn!("MCP catalog refresh remains pending: {error}");
            }
            let snapshot = catalog.snapshot();
            #[cfg(not(target_arch = "wasm32"))]
            let snapshot = match H::request_catalog(&ctx.tool_use_context(), snapshot) {
                Ok(snapshot) => snapshot,
                Err(error) => {
                    on_event(StreamEvent::Error(error.to_string()));
                    return;
                }
            };
            if catalog_snapshot
                .as_ref()
                .is_none_or(|previous| !std::sync::Arc::ptr_eq(previous, &snapshot))
            {
                let specs = snapshot
                    .specs()
                    .into_iter()
                    .filter(|spec| {
                        agent_key.as_deref().unwrap_or("main") == "main" || spec.id != "AgentTool"
                    })
                    .filter(|spec| ctx.tool_use_context().delegated_tool_is_allowed(&spec.id))
                    .collect::<Vec<_>>();
                allowed_tools = allowed_tool_ids_for_request_from_specs(&specs, &req.options);
                discovery = std::sync::Arc::new(H::rebase_discovery(
                    &discovery,
                    specs.into_iter().filter(|spec| allowed_tools.contains(&spec.id)).collect(),
                ));
                catalog_snapshot = Some(snapshot);
            }
        }
        let is_last_step = step >= turn_policy.max_iterations;
        let empty_tools = HashSet::<String>::new();
        let mut system = base_system.runtime_parts(primary_system_prompt.as_deref(), is_last_step);
        let team = H::team_instructions(
            &req.session,
            !is_last_step && allowed_tools.contains("AgentTool"),
        );
        if !team.is_empty() {
            system.push(team);
        }
        let tool_specs_for_context =
            if is_last_step { Vec::new() } else { H::exposed_specs(&discovery) };
        let exposed_tools =
            tool_specs_for_context.iter().map(|spec| spec.id.clone()).collect::<HashSet<_>>();
        let tools_for_step = &exposed_tools;
        if step_index > 0
            && (tools_for_step.contains("skill")
                || !session_protocol
                    .skill_state
                    .invoked_records(&session_protocol.agent_id)
                    .is_empty())
        {
            // 工具可能修改技能元数据；每个请求重新经过加载器和同一能力策略。
            let skills = H::load_skills(&skill_workspace, &config, &req.session_skills, false);
            let skills = match skills {
                Ok(skills) => skills,
                Err(error) => {
                    on_event(StreamEvent::Error(format!(
                        "Session skill sources unavailable: {error}"
                    )));
                    return;
                }
            };
            session_protocol.skills = if let Some(resolution) =
                delegated_skill_policy_resolution(&skills, &req.options)
            {
                skills
                    .into_iter()
                    .filter(|skill| {
                        resolution
                            .visible_ids
                            .contains(&skill.invocation_name().trim().to_lowercase())
                    })
                    .collect()
            } else {
                skills
            };
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            session_protocol.skills = H::accessible_skills(
                std::mem::take(&mut session_protocol.skills),
                Some(ctx.tool_use_context().as_ref()),
            );
        }
        let request_ctx = if let Some(snapshot) = &catalog_snapshot {
            H::with_request_tool_catalog(&ctx, &req.session_tools, snapshot)
        } else {
            ctx.clone()
        };
        let step_ctx = request_ctx.clone().with_tool_use_context(
            request_ctx
                .tool_use_context()
                .as_ref()
                .clone()
                .with_tool_discovery(discovery.clone())
                .with_tool_exposure_snapshot(exposed_tools.iter().cloned()),
        );

        let (provider_retries, provider_backoff_ms) = request_provider_retry_policy::<H>(&req);
        tracing::debug!(
            target: "vw_agent",
            session_id = %req.session,
            step_index = step_index.saturating_add(1),
            provider_retries,
            provider_backoff_ms,
            is_acp,
            max_iterations = turn_policy.max_iterations,
            "session processor selected retry budget"
        );
        step_index = step_index.saturating_add(1);

        let pending_reloads = ctx.tool_use_context().peek_pending_image_reloads();
        if let Some(image_ctx) = image_turn.as_ref() {
            image_ctx.attach_pending_reloads_to_json(&mut llm_messages, &pending_reloads);
        }

        // The preflight estimate must match the tool definitions that the
        // LLM step will actually receive. In particular, OCR's nested request
        // disables tools and should not spend its input budget on every local
        // tool schema.
        let assembled_messages = match session_protocol.assemble(
            &llm_messages,
            base_system.instruction_context(),
            !is_acp && tools_for_step.contains("skill"),
        ) {
            Ok(messages) => messages,
            Err(message) => {
                on_event(StreamEvent::Error(message));
                return;
            }
        };
        let mut provider_messages = assembled_messages.messages;
        provider_messages.extend(
            ctx.tool_use_context()
                .take_hook_contexts()
                .into_iter()
                .map(|text| json!({"role":"user", "content":text})),
        );
        let message_sources = assembled_messages.message_sources;
        let compaction = match prepare_context_before_llm_step(
            &mut provider_messages,
            &system,
            &tool_specs_for_context,
            &model_limit,
            observed_input_tokens,
            is_acp,
        ) {
            Ok(result) => result,
            Err(message) => {
                tracing::warn!(
                    target: "vw_agent",
                    session_id = %req.session,
                    step_index,
                    context_limit = model_limit.context,
                    input_limit = context_governance::effective_input_window(&model_limit),
                    "session processor stopped before llm step because context compaction was insufficient"
                );
                on_event(StreamEvent::Error(message));
                return;
            }
        };
        if compaction.needs_internal_compaction && !internal_compaction_attempted {
            tracing::info!(
                target: "vw_agent",
                session_id = %req.session,
                step_index,
                before_tokens = compaction.before_tokens,
                after_tokens = compaction.after_tokens,
                context_limit = model_limit.context,
                input_limit = context_governance::effective_input_window(&model_limit),
                "session processor requires internal compaction before llm step"
            );
            match tool_context::context::scope_tool_use_context_sync::<H::CtxHost, _, _>(
                ctx.tool_use_context().clone(),
                || {
                    compact_transient_history_for_request::<H>(
                        &req.session,
                        &checkpoint_source,
                        &mut llm_messages,
                        &selected_model.model,
                        &req.reliability,
                        &mut on_event,
                        req.abort_token.as_ref(),
                        &model_limit,
                    )
                },
            ) {
                Ok(true) => {
                    internal_compaction_attempted = true;
                    observed_input_tokens = None;
                    session_protocol.skill_state.reset_catalog(&session_protocol.agent_id);
                    continue;
                }
                Ok(false) => {
                    // Reaching the proactive threshold is not an overflow. A
                    // first turn has nothing to summarize and may still fit.
                    if compaction.before_tokens
                        > context_governance::effective_input_window(&model_limit)
                    {
                        on_event(StreamEvent::Error(
                            "当前请求超过模型输入窗口，且没有可压缩的历史；请缩短输入或减少附件"
                                .to_string(),
                        ));
                        return;
                    }
                }
                Err(error) => {
                    on_event(StreamEvent::Error(error));
                    return;
                }
            }
        }

        if internal_compaction_attempted
            && compaction.before_tokens > context_governance::effective_input_window(&model_limit)
        {
            on_event(StreamEvent::Error(
                "internal compaction did not reduce the request below the model input window"
                    .to_string(),
            ));
            return;
        }

        if !on_event(StreamEvent::StepStart {
            step_index,
            created_ms: crate::processor_clock::now_ms(),
            model: Some(selected_model.model.clone()),
        }) {
            return;
        }

        let provider_options = provider_request_options(&req.options);
        let mut retry_budget = initial_budget.take().unwrap_or_else(|| {
            crate::recovery::RetryBudget::new(provider_retries, provider_backoff_ms)
        });
        let step_out = match tool_context::context::scope_tool_use_context_sync::<H::CtxHost, _, _>(
            ctx.tool_use_context().clone(),
            || {
                llm_runner::run_llm_step_with_budget::<H::Stream>(
                    &req.session,
                    &provider_messages,
                    &system,
                    Some(selected_model.model.clone()),
                    &provider_options,
                    &tool_specs_for_context,
                    tools_for_step,
                    &mut on_event,
                    &mut retry_budget,
                    req.abort_token.as_ref(),
                )
            },
        ) {
            Ok(output) => {
                if let Some(image_ctx) = image_turn.as_mut() {
                    if !pending_reloads.is_empty() {
                        ctx.tool_use_context().clear_pending_image_reloads();
                    }
                    if !image_ctx.has_first_success() || !pending_reloads.is_empty() {
                        image_ctx.mark_first_success();
                        image_ctx.rewrite_json_messages(&mut llm_messages);
                    }
                }
                // A successful primary request permits a fresh compaction after later
                // tool results grow the context. Consecutive failed attempts stay bounded.
                internal_compaction_attempted = false;
                output
            }
            Err(message) => {
                if !is_acp
                    && llm_runner::is_request_too_long_error(&message, &provider_messages)
                    && !internal_compaction_attempted
                {
                    match tool_context::context::scope_tool_use_context_sync::<H::CtxHost, _, _>(
                        ctx.tool_use_context().clone(),
                        || {
                            compact_transient_history_for_request::<H>(
                                &req.session,
                                &checkpoint_source,
                                &mut llm_messages,
                                &selected_model.model,
                                &req.reliability,
                                &mut on_event,
                                req.abort_token.as_ref(),
                                &model_limit,
                            )
                        },
                    ) {
                        Ok(true) => {
                            internal_compaction_attempted = true;
                            observed_input_tokens = None;
                            session_protocol.skill_state.reset_catalog(&session_protocol.agent_id);
                            continue;
                        }
                        Ok(false) => {
                            on_event(StreamEvent::Error(message));
                            return;
                        }
                        Err(error) => {
                            on_event(StreamEvent::Error(format!(
                                "{message}; internal compaction failed: {error}"
                            )));
                            return;
                        }
                    }
                }
                if handle_step_error(
                    &mut session,
                    &mut llm_messages,
                    &mut empty_response_retries,
                    &mut step,
                    &mut on_event,
                    message,
                ) {
                    continue;
                }
                return;
            }
        };

        crate::artifacts::persist_step_ai_call_payload::<H>(
            &artifact_request::<H>(&req),
            &session,
            &total_usage,
            &step_out,
            step_index,
            app_session_scope.as_deref(),
        );

        let finished_ms = crate::processor_clock::now_ms();
        if let Some(raw_payload) = crate::artifacts::persist_llm_raw_step_payload(
            &artifact_request::<H>(&req),
            &ctx.session,
            step_index,
            &provider_messages,
            &message_sources,
            &selected_model.model,
            &system,
            &tool_specs_for_context,
            &step_out,
        ) {
            let Some(turn_id) = req.assistant_message_id.as_deref() else {
                let message =
                    "cannot buffer raw LLM step without a preallocated assistant message id";
                tracing::error!(target: "vw_agent", session_id = %ctx.session, step_index, "{message}");
                on_event(StreamEvent::Error(message.to_string()));
                return;
            };
            raw_rounds.push(raw_payload);
            let raw_artifact = match session_port::llm_raw::LlmRawArtifact::from_full_raw_rounds(
                turn_id,
                &raw_rounds,
            ) {
                Ok(mut artifact) => {
                    let round = artifact.rounds.pop().expect("raw artifact must contain its round");
                    artifact.rounds = vec![round];
                    artifact
                }
                Err(error) => {
                    tracing::error!(target: "vw_agent", session_id = %ctx.session, step_index, error = %error, "failed to construct raw LLM artifact");
                    on_event(StreamEvent::Error(
                        "failed to construct raw LLM artifact".to_string(),
                    ));
                    return;
                }
            };
            if !on_event(StreamEvent::LlmRawArtifact(raw_artifact)) {
                return;
            }
        }

        if !on_event(StreamEvent::StepFinish {
            step_index,
            finished_ms,
            usage: step_out.usage.clone(),
            finish_reason: step_out.finish_reason.clone(),
            model: Some(selected_model.model.clone()),
        }) {
            return;
        }

        total_usage.input_tokens += step_out.usage.input_tokens;
        total_usage.output_tokens += step_out.usage.output_tokens;
        total_usage.cached_tokens += step_out.usage.cached_tokens;
        total_usage.reasoning_tokens += step_out.usage.reasoning_tokens;
        observed_input_tokens = Some(step_out.usage.input_tokens);

        let step_text_preview = response_preview(&step_out.text);
        tracing::debug!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            is_acp,
            is_last_step,
            tool_call_count = step_out.tool_calls.len(),
            tool_calls = %step_out
                .tool_calls
                .iter()
                .map(|call| crate::helpers::tool_call_preview::<H>(&call.name, &call.arguments))
                .collect::<Vec<_>>()
                .join(", "),
            finish_reason = step_out.finish_reason.as_deref().unwrap_or(""),
            text_preview = %step_text_preview,
            "session processor received llm step result"
        );

        if !step_out.tool_calls.is_empty() {
            if !on_event(StreamEvent::ModelToolCalls(step_out.tool_calls.clone())) {
                return;
            }
            if crate::helpers::should_execute_structured_tool_calls_locally(&req.options) {
                empty_response_retries = 0;
                let ran_todo_update = handle_structured_tool_calls::<H>(
                    &req,
                    &mut session,
                    &step_ctx,
                    tools_for_step,
                    base_system.content(),
                    &mut llm_messages,
                    &mut total_usage,
                    &mut on_event,
                    &mut tool_state,
                    &mut session_protocol,
                    &step_out,
                    &step_text_preview,
                    turn_policy.parallel_tools,
                    is_last_step,
                    step_index,
                );
                if let Err(error) = session_protocol.consume_invoked_skills::<H>(
                    ctx.tool_use_context().take_invoked_skills(),
                    &session,
                ) {
                    on_event(StreamEvent::Error(error));
                    return;
                }
                if !on_event(StreamEvent::PostToolRound { step_index }) {
                    return;
                }
                step += 1;
                if ran_todo_update {
                    step += 1;
                }
                continue;
            }

            tracing::warn!(
                target: "vw_agent",
                session_id = %req.session,
                step_index,
                tool_call_count = step_out.tool_calls.len(),
                "session processor ignored structured tool calls emitted by ACP request"
            );
        }

        let should_continue = handle_assistant_text_branch::<H>(
            &req,
            &mut session,
            &step_ctx,
            tools_for_step,
            &mut llm_messages,
            &mut total_usage,
            &mut on_event,
            &mut tool_state,
            &mut empty_response_retries,
            &mut step,
            &step_out,
            &step_text_preview,
            &empty_tools,
            is_last_step,
            step_index,
            app_session_scope.as_deref(),
            &mut stop_hook_active,
        );
        if let Err(error) = session_protocol
            .consume_invoked_skills::<H>(ctx.tool_use_context().take_invoked_skills(), &session)
        {
            on_event(StreamEvent::Error(error));
            return;
        }
        if should_continue {
            continue;
        }
        return;
    }
}

/// The synchronous session runner exits through many error and cancellation branches.
struct InterruptGuard<H: RunnerHost>(std::sync::Arc<ToolUseContext<H::CtxHost>>);
impl<H: RunnerHost> Drop for InterruptGuard<H> {
    fn drop(&mut self) {
        if self.0.abort_token().is_some_and(|token| token.is_cancelled())
            && H::stop_hook(self.0.clone(), true, false, None).is_err()
        {
            tracing::warn!("interrupt hook failed");
        }
    }
}

fn artifact_request<H: RunnerHost>(req: &Request<H>) -> crate::artifacts::ArtifactRequest<'_> {
    crate::artifacts::ArtifactRequest {
        stream: req.stream,
        session: &req.session,
        root: req.root.as_deref(),
        model: req.model.as_deref(),
        options: &req.options,
        persist_app_session_artifacts: req.persist_app_session_artifacts,
    }
}
