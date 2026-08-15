//! 会话级查询引擎。
//!
//! 该模块参考 Claude Code 的 QueryEngine 分层，将“运行时装配”和“多轮消息提交”
//! 从 `runner` 中拆开：
//! - `QueryEngine::new` 负责一次性组装 provider、工具、记忆、系统提示
//! - `QueryEngine::submit_message` 负责向既有历史追加用户消息并执行单轮工具循环
//!
//! 这样可以在不改动底层 `core` 工具循环语义的前提下，为后续接入历史压缩、预算、
//! 任务编排和会话持久化提供稳定挂点。

use crate::app::agent::config::Config;
use crate::app::agent::memory::{self, Memory};
use crate::app::agent::observability::{self, Observer, ObserverEvent};
use crate::app::agent::providers::{ChatMessage, Provider};
use crate::app::agent::runtime;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::Tool;
use crate::observability::traits::ObserverMetric;
use crate::session::ui_types as ui_models;
use anyhow::Result;
use chrono_tz::Tz;
use std::any::Any;
use std::sync::{Arc, Mutex};
use vw_config_types::agent::merged_agent_configs;

use super::context::build_context;
use super::core::agent_turn;
use super::history::compact_history_with_internal_service;
use super::instructions::{build_shell_policy_instructions, build_tool_instructions};

/// 会话级查询引擎。
///
/// 该结构体持有一轮会话内可复用的运行时依赖，并维护消息历史，
/// 以便调用方按 Claude Code 的方式持续提交多条用户消息。
#[derive(Debug, Clone, Default)]
pub(crate) struct QueryEngineUsage {
    pub input_tokens: i64,
    pub output_tokens: i64,
    pub cached_tokens: i64,
    pub reasoning_tokens: i64,
    pub llm_calls: usize,
}

impl QueryEngineUsage {
    pub(crate) fn total_tokens(&self) -> i64 {
        self.input_tokens.saturating_add(self.output_tokens).saturating_add(self.cached_tokens)
    }

