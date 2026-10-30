//! 会话处理主循环模块。
//!
//! 本模块承接原先位于入口文件中的完整执行编排逻辑，
//! 负责初始化上下文、驱动 LLM 循环、处理工具调用并在结束时发出终态事件。

use super::artifacts;
use super::context_governance;
use super::helpers::{
    allowed_tool_ids_for_request_from_specs, is_acp_request, response_preview, tool_call_preview,
};
use super::llm_messages;
use super::llm_runner;
use super::prefetch;
use super::todo_updates;
use super::todos;
use super::tools_exec;
use super::turn_policy::ChatTurnPolicy;
use super::types::{CompactionPhase, Request, ResolvedRequestModel, StreamEvent, ToolSessionState};
use super::utils;
use crate::app::agent::config;
use crate::app::agent::tools::{ToolRuntimeContext, ToolUseContext};
use crate::session::prompt;
use crate::session::session::{Message, Role, Session};
use crate::session::ui_types as models;
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::PathBuf;

const EMPTY_RESPONSE_ERR: &str = "模型未返回内容";
const EMPTY_RESPONSE_RETRY_PROMPT: &str =
    "上一步模型输出为空，已自动重试。请继续当前任务并返回非空答复。";
const EMPTY_RESPONSE_RETRY_LIMIT: usize = 4;

/// Desktop 单轮执行期间的临时协议状态。
///
/// 状态只控制 provider 请求的临时消息，不能进入 `Session`，以免 UI 历史出现元提醒。
#[derive(Default)]
struct DesktopSessionProtocol {
    skills: Vec<crate::app::agent::skills::Skill>,
    skill_state: crate::app::agent::skills::SkillListingSessionState,
    agent_id: String,
    context_window_tokens: usize,
}

impl DesktopSessionProtocol {
    fn new(
        skills: Vec<crate::app::agent::skills::Skill>,
        agent_id: impl Into<String>,
        context_window_tokens: usize,
    ) -> Self {
        Self { skills, agent_id: agent_id.into(), context_window_tokens, ..Self::default() }
    }

    fn assemble(
        &mut self,
        history: &[Value],
        instructions: &[crate::session::instruction::Resolved],
        skill_tool_enabled: bool,
    ) -> Result<llm_messages::AssembledProtocolMessages, String> {
        let invoked_skill_messages = self
            .skill_state
            .restore_invoked(&self.agent_id)?
            .into_iter()
            .map(|replay| {
                let entries = vec![replay.stable_id];
                llm_messages::sourced_protocol_message(
                    json!({ "role": "user", "content": replay.content }),
                    llm_messages::context_message_source("invoked_skill", &entries),
                )
            })
            .collect::<Vec<_>>();
        let skill_listing = self
            .skill_state
            .prepare(&self.agent_id, &self.skills, skill_tool_enabled, self.context_window_tokens)
            .map(|reminder| {
                let source =
                    llm_messages::context_message_source("skill_catalog", &reminder.skill_names);
                llm_messages::sourced_protocol_message(
                    json!({ "role": "user", "content": reminder.content }),
                    source,
                )
            });
        Ok(llm_messages::assemble_session_protocol_messages_with_sources(
            history,
            instructions,
            &invoked_skill_messages,
            skill_listing,
        ))
    }

    fn preload_skills(
        &mut self,
        skill_names: &std::collections::BTreeSet<String>,
    ) -> Result<(), String> {
        self.skill_state.preload_defaults(&self.agent_id, &self.skills, skill_names)
    }

    fn consume_tool_output(
        &mut self,
        tool_name: &str,
        input: &str,
        output: &tools_exec::ToolProtocolOutput,
        llm_messages: &mut Vec<Value>,
    ) {
        let invoked_skill_registered = self.register_invoked_skill(tool_name, input, output);
        if invoked_skill_registered {
            return;
        }
        llm_messages.extend(
            output
                .extra_messages
                .iter()
                .map(|message| json!({ "role": message.role, "content": message.content })),
        );
    }

    fn register_invoked_skill(
        &mut self,
        tool_name: &str,
        input: &str,
        output: &tools_exec::ToolProtocolOutput,
    ) -> bool {
        if !tool_name.eq_ignore_ascii_case("skill") || output.extra_messages.is_empty() {
            return false;
        }
        let Some(skill_name) = output.context_updates.iter().find_map(|update| {
            (update.key == "skill.name").then(|| update.value.as_str()).flatten()
        }) else {
            return false;
        };
        let Some(skill) = self.skills.iter().find(|skill| skill.name == skill_name) else {
            tracing::warn!(
                skill_name,
                "skill tool result could not be registered for protocol replay"
            );
            return false;
        };
        let source = skill
            .location
            .clone()
            .unwrap_or_else(|| PathBuf::from("skills").join(&skill.name).join("SKILL.md"));
        let args = serde_json::from_str::<Value>(input)
            .ok()
            .and_then(|value| value.get("args").and_then(Value::as_str).map(ToOwned::to_owned));
        match crate::app::agent::skills::InvokedSkillRecord::new(skill_name, &source, args) {
            Ok(record) => self.skill_state.register_invoked(&self.agent_id, record),
            Err(error) => {
                tracing::warn!(skill_name, error, "skill tool result could not be registered");
                false
            }
        }
    }
}

