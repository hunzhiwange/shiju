//! Request protocol state and context checkpoint decisions for the session runner.
use crate::{context_governance, llm_messages};
use crate::helpers::is_acp_request;
use crate::session_memory::Session;
use crate::processor_events::StreamEvent;
use session_port::CompactionPhase;
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashSet};

/// Existing scoped checkpoint storage and plugin lifecycle operations.
pub trait RunnerCheckpointHost {
    fn restore_skills(session: &str, agent: &str, history: &[Value]) -> Result<Vec<agent_skills::invoked::InvokedSkillRecord>, String>;
    fn save_skills(session: &str, agent: &str, history: &[Value], records: &[agent_skills::invoked::InvokedSkillRecord]) -> Result<(), String>;
    fn save_context(session: &str, source: &[Value], summary: &str, retained_user: Option<Value>) -> Result<(), String>;
    fn compact_hook(session: &str, conversation: &str, model: &str, automatic: bool, completed: bool) -> Result<(), String>;
}

/// Desktop 单轮执行期间的临时协议状态。
///
/// 状态只控制 provider 请求的临时消息，不能进入 `Session`，以免 UI 历史出现元提醒。
#[derive(Default)]
pub struct DesktopSessionProtocol {
    pub skills: Vec<agent_skills::Skill>,
    pub skill_state: agent_skills::session_state::SkillListingSessionState,
    pub agent_id: String,
    context_window_tokens: usize,
    pub hidden_prefetch: std::ops::Range<usize>,
    pub native_call_ids: HashSet<String>,
    pub persisted_history: Vec<Value>,
    pub persisted_history_len: usize,
}

impl DesktopSessionProtocol {
    pub fn new(
        skills: Vec<agent_skills::Skill>,
        agent_id: impl Into<String>,
        context_window_tokens: usize,
    ) -> Self {
        Self { skills, agent_id: agent_id.into(), context_window_tokens, ..Self::default() }
    }

