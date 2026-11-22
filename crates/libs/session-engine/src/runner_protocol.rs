//! Request protocol state and context checkpoint decisions for the session runner.
use crate::{context_governance, llm_messages};
use crate::helpers::is_acp_request;
use crate::session_memory::Session;
use crate::processor_events::StreamEvent;
use session_port::CompactionPhase;
use serde_json::{Value, json};
use std::borrow::Cow;
use std::collections::{BTreeSet, HashSet};

/// Existing scoped checkpoint storage and plugin lifecycle operations.
pub trait RunnerCheckpointHost {
    fn restore_skills(session: &str, agent: &str, history: &[Value]) -> Result<Vec<agent_skills::invoked::InvokedSkillRecord>, String>;
    fn save_skills(session: &str, agent: &str, history: &[Value], records: &[agent_skills::invoked::InvokedSkillRecord]) -> Result<(), String>;
    fn save_context(
        session: &str,
        source: &[Value],
        summary: &str,
        retained_user: Option<Value>,
        retained_tail: Vec<Value>,
    ) -> Result<(), String>;
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

    /// Assemble the continuation with the same catalog reset as a committed compaction.
    pub fn assemble_after_compaction(
        &mut self,
        history: &[Value],
        instructions: &[session_instructions::Resolved],
        skill_tool_enabled: bool,
    ) -> Result<llm_messages::AssembledProtocolMessages, String> {
        let previous = self.skill_state.clone();
        self.skill_state.reset_catalog(&self.agent_id);
        let assembled = self.assemble(history, instructions, skill_tool_enabled);
        self.skill_state = previous;
        assembled
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

/// A function call message may start a parallel batch. Native search pairs and
/// tool-supplied context can appear between that message and its final result.
fn completed_tool_groups(messages: &[Value]) -> Vec<std::ops::Range<usize>> {
    let mut groups = Vec::new();
    let mut start = None;
    let mut pending_functions = HashSet::new();
    let mut pending_search: Option<String> = None;

    for (index, message) in messages.iter().enumerate() {
        let valid = match (message["role"].as_str(), message["tool_calls"].as_array()) {
            (Some("assistant"), Some(calls)) => {
                start = Some(index);
                pending_functions.clear();
                pending_search = None;
                !calls.is_empty()
                    && calls.iter().all(|call| {
                        call["id"].as_str().is_some_and(|id| {
                            !id.trim().is_empty()
                                && call["function"]["name"]
                                    .as_str()
                                    .is_some_and(|name| !name.trim().is_empty())
                                && pending_functions.insert(id)
                        })
                    })
            }
            (Some("tool"), _) => {
                message["tool_call_id"].as_str().is_some_and(|id| pending_functions.remove(id))
            }
            #[cfg(not(target_arch = "wasm32"))]
            (Some("native"), _) => {
                use crate::native_history::NativeHistoryItem;
                use aisdk::core::ToolSearchExecution;

                match message["content"]
                    .as_str()
                    .and_then(|content| serde_json::from_str::<NativeHistoryItem>(content).ok())
                {
                    Some(NativeHistoryItem::ToolSearchCall(call))
                        if call.execution == ToolSearchExecution::Client
                            && pending_search.is_none() =>
                    {
                        start.get_or_insert(index);
                        pending_search = call.call_id.filter(|id| !id.trim().is_empty());
                        pending_search.is_some()
                    }
                    Some(NativeHistoryItem::ToolSearchOutput(output))
                        if output.execution == ToolSearchExecution::Client =>
                    {
                        pending_search.take().is_some_and(|id| output.call_id == Some(id))
                    }
                    _ => false,
                }
            }
            (Some("user"), _) => true, // For example, an image attached by pdf_read.
            _ => false,
        };
        if !valid {
            start = None;
            pending_functions.clear();
            pending_search = None;
        } else if pending_functions.is_empty()
            && pending_search.is_none()
            && let Some(start) = start.take()
        {
            groups.push(start..index + 1);
        }
    }
    groups
}

fn compaction_history(
    history: &[Value],
    user_index: usize,
    model_limit: &model_types::ModelLimit,
    request_overhead_tokens: u64,
) -> (Vec<Value>, Vec<Value>) {
    const MAX_RESULT_PREVIEW_BYTES: u64 = 3 * 1024;

    let tail = &history[user_index + 1..];
    let window = context_governance::effective_input_window(model_limit);
    let user_tokens =
        context_governance::context_usage_tokens(&history[user_index..=user_index], &[], &[], None);
    let reserved = request_overhead_tokens
        .saturating_add(user_tokens)
        .saturating_add(crate::compaction_budget::summary_output_budget(model_limit));
    // Share the continuation window with the summary, current user, and request scaffolding.
    let mut budget = (window / 4).min(window.saturating_sub(reserved));
    let mut omit_from_summary = tail
        .iter()
        .map(|message| {
            matches!(message["role"].as_str(), Some("tool" | "native"))
                || (message["role"] == "assistant" && message["tool_calls"].is_array())
        })
        .collect::<Vec<_>>();
    let mut retained_groups = Vec::new();

    for range in completed_tool_groups(tail).into_iter().rev() {
        if budget == 0 {
            omit_from_summary[range].fill(false);
            continue;
        }
        let mut group = tail[range.clone()].to_vec();
        let results = group.iter().filter(|message| message["role"] == "tool").count().max(1);
        let result_bytes =
            MAX_RESULT_PREVIEW_BYTES.min(budget.saturating_mul(2) / results as u64) as usize;
        for message in group.iter_mut().filter(|message| message["role"] == "tool") {
            let content = &message["content"];
            let text = match content {
                Value::String(text) => Cow::Borrowed(text.as_str()),
                Value::Object(_) | Value::Array(_) => Cow::Owned(content.to_string()),
                _ => continue,
            };
            if text.len() > result_bytes {
                message["content"] =
                    Value::String(crate::tool_output::truncate_string_middle(&text, result_bytes));
            }
        }
        let tokens = context_governance::context_usage_tokens(&group, &[], &[], None);
        let retain = tokens <= budget;
        // Complete groups either travel together to the continuation or remain
        // together in the summary input. Unmatched protocol items go to neither.
        omit_from_summary[range].fill(retain);
        if retain {
            budget -= tokens;
            retained_groups.push(group);
        }
    }

    let summary_input = history[..=user_index]
        .iter()
        .chain(
            tail.iter()
                .zip(omit_from_summary)
                .filter_map(|(message, omit)| (!omit).then_some(message)),
        )
        .cloned()
        .collect();
    let retained = retained_groups.into_iter().rev().flatten().collect();
    (summary_input, retained)
}

/// Maps the raw checkpoint source to the live prefix eligible for compaction.
pub struct CompactionBoundary {
    pub source: Vec<Value>,
    pub live_prefix_len: usize,
    pub hidden_prefetch: std::ops::Range<usize>,
}

/// Summarize completed history around the user position captured at session ingestion.
/// Replace model context after checkpoint persistence and move the index to the retained user.
#[allow(clippy::too_many_arguments)] // Keep request identity, budget, and lifecycle explicit.
pub fn compact_transient_history_for_request<H: RunnerCheckpointHost + crate::compaction_summary::SummaryHost>(
    session_id: &str,
    mut boundary: Option<&mut CompactionBoundary>,
    llm_messages: &mut Vec<Value>,
    current_user_index: &mut Option<usize>,
    model: &str,
    reliability: &agent_config::runtime::ReliabilityConfig,
    on_event: &mut impl FnMut(StreamEvent) -> bool,
    cancellation: Option<&tokio_util::sync::CancellationToken>,
    model_limit: &model_types::ModelLimit,
    request_overhead_tokens: u64,
    validate_candidate: impl FnOnce(&[Value]) -> Result<bool, String>,
) -> Result<bool, String> {
    let Some(index) = *current_user_index else {
        return Ok(false);
    };
    let current_user = llm_messages
        .get(index)
        .filter(|message| message["role"] == "user")
        .ok_or_else(|| "invalid compaction user index".to_string())?;
    let prefix_len = boundary.as_ref().map_or(llm_messages.len(), |b| b.live_prefix_len);
    let prefix = llm_messages.get(..prefix_len).ok_or("invalid compaction history boundary")?;
    if index >= prefix_len {
        return Err("compaction user lies past the history boundary".into());
    }
    let (prefix, prefix_user_index) = match boundary.as_ref().map(|b| &b.hidden_prefetch) {
        Some(prefetch) if !prefetch.is_empty() => {
            if prefetch.contains(&index) {
                return Err("compaction user lies within transient prefetch".into());
            }
            let before_prefetch =
                prefix.get(..prefetch.start).ok_or("invalid compaction prefetch boundary")?;
            let after_prefetch =
                prefix.get(prefetch.end..).ok_or("invalid compaction prefetch boundary")?;
            let without_prefetch =
                before_prefetch.iter().chain(after_prefetch).cloned().collect::<Vec<_>>();
            let before_user = prefetch.end.min(index).saturating_sub(prefetch.start);
            (Cow::Owned(without_prefetch), index - before_user)
        }
        _ => (Cow::Borrowed(prefix), index),
    };
    if prefix.len() <= 1 {
        return Ok(false);
    }
    if !on_event(StreamEvent::Compaction { phase: CompactionPhase::Started }) {
        return Err("stream consumer stopped".to_string());
    }
    H::compact_hook(session_id, session_id, model, true, false)?;
    let delta = &llm_messages[prefix_len..];
    let reserved_tokens = request_overhead_tokens
        .saturating_add(context_governance::context_usage_tokens(delta, &[], &[], None));
    let (summary_input, retained) =
        compaction_history(&prefix, prefix_user_index, model_limit, reserved_tokens);
    let summary = crate::compaction_summary::summarize::<H>(
        session_id,
        &summary_input,
        model,
        reliability,
        cancellation,
        model_limit,
    )?;
    let retained_user = current_user.clone();
    let mut replacement = vec![
        json!({ "role": "assistant", "content": format!("[Compaction summary]\n{summary}") }),
        retained_user.clone(),
    ];
    replacement.extend_from_slice(&retained);
    let replacement_prefix_len = replacement.len();
    replacement.extend_from_slice(delta);
    if !validate_candidate(&replacement)? {
        return Ok(false);
    }
    if cancellation.is_some_and(|token| token.is_cancelled()) {
        return Err("执行已取消".into());
    }
    if let Some(boundary) = boundary.as_mut() {
        H::save_context(session_id, &boundary.source, &summary, Some(retained_user), retained)?;
        boundary.live_prefix_len = replacement_prefix_len;
        boundary.hidden_prefetch = 0..0;
    }
    *llm_messages = replacement;
    *current_user_index = Some(1);
    if let Err(error) = H::compact_hook(session_id, session_id, model, true, true) {
        tracing::warn!(session_id, %error, "post-compaction hook failed");
    }
    if !on_event(StreamEvent::Compaction { phase: CompactionPhase::Completed }) {
        return Err("stream consumer stopped".into());
    }
    Ok(true)
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