fn prepare_context_before_llm_step(
    llm_messages: &mut Vec<Value>,
    system: &[String],
    tools: &[crate::app::agent::tools::ToolSpec],
    model_limit: &crate::app::agent::provider::provider::ModelLimit,
    observed_input_tokens: Option<i64>,
    is_acp: bool,
) -> Result<context_governance::ContextCompactionResult, String> {
    // ACP agents own their model window and remote session history. The local
    // placeholder model limit cannot govern or summarize that context.
    if is_acp {
        return Ok(context_governance::ContextCompactionResult::default());
    }
    context_governance::compact_context_for_next_request(
        llm_messages,
        system,
        tools,
        model_limit,
        observed_input_tokens,
    )
}

pub(crate) fn provider_request_options(options: &Value) -> Value {
    let mut options = options.clone();
    if let Some(object) = options.as_object_mut() {
        object.retain(|key, _| !key.starts_with("_vw_") && key != "source");
    }
    options
}

/// Summarize completed history, including tool rounds after the latest user request.
/// Preserve that user request verbatim and replace model context only after checkpoint persistence.
#[allow(clippy::too_many_arguments)] // Keep request identity, budget, and lifecycle explicit.
fn compact_transient_history_for_request(
    session_id: &str,
    checkpoint_source: &[Value],
    llm_messages: &mut Vec<Value>,
    model: &str,
    reliability: &crate::app::agent::config::ReliabilityConfig,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
    model_limit: &crate::provider::provider::ModelLimit,
) -> Result<bool, String> {
    let Some(boundary) = llm_messages
        .iter()
        .rposition(|message| message.get("role").and_then(Value::as_str) == Some("user"))
    else {
        return Ok(false);
    };
    if llm_messages.len() == 1 {
        return Ok(false);
    }
    if !on_event(StreamEvent::Compaction { phase: CompactionPhase::Started }) {
        return Err("stream consumer stopped".to_string());
    }
    let summary = crate::session::compaction_summary::summarize(
        session_id,
        llm_messages,
        model,
        reliability,
        cancellation,
        model_limit,
    )?;
    let current_user = llm_messages[boundary].clone();
    let replacement = vec![
        json!({ "role": "assistant", "content": format!("[Compaction summary]\n{summary}") }),
        current_user,
    ];
    if cancellation.is_some_and(|token| token.is_cancelled()) {
        return Err("执行已取消".into());
    }
    persist_context_checkpoint(session_id, checkpoint_source, &replacement)?;
    *llm_messages = replacement;
    on_event(StreamEvent::Compaction { phase: CompactionPhase::Completed });
    Ok(true)
}

fn persist_context_checkpoint(
    session: &str,
    source: &[Value],
    context: &[Value],
) -> Result<(), String> {
    if source.is_empty() {
        return Ok(());
    }
    let summary = context
        .first()
        .and_then(|item| item["content"].as_str())
        .and_then(|text| text.strip_prefix("[Compaction summary]\n"))
        .ok_or_else(|| "missing compaction summary".to_string())?;
    prompt::block_on(crate::session::compaction_checkpoint::save(
        session,
        source,
        summary,
        context.get(1).cloned(),
    ))
}

fn request_provider_retry_policy(req: &Request) -> (u32, u64) {
    (req.reliability.provider_retries, req.reliability.provider_backoff_ms)
}

fn request_full_access_enabled(options: &Value) -> bool {
    options.get("full_access").and_then(Value::as_bool).unwrap_or(false)
}

fn with_request_model_context(
    mut context: ToolUseContext,
    request_model: Option<&str>,
) -> ToolUseContext {
    let Some(request_model) = request_model.map(str::trim).filter(|value| !value.is_empty()) else {
        return context;
    };
    if let Some((provider, model)) = request_model.split_once('/')
        && !provider.trim().is_empty()
        && !model.trim().is_empty()
    {
        context =
            context.with_provider(provider.trim().to_string()).with_model(model.trim().to_string());
    } else {
        context = context.with_model(request_model.to_string());
    }
    context
}

fn is_auto_model_request(model: Option<&str>) -> bool {
    model.map(str::trim).is_none_or(|model| model.is_empty() || model.eq_ignore_ascii_case("auto"))
}

fn request_needs_image_input(req: &Request) -> bool {
    use crate::app::agent::multimodal::parse_image_markers;

    !parse_image_markers(&req.query).1.is_empty()
        || req.history.iter().any(|message| !parse_image_markers(&message.content).1.is_empty())
}