    pub(crate) fn as_ui_token_usage(&self) -> ui_models::TokenUsage {
        ui_models::TokenUsage {
            input_tokens: self.input_tokens,
            output_tokens: self.output_tokens,
            cached_tokens: self.cached_tokens,
            reasoning_tokens: self.reasoning_tokens,
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct QueryEngineBudgetSnapshot {
    pub max_tool_iterations: usize,
    pub max_history_messages: usize,
    pub non_system_messages: usize,
    pub remaining_history_messages: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct QueryEngineSessionState {
    pub turn_count: usize,
    pub usage: QueryEngineUsage,
    pub budget: QueryEngineBudgetSnapshot,
}

#[derive(Debug, Clone)]
pub(crate) struct QueryEngineSnapshot {
    history: Vec<ChatMessage>,
    turn_count: usize,
    usage: QueryEngineUsage,
}

pub(crate) struct QueryEngine {
    history: Vec<ChatMessage>,
    request_system_prompt: String,
    tools_registry: Vec<Box<dyn Tool>>,
    observer: Arc<RecordingObserver>,
    provider: Arc<dyn Provider>,
    memory: Arc<dyn Memory>,
    provider_name: String,
    model_name: String,
    default_temperature: f64,
    multimodal_config: crate::app::agent::config::MultimodalConfig,
    compact_context: bool,
    model_limit: crate::app::agent::provider::provider::ModelLimit,
    max_tool_iterations: usize,
    max_history_messages: usize,
    min_memory_relevance_score: f64,
    turn_count: usize,
    timezone: Tz,
}

#[derive(Debug, Default)]
struct RecordingObserverState {
    usage: QueryEngineUsage,
}

struct RecordingObserver {
    inner: Arc<dyn Observer>,
    state: Mutex<RecordingObserverState>,
}

impl RecordingObserver {
    fn new(inner: Arc<dyn Observer>) -> Self {
        Self { inner, state: Mutex::new(RecordingObserverState::default()) }
    }

    fn usage_snapshot(&self) -> QueryEngineUsage {
        self.state.lock().expect("recording observer state poisoned").usage.clone()
    }

    fn replace_usage(&self, usage: QueryEngineUsage) {
        self.state.lock().expect("recording observer state poisoned").usage = usage;
    }
}

impl Observer for RecordingObserver {
    fn record_event(&self, event: &ObserverEvent) {
        if let ObserverEvent::LlmResponse {
            success,
            input_tokens,
            output_tokens,
            cached_tokens,
            reasoning_tokens,
            ..
        } = event
            && *success
        {
            let mut state = self.state.lock().expect("recording observer state poisoned");
            state.usage.llm_calls = state.usage.llm_calls.saturating_add(1);
            state.usage.input_tokens = state
                .usage
                .input_tokens
                .saturating_add(i64::try_from(input_tokens.unwrap_or(0)).unwrap_or(i64::MAX));
            state.usage.output_tokens = state
                .usage
                .output_tokens
                .saturating_add(i64::try_from(output_tokens.unwrap_or(0)).unwrap_or(i64::MAX));
            state.usage.cached_tokens = state
                .usage
                .cached_tokens
                .saturating_add(i64::try_from(cached_tokens.unwrap_or(0)).unwrap_or(i64::MAX));
            state.usage.reasoning_tokens = state
                .usage
                .reasoning_tokens
                .saturating_add(i64::try_from(reasoning_tokens.unwrap_or(0)).unwrap_or(i64::MAX));
        }

        self.inner.record_event(event);
    }

    fn record_metric(&self, metric: &ObserverMetric) {
        self.inner.record_metric(metric);
    }

    fn flush(&self) {
        self.inner.flush();
    }

    fn name(&self) -> &str {
        self.inner.name()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl QueryEngine {
    /// 从配置构建一个可复用的会话级引擎。
    pub(crate) async fn new(config: Config, session_id: &str) -> Result<Self> {
        Self::new_with_provider(config, session_id, None).await
    }

    /// 从配置构建引擎，并可复用调用方已经解析好的 provider。
    pub(crate) async fn new_with_provider(
        config: Config,
        session_id: &str,
        provider_override: Option<Arc<dyn Provider>>,
    ) -> Result<Self> {
        let timezone = crate::app::agent::util::locale::parse_timezone(
            &config.app_ui.system_settings.timezone,
        )?;
        let base_observer: Arc<dyn Observer> =
            Arc::from(observability::create_observer(&config.observability));
        let observer = Arc::new(RecordingObserver::new(base_observer));

        let runtime: Arc<dyn runtime::RuntimeAdapter> =
            Arc::from(runtime::create_runtime(&config.runtime)?);

        let security = Arc::new(SecurityPolicy::from_configs(
            &config.autonomy,
            &config.security,
            &config.workspace_dir,
        ));

        let memory: Arc<dyn Memory> = Arc::from(memory::create_memory_with_storage(
            &config.memory,
            Some(&config.storage.provider.config),
            &config.workspace_dir,
            config.api_key.as_deref(),
        )?);

        let tools_registry = crate::app::agent::tools::all_tools_with_runtime(
            Arc::new(config.clone()),
            &security,
            runtime,
            memory.clone(),
            &config.browser,
            &config.http_request,
            &config.web_fetch,
            &config.workspace_dir,
            &config.agents,
            config.api_key.as_deref(),
            &config,
            Some(session_id),
        );

        let provider_name = config.default_provider.as_deref().unwrap_or("openrouter").to_string();
        let model_name = config
            .default_model
            .clone()
            .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".into());

        let provider_runtime_options = crate::app::agent::providers::ProviderRuntimeOptions {
            auth_profile_override: None,
            provider_api_url: config.api_url.clone(),
            vibewindow_dir: config.config_path.parent().map(std::path::PathBuf::from),
            secrets_encrypt: config.secrets.encrypt,
            reasoning_enabled: config.runtime.reasoning_enabled,
            reasoning_level: config.effective_provider_reasoning_level(),
            custom_provider_api_mode: config.provider_api.map(|mode| mode.as_compatible_mode()),
            max_tokens_override: None,
            model_support_vision: config.model_support_vision,
        };

        let provider: Arc<dyn Provider> = if let Some(provider) = provider_override {
            provider
        } else {
            Arc::from(crate::app::agent::providers::create_routed_provider_with_options(
                &provider_name,
                config.api_key.as_deref(),
                config.api_url.as_deref(),
                &config.reliability,
                &config.model_routes,
                &model_name,
                &provider_runtime_options,
            )?)
        };

        let skills =
            crate::app::agent::skills::load_skills_with_config(&config.workspace_dir, &config);
        let tool_descs = build_tool_descriptions(&config);
        let bootstrap_max_chars = if config.agent.compact_context { Some(6000) } else { None };
        let native_tools = provider.supports_native_tools();

        let mut system_prompt = crate::app::agent::channels::build_system_prompt_with_mode(
            &config.workspace_dir,
            &model_name,
            &tool_descs,
            &skills,
            Some(&config.identity),
            bootstrap_max_chars,
            native_tools,
            config.skills.prompt_injection_mode,
            timezone,
        );

        if !native_tools {
            system_prompt.push_str(&build_tool_instructions(&tools_registry));
        }

        system_prompt.push_str(&build_shell_policy_instructions(&config.autonomy));

        let parsed_model = crate::app::agent::provider::provider::parse_model(&model_name);
        let model_limit = crate::app::agent::provider::provider::get_model(
            &parsed_model.provider_id,
            &parsed_model.model_id,
        )
        .await
        .map(|model| model.limit)
        .unwrap_or(crate::app::agent::provider::provider::ModelLimit {
            context: 0,
            input: None,
            output: 0,
        });

        Ok(Self {
            history: Vec::new(),
            request_system_prompt: system_prompt,
            tools_registry,
            observer,
            provider,
            memory,
            provider_name,
            model_name,
            default_temperature: config.default_temperature,
            multimodal_config: config.multimodal,
            compact_context: config.agent.compact_context,
            model_limit,
            max_tool_iterations: config.agent.max_tool_iterations,
            max_history_messages: config.agent.max_history_messages,
            min_memory_relevance_score: config.memory.min_relevance_score,
            turn_count: 0,
            timezone,
        })
    }

    /// 提交一条用户消息并在当前会话历史上执行一次 agent 轮次。
    pub(crate) async fn submit_message(&mut self, message: &str) -> Result<String> {
        let history_snapshot = self.history.clone();
        let memory_context =
            build_context(self.memory.as_ref(), message, self.min_memory_relevance_score).await;

        let now = crate::app::agent::util::locale::configured_datetime(self.timezone);
        let enriched = if memory_context.is_empty() {
            format!("[{now}] {message}")
        } else {
            format!("{memory_context}[{now}] {message}")
        };

        self.history.push(ChatMessage::user(enriched));
        if let Err(error) = self.compact_before_primary_request(false).await {
            self.history = history_snapshot;
            return Err(error);
        }
        let persistent_len = self.history.len();
        let mut request_history = vec![ChatMessage::user(format!(
            "<system-reminder>\n{}\n</system-reminder>",
            self.request_system_prompt
        ))];
        let transient_len = request_history.len();
        request_history.extend(self.history.iter().cloned());

        let result = agent_turn(
            self.provider.as_ref(),
            &mut request_history,
            &self.tools_registry,
            self.observer.as_ref(),
            &self.provider_name,
            &self.model_name,
            self.default_temperature,
            true,
            &self.multimodal_config,
            self.max_tool_iterations,
        )
        .await;

        let mut retry_response_persisted = false;
        let result = match result {
            Err(error) if is_prompt_too_long(&error.to_string()) => {
                if let Err(compaction_error) = self.compact_before_primary_request(true).await {
                    self.history = history_snapshot;
                    return Err(anyhow::anyhow!(
                        "{error}; internal compaction could not recover the request: {compaction_error}"
                    ));
                }
                let persistent_len = self.history.len();
                let mut retry_history = vec![ChatMessage::user(format!(
                    "<system-reminder>\n{}\n</system-reminder>",
                    self.request_system_prompt
                ))];
                let transient_len = retry_history.len();
                retry_history.extend(self.history.iter().cloned());
                let retry = agent_turn(
                    self.provider.as_ref(),
                    &mut retry_history,
                    &self.tools_registry,
                    self.observer.as_ref(),
                    &self.provider_name,
                    &self.model_name,
                    self.default_temperature,
                    true,
                    &self.multimodal_config,
                    self.max_tool_iterations,
                )
                .await;
                if retry.is_ok() {
                    self.history
                        .extend(retry_history.into_iter().skip(transient_len + persistent_len));
                    retry_response_persisted = true;
                }
                retry
            }
            result => result,
        };

        if result.is_err() {
            self.history = history_snapshot;
        } else {
            if !retry_response_persisted {
                self.history
                    .extend(request_history.into_iter().skip(transient_len + persistent_len));
            }
            self.turn_count = self.turn_count.saturating_add(1);
            self.govern_history().await;
        }

        result
    }

    async fn govern_history(&mut self) {
        // History replacement must only happen through the internal summary service before a
        // primary request.  A post-response message-count trim can discard that summary and
        // reintroduce the split compaction policy this engine is avoiding.
    }

    async fn compact_before_primary_request(&mut self, force: bool) -> Result<()> {
        if !self.compact_context {
            return Ok(());
        }
        let input_tokens = self
            .history
            .iter()
            .map(|message| crate::app::agent::util::token::estimate(&message.content))
            .sum();
        if !force
            && !crate::session::processor::context_governance::should_compact(
                input_tokens,
                &self.model_limit,
            )
        {
            return Ok(());
        }
        compact_history_with_internal_service(
            &mut self.history,
            self.provider.as_ref(),
            &self.model_name,
            self.max_history_messages,
        )
        .await
        .map(|_| ())
    }

    /// 返回当前会话已成功提交的轮次数。
    pub(crate) fn turn_count(&self) -> usize {
        self.turn_count
    }

    /// 返回当前会话累计的 LLM 使用量与已消耗预算快照。
    pub(crate) fn session_state(&self) -> QueryEngineSessionState {
        let non_system_messages = self.non_system_message_count();
        QueryEngineSessionState {
            turn_count: self.turn_count,
            usage: self.observer.usage_snapshot(),
            budget: QueryEngineBudgetSnapshot {
                max_tool_iterations: self.max_tool_iterations,
                max_history_messages: self.max_history_messages,
                non_system_messages,
                remaining_history_messages: self
                    .max_history_messages
                    .saturating_sub(non_system_messages),
            },
        }
    }

    /// 导出当前会话历史快照，供 session fork 等上层生命周期管理复用。
    pub(crate) fn history_snapshot(&self) -> Vec<ChatMessage> {
        self.history.clone()
    }

    /// 导出当前引擎快照，供 session fork 等生命周期管理复用。
    pub(crate) fn snapshot(&self) -> QueryEngineSnapshot {
        QueryEngineSnapshot {
            history: self.history.clone(),
            turn_count: self.turn_count,
            usage: self.observer.usage_snapshot(),
        }
    }

    /// 用外部提供的历史覆盖当前会话历史。
    pub(crate) fn replace_history(&mut self, history: Vec<ChatMessage>) {
        self.history = history;
    }

    /// 用已有引擎快照恢复会话态，供 fork 后的子引擎继承父会话进度。
    pub(crate) fn restore_snapshot(&mut self, snapshot: QueryEngineSnapshot) {
        self.history = snapshot.history;
        self.turn_count = snapshot.turn_count;
        self.observer.replace_usage(snapshot.usage);
    }

    fn non_system_message_count(&self) -> usize {
        self.history.iter().filter(|message| message.role != "system").count()
    }
}

fn is_prompt_too_long(message: &str) -> bool {
    let normalized = message.to_ascii_lowercase();
    ["context length", "context window", "prompt too long", "too many tokens", "context overflow"]
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn build_tool_descriptions(config: &Config) -> Vec<(&'static str, &'static str)> {
    let mut tool_descs = vec![
        ("bash", "Execute terminal commands."),
        ("file_read", "Read file contents."),
        ("notebook_edit", "Edit existing notebook cells by structured notebook operations."),
        ("file_edit", "Edit existing file contents by targeted replacement."),
        ("file_write", "Create files or fully overwrite files."),
        ("memory_store", "Save to memory."),
        ("memory_recall", "Search memory."),
        ("memory_forget", "Delete a memory entry."),
        ("model_routing_config", "Configure default model, scenario routing, and delegate agents."),
        ("screenshot", "Capture a screenshot."),
        ("image_info", "Read image metadata."),
    ];

    if config.browser.enabled {
        if !config.browser.browser_open.eq_ignore_ascii_case("disable") {
            tool_descs.push(("BrowserOpen", "Open approved URLs in browser."));
        }
        tool_descs.push(("Browser", "Control a browser for navigation and page interaction."));
    }

    if config.web_fetch.enabled {
        tool_descs
            .push(("WebFetch", "Fetch and convert approved web pages for model consumption."));
    }

    if config.web_search.enabled {
        tool_descs
            .push(("WebSearch", "Search the web for current information and relevant sources."));
    }

    let has_agent_tool = merged_agent_configs(&config.agents)
        .values()
        .any(|agent| agent.enabled && agent.allow_as_subagent);
    if has_agent_tool {
        tool_descs.push((
            "AgentTool",
            "Delegate complex work to persistent sub-agents; skip only genuinely simple single-step tasks. Use launch (or spawn) to start independent work in the background and in parallel, run when the result is an immediate dependency, status to inspect tasks, wait for event-driven any/all completion with a bounded timeout, and stop to cancel owned tasks. Continue useful parent work after launch, then verify and synthesize required results before answering.",
        ));
    }

    tool_descs
}

#[cfg(test)]
#[path = "query_engine_tests.rs"]
mod query_engine_tests;