    pub fn assemble(
        &mut self,
        history: &[Value],
        instructions: &[session_instructions::Resolved],
        skill_tool_enabled: bool,
    ) -> Result<llm_messages::AssembledProtocolMessages, String> {
        self.skill_state.validate_invoked_sources(&self.agent_id, &self.skills)?;
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
        if let Some(delta) = self.skill_state.prepare(
            &self.agent_id,
            &self.skills,
            skill_tool_enabled,
            self.context_window_tokens,
        ) {
            tracing::trace!(
                sent_skills = delta.skill_names.len(),
                deferred_skills = delta.deferred_skill_names.len(),
                payload_bytes = delta.content.len(),
                estimated_tokens = delta.estimated_tokens,
                budget_tokens = delta.budget_tokens,
                "prepared skill catalog increment"
            );
        }
        let skill_listing =
            self.skill_state.sent_catalog(&self.agent_id).map(|(content, skill_names)| {
                let source = llm_messages::context_message_source("skill_catalog", &skill_names);
                llm_messages::sourced_protocol_message(
                    json!({ "role": "developer", "content": content }),
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

    pub fn prepare_native_skill_context<H: RunnerCheckpointHost>(
        &mut self,
        session: &str,
        options: &Value,
        history: &[Value],
        skill_policy: Option<&crate::capability_policy::CapabilityPolicyResolution>,
        agents_referenced_skills: &BTreeSet<String>,
        user_referenced_skills: &BTreeSet<String>,
    ) -> Result<(), String> {
        // 验证原始默认 ID，不能先与已发现项求交而把丢失/禁用项静默丢弃。
        let default_skills = option_string_values(options, "delegate_default_allowed_skills")
            .into_iter()
            .map(|name| name.to_lowercase())
            .collect::<BTreeSet<_>>();
        if let Some(resolution) = skill_policy
            && let Some(missing) =
                default_skills.difference(&resolution.default_injected_ids).next()
        {
            return Err(format!("默认技能不存在、已禁用或不在当前权限范围内：{missing}"));
        }
        // Outbound ACP resolves directory mounts at the adapter boundary. Native
        // preloads and checkpoint replay would send expanded skill bodies instead.
        if is_acp_request(options) {
            return Ok(());
        }

        self.restore_checkpoint::<H>(session, history)?;
        let default_skills = skill_policy
            .map(|resolution| &resolution.default_injected_ids)
            .unwrap_or(&default_skills);
        self.preload_skills(default_skills)?;
        self.preload_skills(agents_referenced_skills)?;
        self.skill_state.preload_explicit(&self.agent_id, &self.skills, user_referenced_skills)
    }

    pub fn preload_skills(&mut self, skill_names: &BTreeSet<String>) -> Result<(), String> {
        self.skill_state.preload_defaults(&self.agent_id, &self.skills, skill_names)
    }

    pub fn consume_tool_output(
        &mut self,
        tool_name: &str,
        extra_messages: &[tool_types::ToolExtraMessage],
        llm_messages: &mut Vec<Value>,
    ) {
        // Skill bodies are rebuilt from the typed runtime provenance below,
        // rather than copied from model-shaped context_updates or result JSON.
        if tool_name == "skill" {
            return;
        }
        llm_messages.extend(
            extra_messages
                .iter()
                .map(|message| json!({ "role": message.role, "content": message.content })),
        );
    }

    pub fn restore_checkpoint<H: RunnerCheckpointHost>(&mut self, session: &str, history: &[Value]) -> Result<(), String> {
        for record in H::restore_skills(session, &self.agent_id, history)? {
            self.skill_state.register_invoked(&self.agent_id, record);
        }
        self.skill_state.validate_invoked_sources(&self.agent_id, &self.skills)
    }

    pub fn persist_checkpoint<H: RunnerCheckpointHost>(&self, session: &str, history: &[Value]) -> Result<(), String> {
        if history.is_empty() {
            return Ok(());
        }
        H::save_skills(
            session,
            &self.agent_id,
            history,
            self.skill_state.invoked_records(&self.agent_id),
        )
    }

    pub fn checkpoint_history(&self, session: &Session) -> Vec<Value> {
        let mut history = self.persisted_history.clone();
        history.extend(
            session
                .messages
                .iter()
                .enumerate()
                .skip(self.persisted_history_len)
                .filter(|(index, _)| !self.hidden_prefetch.contains(index))
                .flat_map(|(_, message)| llm_messages::session_message_to_llm_messages(message)),
        );
        history
    }

    pub fn consume_invoked_skills<H: RunnerCheckpointHost>(
        &mut self,
        records: Vec<agent_skills::invoked::InvokedSkillRecord>,
        session: &Session,
    ) -> Result<(), String> {
        if records.is_empty() {
            return Ok(());
        }
        for record in records {
            self.skill_state.register_invoked(&self.agent_id, record);
        }
        self.skill_state.validate_invoked_sources(&self.agent_id, &self.skills)?;
        self.persist_checkpoint::<H>(&session.id, &self.checkpoint_history(session))
    }
}

pub fn prepare_context_before_llm_step(
    llm_messages: &mut Vec<Value>,
    system: &[String],
    tools: &[tool_types::ToolSpec],
    model_limit: &model_types::ModelLimit,
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

pub fn provider_request_options(options: &Value) -> Value {
    let mut options = options.clone();
    if let Some(object) = options.as_object_mut() {
        object.retain(|key, _| !key.starts_with("_vw_") && key != "source");
    }
    options
}

/// Summarize completed history, including tool rounds after the latest user request.
/// Preserve that user request verbatim and replace model context only after checkpoint persistence.
#[allow(clippy::too_many_arguments)] // Keep request identity, budget, and lifecycle explicit.
pub fn compact_transient_history_for_request<H: RunnerCheckpointHost + crate::compaction_summary::SummaryHost>(
    session_id: &str,
    checkpoint_source: &[Value],
    llm_messages: &mut Vec<Value>,
    model: &str,
    reliability: &agent_config::runtime::ReliabilityConfig,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
    model_limit: &model_types::ModelLimit,
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
    H::compact_hook(
        session_id, session_id, model, true, false,
    )
    .map_err(|error| error.to_string())?;
    let summary = crate::compaction_summary::summarize::<H>(
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
    persist_context_checkpoint::<H>(session_id, checkpoint_source, &replacement)?;
    *llm_messages = replacement;
    H::compact_hook(session_id, session_id, model, true, true)
        .map_err(|error| error.to_string())?;
    on_event(StreamEvent::Compaction { phase: CompactionPhase::Completed });
    Ok(true)
}

pub fn persist_context_checkpoint<H: RunnerCheckpointHost>(
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
    H::save_context(
        session,
        source,
        summary,
        context.get(1).cloned(),
    )
}

pub fn delegated_skill_policy_resolution(
    skills: &[agent_skills::Skill],
    options: &Value,
) -> Option<crate::capability_policy::CapabilityPolicyResolution> {
    let policy = options.get("delegate_skill_policy").cloned().and_then(|value| {
        serde_json::from_value::<config_agent::DelegateCapabilityPolicy>(value).ok()
    })?;
    let defaults = option_string_values(options, "delegate_default_allowed_skills");
    let boundary = if options.get("delegate_allowed_skills").is_some() {
        option_string_values(options, "delegate_allowed_skills")
    } else {
        defaults.clone()
    };
    Some(crate::capability_policy::resolve_skill_defaults(
        &skills.iter().map(|skill| skill.invocation_name().to_string()).collect::<Vec<_>>(),
        &boundary,
        &defaults,
        policy,
    ))
}

pub fn option_string_values(options: &Value, key: &str) -> Vec<String> {
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

#[cfg(test)]
#[path = "runner_protocol_tests.rs"]
mod tests;