fn resolve_request_model(
    req: &Request,
    config: &crate::config::Config,
) -> Result<ResolvedRequestModel, String> {
    if is_acp_request(&req.options) {
        let model = req
            .options
            .get("acp_agent")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|model| !model.is_empty())
            .unwrap_or("acp");
        return Ok(ResolvedRequestModel::new(
            format!("acp/{model}"),
            crate::app::agent::provider::provider::ModelLimit {
                context: 8192,
                input: None,
                output: 4096,
            },
        ));
    }

    let providers = prompt::block_on(crate::app::agent::provider::provider::list())
        .map_err(crate::session::llm::catalog_error_string)?;
    let candidates =
        providers.values().flat_map(|info| info.models.values()).cloned().collect::<Vec<_>>();
    let selected = if is_auto_model_request(req.model.as_deref()) {
        let needs_image_input = request_needs_image_input(req);
        let agent_key = req.options.get("agent").and_then(Value::as_str).map(str::trim);
        let soft_preference = agent_key
            .and_then(|key| config.agents.get(key))
            .filter(|agent| !agent.provider.trim().is_empty() && !agent.model.trim().is_empty())
            .map(|agent| crate::session::llm::ModelReference::new(&agent.provider, &agent.model));
        let candidates_for_selection = candidates
            .iter()
            .map(|model| crate::session::llm::ModelCandidate {
                provider_id: &model.provider_id,
                model,
            })
            .collect::<Vec<_>>();
        let decision = crate::session::llm::select_auto_model(
            &crate::session::llm::AutoModelSelectionRequest {
                message: &req.query,
                required_context: 0,
                needs_tools: true,
                needs_image_input,
                needs_attachments: needs_image_input,
                soft_preference,
                query_classification: config.query_classification.clone(),
                model_routes: config.model_routes.clone(),
            },
            &candidates_for_selection,
        )
        .map_err(|error| error.to_string())?;
        candidates
            .into_iter()
            .find(|model| {
                model.provider_id == decision.model.provider && model.id == decision.model.model
            })
            .ok_or_else(|| "Auto 模型选择结果不在 active 候选中".to_string())?
    } else {
        let requested = req.model.as_deref().expect("non-auto request has a model").trim();
        let matches = candidates
            .into_iter()
            .filter(|candidate| {
                if let Some((provider, model)) = requested.split_once('/') {
                    candidate.provider_id == provider && candidate.id == model
                } else {
                    candidate.id == requested
                }
            })
            .collect::<Vec<_>>();
        match matches.as_slice() {
            [model] => model.clone(),
            [] => return Err(format!("未找到模型：{requested}")),
            _ => return Err(format!("模型ID存在歧义：{requested}，请使用 provider/model")),
        }
    };

    Ok(ResolvedRequestModel::new(
        format!("{}/{}", selected.provider_id, selected.id),
        selected.limit,
    ))
}

fn enabled_identity_files_for_agent(
    config: &crate::config::Config,
    agent_key: Option<&str>,
) -> Vec<String> {
    let agent_key = agent_key.map(str::trim).filter(|key| !key.is_empty()).unwrap_or("main");
    config
        .agents
        .get(agent_key)
        .map(|agent| agent.enabled_identity_files.clone())
        .unwrap_or_default()
}

fn delegated_skill_policy_resolution(
    skills: &[crate::app::agent::skills::Skill],
    options: &Value,
) -> Option<crate::session::capability_policy::CapabilityPolicyResolution> {
    options.get("delegate_skill_policy").cloned().and_then(|value| {
        serde_json::from_value::<vw_config_types::agent::DelegateCapabilityPolicy>(value).ok()
    })?;
    Some(crate::session::capability_policy::resolve_skill_capabilities(
        skills.iter().map(|skill| skill.name.as_str()),
        option_string_values(options, "delegate_default_allowed_skills"),
        option_string_values(options, "delegate_trusted_user_selected_skills"),
        vw_config_types::agent::DelegateCapabilityPolicy::Unrestricted,
        std::iter::empty::<&str>(),
    ))
}

fn option_string_values(options: &Value, key: &str) -> Vec<String> {
    options
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn ensure_persistent_request_message_id(req: &mut Request) -> Result<(), String> {
    if req.persist_app_session_artifacts && req.assistant_message_id.is_none() {
        req.assistant_message_id = Some(
            crate::id::ascending(crate::id::Prefix::Message, None)
                .map_err(|error| error.to_string())?,
        );
    }
    Ok(())
}

/// 执行会话处理主循环。
///
/// 本函数保留原有入口签名，对外仍通过 `processor::run` 暴露；
/// 这里仅将实现迁移至独立文件，方便后续按职责维护。
pub fn run(req: Request, mut on_event: impl FnMut(StreamEvent) -> bool + Send + 'static) {
    let Some(root) = req.root.as_deref().filter(|root| !root.trim().is_empty()) else {
        return run_in_project(req, on_event);
    };
    let tenant = crate::session::ui_store::current_tenant_scope();
    let project =
        prompt::block_on(crate::project::from_directory(&tenant, std::path::Path::new(root)));
    match project {
        Ok((project, worktree)) => {
            let context = crate::project::instance::Context {
                directory: root.to_string(),
                worktree,
                project,
            };
            crate::project::instance::provide_context_sync(context, || {
                run_in_project(req, on_event)
            })
        }
        Err(error) => {
            on_event(StreamEvent::Error(format!("Failed to load request project: {error}")));
        }
    }
}

fn run_in_project(
    mut req: Request,
    mut on_event: impl FnMut(StreamEvent) -> bool + Send + 'static,
) {
    if let Err(message) = ensure_persistent_request_message_id(&mut req) {
        on_event(StreamEvent::Error(message));
        return;
    }
    let mut session = Session::new(req.session.clone());
    let mut raw_rounds = Vec::new();
    let is_acp = is_acp_request(&req.options);
    let model_selected_by_auto = !is_acp && is_auto_model_request(req.model.as_deref());
    let app_session_scope = req.app_session_scope.as_deref();

    let agent_key = req.options.get("agent").and_then(Value::as_str).map(ToOwned::to_owned);
    let config = match prompt::block_on(config::get_current()) {
        Ok(config) => config,
        Err(error) => {
            on_event(StreamEvent::Error(format!("Failed to load request config: {error}")));
            return;
        }
    };
    let (provider_retries, provider_backoff_ms) = request_provider_retry_policy(&req);
    let mut initial_budget =
        super::recovery::RetryBudget::new(provider_retries, provider_backoff_ms);
    let selected_model =
        match initial_budget.run(is_acp, req.abort_token.as_ref(), &mut on_event, |_| {
            resolve_request_model(&req, &config)
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
                models::ChatRole::Tool => Role::Tool,
            },
            content: message.content.clone(),
        })
        .collect();

    let checkpoint_source = llm_messages::session_messages_to_llm_messages(&session);
    let restored_context = match prompt::block_on(crate::session::compaction_checkpoint::restore(
        &req.session,
        &checkpoint_source,
    )) {
        Ok(history) => history,
        Err(error) => {
            on_event(StreamEvent::Error(error));
            return;
        }
    };
    let mut tool_use_context = with_request_model_context(
        ToolUseContext::new(req.session.clone(), req.root.clone()),
        selected_model.step_event_model(),
    )
    .with_auto_model_selection(model_selected_by_auto);
    match crate::hooks::execution_hook_runner(&config) {
        Ok(Some(runner)) => tool_use_context = tool_use_context.with_hook_runner(runner),
        Ok(None) => {}
        Err(_) => {
            on_event(StreamEvent::Error("required execution hooks unavailable".into()));
            return;
        }
    }
    let trusted_user_selected_skills =
        option_string_values(&req.options, "delegate_trusted_user_selected_skills");
    if !trusted_user_selected_skills.is_empty() {
        tool_use_context =
            tool_use_context.with_trusted_user_selected_skills(trusted_user_selected_skills);
    }
    if let Some(channel_name) = req.channel_name.clone() {
        tool_use_context = tool_use_context.with_channel(channel_name);
    }
    if let Some(approval) = req.approval.clone() {
        tool_use_context = tool_use_context.with_approval(approval);
    }
    if let Some(non_cli_approval_context) = req.non_cli_approval_context.clone() {
        tool_use_context = tool_use_context.with_non_cli_approval_context(non_cli_approval_context);
    }
    if let Some(message_id) = req.assistant_message_id.clone() {
        tool_use_context = tool_use_context.with_message_id(message_id);
    }
    if let Some(abort_token) = req.abort_token.clone() {
        tool_use_context = tool_use_context.with_abort_token(abort_token);
    }
    let full_access_enabled = request_full_access_enabled(&req.options);
    tool_use_context = tool_use_context.with_full_access_enabled(full_access_enabled);
    if full_access_enabled {
        tool_use_context = tool_use_context.with_bypass_non_cli_approval_for_turn(true);
    }
    let mut ctx = ToolRuntimeContext::new(req.session.clone(), req.root.clone())
        .with_tool_use_context(tool_use_context)
        .with_session_tools(req.session_tools.clone());
    let mut tool_state = ToolSessionState::default();
    let skill_workspace = req
        .root
        .as_deref()
        .map(PathBuf::from)
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."));
    let skills = crate::app::agent::skills::load_skills_with_config(&skill_workspace, &config);
    let skill_policy_resolution = delegated_skill_policy_resolution(&skills, &req.options);
    let skills = if let Some(resolution) = skill_policy_resolution.as_ref() {
        skills
            .into_iter()
            .filter(|skill| resolution.visible_ids.contains(&skill.name.trim().to_lowercase()))
            .collect()
    } else {
        skills
    };
    let mut session_protocol = DesktopSessionProtocol::new(
        skills,
        agent_key.as_deref().unwrap_or("main"),
        usize::try_from(selected_model.limit.context).unwrap_or(usize::MAX),
    );
    if let Some(resolution) = &skill_policy_resolution
        && let Err(message) = session_protocol.preload_skills(&resolution.default_injected_ids)
    {
        on_event(StreamEvent::Error(message));
        return;
    }

    if should_handle_docs_request_locally(&req.query) {
        handle_docs_request(&mut session, &ctx, &mut on_event);
        return;
    }

    let turn_policy = ChatTurnPolicy::from_config(&config, agent_key.as_deref());
    let enabled_identity_files = enabled_identity_files_for_agent(&config, agent_key.as_deref());
    let mut base_system = prompt::system_with_agent_metadata(
        selected_model.step_event_model(),
        req.root.as_deref(),
        agent_key.as_deref(),
        &enabled_identity_files,
    );
    let agents_referenced_skills =
        crate::app::agent::tools::skill::explicitly_referenced_skill_names(
            &session_protocol.skills,
            base_system
                .instruction_context()
                .iter()
                .map(|instruction| instruction.content.as_str()),
        );
    if let Err(message) = session_protocol.preload_skills(&agents_referenced_skills) {
        on_event(StreamEvent::Error(message));
        return;
    }
    let mut explicitly_requested_skills = agents_referenced_skills;
    explicitly_requested_skills.extend(
        crate::app::agent::tools::skill::explicitly_referenced_skill_names(
            &session_protocol.skills,
            std::iter::once(req.query.as_str()),
        ),
    );
    if crate::app::agent::tools::skill::user_explicitly_requests_skill_use(&req.query) {
        explicitly_requested_skills.extend(
            crate::app::agent::tools::skill::user_invocable_skill_names(&session_protocol.skills),
        );
    }
    let tool_use_context = ctx
        .tool_use_context()
        .as_ref()
        .clone()
        .with_explicitly_requested_skills(explicitly_requested_skills);
    ctx = ctx.with_tool_use_context(tool_use_context);
    // AGENTS.md 和用户显式点名的技能必须先进入可信工具上下文，再生成工具目录。
    // 否则 deny 规则会提前从 skill 工具描述中移除这些技能，模型只能看到一个
    // 无法执行的协议要求。
    let available_tool_specs = crate::app::agent::tools::tool_specs_for_context(&ctx);
    let allowed_tools =
        allowed_tool_ids_for_request_from_specs(&available_tool_specs, &req.options);
    let prompt_tool_ids = allowed_tools.iter().cloned().collect::<Vec<_>>();
    base_system = base_system.with_runtime_context_and_modes(
        selected_model.step_event_model(),
        req.root.as_deref(),
        &prompt_tool_ids,
        crate::session::system::RuntimePromptModes::from_options(&req.options),
    );
    if is_acp {
        req.options["acp_allowed_tools"] =
            serde_json::json!(allowed_tools.iter().cloned().collect::<Vec<_>>());
    }
    if !base_system.visible_instruction_paths.is_empty()
        && !on_event(StreamEvent::ContextLoaded {
            paths: base_system.visible_instruction_paths.clone(),
        })
    {
        return;
    }

    prefetch::prefetch_seed_context(
        &mut session,
        &req.query,
        &ctx,
        &allowed_tools,
        &mut tool_state,
    );
    utils::ingest_user_query(
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
    let current_messages = llm_messages::session_messages_to_llm_messages(&session);
    llm_messages.extend_from_slice(&current_messages[checkpoint_source.len()..]);
    let checkpoint_source = current_messages;
    if let Some(source) = llm_messages::input_source_from_options(&req.options) {
        llm_messages::annotate_current_user_source(&mut llm_messages, source);
    }
    let mut image_turn = match build_image_turn_context_for_request(&req, &session, &config) {
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
        let store = crate::app::agent::multimodal::assets::ImageAssetStore::from_global_paths();
        if let Err(error) = ctx_images.register_session_refs(&store, &req.session) {
            tracing::warn!(
                target: "vw_agent",
                session_id = %req.session,
                error = %error,
                "failed to register image asset session refs"
            );
        }
    }

    let mut initial_budget = Some(initial_budget);
    let mut step = 0usize;
    let mut step_index: u32 = 0;
    let mut total_usage = models::TokenUsage::default();
    let mut empty_response_retries = 0usize;
    let model_limit = selected_model.limit.clone();
    let mut observed_input_tokens: Option<i64> = None;
    let mut internal_compaction_attempted = false;

    loop {
        let is_last_step = step >= turn_policy.max_iterations;
        let empty_tools = HashSet::<String>::new();
        let system = base_system.runtime_parts(primary_system_prompt.as_deref(), is_last_step);
        let tools_for_step = if is_last_step { &empty_tools } else { &allowed_tools };

        let (provider_retries, provider_backoff_ms) = request_provider_retry_policy(&req);
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
        let tool_specs_for_context = if is_last_step {
            Vec::new()
        } else {
            available_tool_specs
                .iter()
                .filter(|spec| allowed_tools.contains(&spec.id))
                .cloned()
                .collect::<Vec<_>>()
        };
        let assembled_messages = match session_protocol.assemble(
            &llm_messages,
            base_system.instruction_context(),
            tools_for_step.contains("skill"),
        ) {
            Ok(messages) => messages,
            Err(message) => {
                on_event(StreamEvent::Error(message));
                return;
            }
        };
        let mut provider_messages = assembled_messages.messages;
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
            match compact_transient_history_for_request(
                &req.session,
                &checkpoint_source,
                &mut llm_messages,
                &selected_model.model,
                &req.reliability,
                &mut on_event,
                req.abort_token.as_ref(),
                &model_limit,
            ) {
                Ok(true) => {
                    internal_compaction_attempted = true;
                    observed_input_tokens = None;
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
            created_ms: utils::now_ms(),
            model: Some(selected_model.model.clone()),
        }) {
            return;
        }

        let provider_options = provider_request_options(&req.options);
        let mut retry_budget = initial_budget.take().unwrap_or_else(|| {
            super::recovery::RetryBudget::new(provider_retries, provider_backoff_ms)
        });
        let step_out = match llm_runner::run_llm_step_with_budget(
            &req.session,
            &provider_messages,
            &system,
            Some(selected_model.model.clone()),
            &provider_options,
            &available_tool_specs,
            tools_for_step,
            &mut on_event,
            &mut retry_budget,
            req.abort_token.as_ref(),
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
                    match compact_transient_history_for_request(
                        &req.session,
                        &checkpoint_source,
                        &mut llm_messages,
                        &selected_model.model,
                        &req.reliability,
                        &mut on_event,
                        req.abort_token.as_ref(),
                        &model_limit,
                    ) {
                        Ok(true) => {
                            internal_compaction_attempted = true;
                            observed_input_tokens = None;
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

        artifacts::persist_step_ai_call_payload(
            &req,
            &session,
            &total_usage,
            &step_out,
            step_index,
            app_session_scope.as_deref(),
        );

        let finished_ms = utils::now_ms();
        if let Some(raw_payload) = artifacts::persist_llm_raw_step_payload(
            &req,
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
            let raw_artifact = match crate::session::llm_raw::LlmRawArtifact::from_full_raw_rounds(
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
                .map(|call| tool_call_preview(&call.name, &call.arguments))
                .collect::<Vec<_>>()
                .join(", "),
            finish_reason = step_out.finish_reason.as_deref().unwrap_or(""),
            text_preview = %step_text_preview,
            "session processor received llm step result"
        );

        if !step_out.tool_calls.is_empty() {
            if super::should_execute_structured_tool_calls_locally(&req.options) {
                empty_response_retries = 0;
                let ran_todo_update = handle_structured_tool_calls(
                    &req,
                    &mut session,
                    &ctx,
                    &allowed_tools,
                    &base_system.content,
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

        if handle_assistant_text_branch(
            &req,
            &mut session,
            &ctx,
            &allowed_tools,
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
        ) {
            continue;
        }
        return;
    }
}

fn should_handle_docs_request_locally(query: &str) -> bool {
    utils::is_docs_request(query)
}

fn handle_docs_request(
    session: &mut Session,
    ctx: &ToolRuntimeContext,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
) {
    match utils::list_docs(ctx.root.as_ref()) {
        Ok(files) => {
            let mut out = String::new();
            out.push_str(
                "<think>\n思考中\n任务分解：\n- 读取 docs 目录\n- 整理文件列表\n</think>\n\n",
            );
            out.push_str("docs 目录下的文件：\n");
            for path in files {
                out.push_str("- ");
                out.push_str(&path);
                out.push('\n');
            }
            session.push(Role::Assistant, out.clone());
            if !on_event(StreamEvent::Delta(out)) {
                return;
            }
            on_event(StreamEvent::Done(models::TokenUsage::default()));
        }
        Err(message) => {
            session.push(Role::System, message.clone());
            on_event(StreamEvent::Error(message));
        }
    }
}

fn handle_step_error(
    session: &mut Session,
    llm_messages: &mut Vec<Value>,
    empty_response_retries: &mut usize,
    step: &mut usize,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    message: String,
) -> bool {
    if message.trim() != EMPTY_RESPONSE_ERR {
        session.push(Role::System, message.clone());
        on_event(StreamEvent::Error(message));
        return false;
    }

    *empty_response_retries = empty_response_retries.saturating_add(1);
    if *empty_response_retries <= EMPTY_RESPONSE_RETRY_LIMIT {
        session.push(
            Role::System,
            format!(
                "{}，自动重试中（{}/{}）",
                EMPTY_RESPONSE_ERR, empty_response_retries, EMPTY_RESPONSE_RETRY_LIMIT
            ),
        );
        llm_messages.push(json!({
            "role": "system",
            "content": EMPTY_RESPONSE_RETRY_PROMPT
        }));
        *step += 1;
        return true;
    }

    let final_message = format!(
        "{}，自动重试 {} 次后仍为空，任务终止",
        EMPTY_RESPONSE_ERR, EMPTY_RESPONSE_RETRY_LIMIT
    );
    session.push(Role::System, final_message.clone());
    on_event(StreamEvent::Error(final_message));
    false
}

#[allow(clippy::too_many_arguments)]
fn handle_structured_tool_calls(
    req: &Request,
    session: &mut Session,
    ctx: &ToolRuntimeContext,
    allowed_tools: &HashSet<String>,
    base_system: &str,
    llm_messages: &mut Vec<Value>,
    total_usage: &mut models::TokenUsage,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut ToolSessionState,
    session_protocol: &mut DesktopSessionProtocol,
    step_out: &llm_runner::LlmStep,
    step_text_preview: &str,
    parallel_tools: bool,
    is_last_step: bool,
    step_index: u32,
) -> bool {
    let structured_tool_call_count = step_out.tool_calls.len();
    llm_messages.push(utils::tool_calls_to_assistant_message(
        &step_out.tool_calls,
        &step_out.reasoning_content,
    ));
    todos::maybe_mark_todo_in_progress(session, ctx, tool_state);

    let step_has_todowrite = step_out
        .tool_calls
        .iter()
        .any(|call| crate::app::agent::tools::is_todo_write_tool_id(&call.name));
    let non_todo_runs_before = tool_state.non_todo_tool_runs;
    let tool_context = |tool_call_id: &str| {
        if let Some(message_id) = req.assistant_message_id.as_deref() {
            ctx.clone().with_tool_use_context(
                ctx.tool_use_context()
                    .as_ref()
                    .clone()
                    .with_message_id(message_id.to_string())
                    .with_tool_call_id(tool_call_id.to_string()),
            )
        } else {
            ctx.clone().with_tool_use_context(
                ctx.tool_use_context().as_ref().clone().with_tool_call_id(tool_call_id.to_string()),
            )
        }
    };

    let mut call_index = 0;
    while call_index < step_out.tool_calls.len() {
        let call = &step_out.tool_calls[call_index];
        let content = if is_last_step {
            format!(
                "tool unavailable: maximum tool iteration budget reached; \
                 `{}` was not rejected by the permission policy",
                call.name
            )
        } else if !allowed_tools.contains(call.name.as_str()) {
            format!("tool denied: {}", call.name)
        } else if tools_exec::is_agent_tool(&call.name) {
            let group_start = call_index;
            while call_index < step_out.tool_calls.len() {
                let grouped_call = &step_out.tool_calls[call_index];
                if !tools_exec::is_agent_tool(&grouped_call.name)
                    || !allowed_tools.contains(grouped_call.name.as_str())
                {
                    break;
                }
                call_index += 1;
            }
            let grouped_calls = &step_out.tool_calls[group_start..call_index];
            let planned = grouped_calls
                .iter()
                .map(|grouped_call| {
                    tools_exec::AgentToolPlannedCall::new(
                        &grouped_call.name,
                        &grouped_call.arguments,
                        tool_context(&grouped_call.id),
                    )
                })
                .collect::<Vec<_>>();
            let current_agent = req
                .options
                .get("agent")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .unwrap_or("main");
            let max_parallel = if parallel_tools {
                crate::app::agent::tools::agent_tool_parallel_limit_for_context(ctx, current_agent)
            } else {
                1
            };
            let contents = tools_exec::run_agent_tools_and_record(
                session,
                &planned,
                max_parallel,
                true,
                on_event,
                tool_state,
            );
            for (grouped_call, content) in grouped_calls.iter().zip(contents) {
                llm_messages.push(utils::tool_result_to_message(&grouped_call.id, &content));
            }
            continue;
        } else {
            let output = tools_exec::run_tool_and_record_with_protocol(
                session,
                &call.name,
                &call.arguments,
                &tool_context(&call.id),
                true,
                on_event,
                tool_state,
            );
            if let Some(output) = output {
                session_protocol.consume_tool_output(
                    &call.name,
                    &call.arguments,
                    &output,
                    llm_messages,
                );
                output.content_for_model
            } else {
                String::new()
            }
        };
        llm_messages.push(utils::tool_result_to_message(&call.id, &content));
        call_index += 1;
    }

    let non_todo_runs_after = tool_state.non_todo_tool_runs;
    let ran_todo_update = non_todo_runs_after > non_todo_runs_before
        && !step_has_todowrite
        && todo_updates::maybe_update_todos_after_work(
            session,
            ctx,
            req.model.clone(),
            base_system,
            allowed_tools,
            llm_messages,
            total_usage,
            on_event,
            tool_state,
            &req.reliability,
        );
    tracing::debug!(
        target: "vw_agent",
        session_id = %req.session,
        step_index,
        tool_call_count = structured_tool_call_count,
        step_has_todowrite,
        non_todo_runs_before,
        non_todo_runs_after,
        ran_todo_update,
        text_preview = %step_text_preview,
        "session processor continuing after structured tool calls"
    );
    ran_todo_update
}

#[allow(clippy::too_many_arguments)]
fn handle_assistant_text_branch(
    req: &Request,
    session: &mut Session,
    ctx: &ToolRuntimeContext,
    allowed_tools: &HashSet<String>,
    llm_messages: &mut Vec<Value>,
    total_usage: &mut models::TokenUsage,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    tool_state: &mut ToolSessionState,
    empty_response_retries: &mut usize,
    step: &mut usize,
    step_out: &llm_runner::LlmStep,
    step_text_preview: &str,
    empty_tools: &HashSet<String>,
    is_last_step: bool,
    step_index: u32,
    app_session_scope: Option<&str>,
) -> bool {
    let mut ran_tool = false;
    let session_message_count_before_tool_parse = session.messages.len();
    let assistant_text = utils::ingest_assistant_answer(
        session,
        &step_out.text,
        ctx,
        empty_tools,
        on_event,
        &mut ran_tool,
        tool_state,
    );
    let assistant_text_preview = response_preview(&assistant_text);
    tracing::info!(
        target: "vw_agent",
        session_id = %req.session,
        step_index,
        ran_inline_tool = ran_tool,
        assistant_text_preview = %assistant_text_preview,
        raw_text_preview = %step_text_preview,
        "session processor evaluated assistant text branch"
    );

    if ran_tool {
        *empty_response_retries = 0;
        llm_messages::extend_llm_messages_from_session_range(
            llm_messages,
            session,
            session_message_count_before_tool_parse,
        );

        let inline_text = assistant_text.trim();
        if !inline_text.is_empty() {
            session.push(Role::Assistant, inline_text.to_string());
            llm_messages.push(utils::assistant_message_with_reasoning(
                inline_text,
                &step_out.reasoning_content,
            ));
        }

        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            assistant_text_preview = %assistant_text_preview,
            "session processor continuing after inline tool execution parsed from assistant text"
        );
        if !on_event(StreamEvent::PostToolRound { step_index }) {
            return false;
        }
        *step += 1;
        return true;
    }

    let text = assistant_text.trim();
    if text.is_empty() {
        return handle_empty_assistant_text(
            req,
            session,
            llm_messages,
            on_event,
            empty_response_retries,
            step,
            &step_out.reasoning_content,
            is_last_step,
            step_index,
        );
    }

    *empty_response_retries = 0;
    session.push(Role::Assistant, text.to_string());
    llm_messages.push(utils::assistant_message_with_reasoning(text, &step_out.reasoning_content));

    if is_last_step {
        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            is_last_step = true,
            final_text_preview = %response_preview(text),
            "session processor accepted final-step assistant response and completed turn"
        );
        if !tools_exec::finalize_agent_tool_children(&req.session, session, on_event, tool_state) {
            return false;
        }
        artifacts::persist_final_ai_call_payload(
            req,
            session,
            total_usage,
            text,
            app_session_scope,
            true,
        );
        on_event(StreamEvent::Done(total_usage.clone()));
        return false;
    }

    tracing::info!(
        target: "vw_agent",
        session_id = %req.session,
        step_index,
        final_text_preview = %response_preview(text),
        "session processor accepted assistant response without tool calls"
    );

    // TodoWrite 可用时强制续轮：未完成 todo 不得提前结束，避免模型“口头完成”后意外退出。
    let enforce_todos =
        allowed_tools.iter().any(|name| crate::app::agent::tools::is_todo_write_tool_id(name));
    let incomplete = enforce_todos && todos::has_incomplete_todos(ctx);

    if enforce_todos && incomplete {
        let message = "任务未完成：todo 列表仍有未完成项，请继续推进直至全部完成，再输出最终答复。"
            .to_string();
        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            "session processor continuing because enforce_todos found incomplete items"
        );
        session.push(Role::User, message.clone());
        llm_messages.push(json!({ "role": "user", "content": message.clone() }));
        if !on_event(StreamEvent::Delta(format!("\n\n{}\n\n", message))) {
            return false;
        }
        *step += 1;
        return true;
    }

    tracing::info!(
        target: "vw_agent",
        session_id = %req.session,
        step_index,
        final_text_preview = %response_preview(text),
        "session processor completed turn with assistant response"
    );
    if !tools_exec::finalize_agent_tool_children(&req.session, session, on_event, tool_state) {
        return false;
    }
    artifacts::persist_final_ai_call_payload(
        req,
        session,
        total_usage,
        text,
        app_session_scope,
        req.persist_app_session_artifacts,
    );
    on_event(StreamEvent::Done(total_usage.clone()));
    false
}

fn handle_empty_assistant_text(
    req: &Request,
    session: &mut Session,
    llm_messages: &mut Vec<Value>,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    empty_response_retries: &mut usize,
    step: &mut usize,
    reasoning_content: &str,
    is_last_step: bool,
    step_index: u32,
) -> bool {
    *empty_response_retries = empty_response_retries.saturating_add(1);
    if is_last_step {
        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            is_last_step,
            empty_response_retries,
            "session processor saw empty final-step assistant text"
        );
    } else {
        tracing::info!(
            target: "vw_agent",
            session_id = %req.session,
            step_index,
            is_last_step,
            empty_response_retries,
            "session processor saw empty non-final assistant text"
        );
    }

    if *empty_response_retries > EMPTY_RESPONSE_RETRY_LIMIT {
        let final_message = format!(
            "{}，自动重试 {} 次后仍为空，任务终止",
            EMPTY_RESPONSE_ERR, EMPTY_RESPONSE_RETRY_LIMIT
        );
        session.push(Role::System, final_message.clone());
        on_event(StreamEvent::Error(final_message));
        return false;
    }

    session.push(
        Role::System,
        format!(
            "{}，自动重试中（{}/{}）",
            EMPTY_RESPONSE_ERR, empty_response_retries, EMPTY_RESPONSE_RETRY_LIMIT
        ),
    );
    llm_messages.push(utils::assistant_message_with_reasoning("", reasoning_content));
    llm_messages.push(json!({
        "role": "system",
        "content": EMPTY_RESPONSE_RETRY_PROMPT
    }));
    *step += 1;
    true
}

fn build_image_turn_context_for_request(
    req: &Request,
    session: &Session,
    config: &crate::config::Config,
) -> Result<Option<crate::app::agent::multimodal::context::ImageTurnContext>, String> {
    use crate::app::agent::multimodal::DISABLE_OCR_OPTION;
    use crate::app::agent::multimodal::assets::{ImageAssetScope, ImageAssetStore};
    use crate::app::agent::multimodal::context::ImageTurnContext;
    use crate::app::agent::multimodal::ocr::SessionImageTextExtractor;
    use crate::app::agent::multimodal::{contains_image_markers, parse_image_markers};
    use crate::app::agent::providers::ChatMessage;

    // OCR itself is routed through the local gateway. Its nested request still
    // contains the image marker, so it must reach the configured vision model
    // without entering this preprocessing path again.
    if req.options.get(DISABLE_OCR_OPTION).and_then(Value::as_bool) == Some(true) {
        return Ok(None);
    }

    let resume_history_only =
        req.options.get("desktop_resume_history_only").and_then(Value::as_bool).unwrap_or(false);

    let history: Vec<ChatMessage> = session
        .messages
        .iter()
        .filter(|message| matches!(message.role, Role::User | Role::Assistant | Role::System))
        .map(|message| match message.role {
            Role::User => ChatMessage::user(&message.content),
            Role::Assistant => ChatMessage::assistant(&message.content),
            Role::System => ChatMessage::system(&message.content),
            Role::Tool => ChatMessage::assistant(&message.content),
        })
        .collect();

    if !contains_image_markers(&history) && parse_image_markers(&req.query).1.is_empty() {
        return Ok(None);
    }

    // 当前轮用户内容：非 resume 时取最后一条 user；resume 时全部按历史处理。
    let (history_messages, current_user_content) = if resume_history_only {
        (history, None)
    } else {
        let mut prior = history;
        let current = prior
            .iter()
            .rposition(|message| message.role == "user")
            .map(|index| prior.remove(index).content);
        (prior, current)
    };

    let scope =
        ImageAssetScope::from_request_context(req.app_session_scope.as_deref(), &req.session)
            .map_err(|error| format!("invalid image asset scope: {error}"))?;
    let store = ImageAssetStore::from_global_paths();
    let extractor = SessionImageTextExtractor::new();
    match prompt::block_on(ImageTurnContext::build(
        &store,
        scope,
        &history_messages,
        current_user_content.as_deref(),
        resume_history_only,
        &config.multimodal,
        req.model.as_deref(),
        &extractor,
    )) {
        Ok(ctx) if !ctx.images.is_empty() => Ok(Some(ctx)),
        Ok(_) => Ok(None),
        Err(error) => {
            if config.multimodal.ocr.enabled && !config.multimodal.ocr.model_supports_images {
                // OCR 开启时必须把失败暴露给用户，不能静默回退成直传图片。
                Err(format!("image OCR preprocessing failed: {error}"))
            } else {
                tracing::warn!(
                    target: "vw_agent",
                    session_id = %req.session,
                    error = %error,
                    "failed to build image turn context; continuing without asset reuse"
                );
                Ok(None)
            }
        }
    }
}

#[cfg(test)]
#[path = "runner_tests.rs"]
mod runner_tests;
