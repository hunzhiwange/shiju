//! The core entry points reuse the processor's trusted catalog and source replay.

use agent_config::runtime::Config;
use model_types::ChatMessage;
use agent_skills::{types::Skill, session_state::SkillListingSessionState};
use tool_types::ToolSpec;
use tool_context::{context::ToolUseContext, host::ContextHost};
use agent_skills::invoked::InvokedSkillRecord;
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use config_agent::AgentDefinitionConfig;

#[derive(Clone)]
pub struct CoreSkillSession<H: CoreSkillHost> {
    host: std::marker::PhantomData<H>,
    pub(crate) config: Option<Arc<Config>>,
    pub(crate) context_window_tokens: usize,
    pub(crate) session_id: Option<String>,
    workspace_dir: PathBuf,
    skills: Vec<Skill>,
    definition: AgentDefinitionConfig,
    state: SkillListingSessionState,
    source_history: Vec<Value>,
    query: String,
    instructions: String,
}

impl<H: CoreSkillHost> CoreSkillSession<H> {
    pub fn new(
        workspace_dir: PathBuf,
        skills: Vec<Skill>,
        definition: AgentDefinitionConfig,
    ) -> Self {
        Self {
            host: std::marker::PhantomData,
            config: None,
            context_window_tokens: 0,
            session_id: None,
            workspace_dir,
            skills,
            definition,
            state: SkillListingSessionState::default(),
            source_history: Vec::new(),
            query: String::new(),
            instructions: String::new(),
        }
    }

    pub fn from_config(
        config: Arc<Config>,
        session_id: &str,
        context_window_tokens: usize,
    ) -> Self {
        let mut session = Self::new(
            config.workspace_dir.clone(),
            Vec::new(),
            config_agent::effective_main_agent_config(&config.agents),
        );
        session.config = Some(config);
        session.session_id = Some(session_id.to_string());
        session.context_window_tokens = context_window_tokens;
        session
    }

    pub fn begin_turn(&mut self, query: &str, instructions: &str, history: &[ChatMessage]) {
        if let Some(config) = &self.config {
            self.skills = H::load_skills_with_config(&self.workspace_dir, config);
        }
        self.query = query.to_string();
        // Only the workspace instruction source can preload instruction-selected
        // skills. Tool descriptions, workspace names, and runtime metadata are
        // not user selections, even when they mention an available skill.
        self.instructions = if instructions.contains("### AGENTS.md\n") {
            std::fs::read_to_string(self.workspace_dir.join("AGENTS.md")).unwrap_or_default()
        } else {
            String::new()
        };
        self.source_history = history
            .iter()
            .map(|message| json!({"role":message.role,"content":message.content}))
            .collect();
    }

    pub async fn set_model_context(&mut self, model: &str) -> Result<()> {
        if self.skills.is_empty() && self.definition.allowed_skills.is_empty() {
            return Ok(());
        }
        let parsed = agent_provider::provider::parse_model(model);
        self.context_window_tokens =
            match H::get_model(&parsed.provider_id, &parsed.model_id).await
            {
                Ok(model) => usize::try_from(model.limit.context).unwrap_or(usize::MAX),
                Err(error @ agent_provider::provider::CatalogError::Configuration(_)) => {
                    return Err(error.into());
                }
                Err(agent_provider::provider::CatalogError::ModelNotFound(_)) => 0,
            };
        Ok(())
    }

    pub async fn project_context(
        &self,
    ) -> Result<Option<agent_project::instance::Context>> {
        if (self.config.is_none()
            && self.skills.is_empty()
            && self.definition.allowed_skills.is_empty())
            || std::path::Path::new(&agent_project::instance::directory()) == self.workspace_dir
        {
            return Ok(None);
        }
        let tenant = H::current_tenant_scope();
        let (project, worktree) =
            Box::pin(agent_project::from_directory(&tenant, &self.workspace_dir)).await?;
        Ok(Some(agent_project::instance::Context {
            directory: self.workspace_dir.to_string_lossy().to_string(),
            worktree,
            project,
        }))
    }

    pub fn clear(&mut self) {
        self.state = SkillListingSessionState::default();
    }

    pub async fn configure(&mut self, context: ToolUseContext<H::Context>) -> Result<ToolUseContext<H::Context>> {
        let selected = match &self.definition.defaults.skills {
            config_agent_profile::AgentDefault::Inherit => &self.definition.allowed_skills,
            config_agent_profile::AgentDefault::Fixed(ids) => ids,
        };
        let defaults = selected
            .iter()
            .filter_map(|name| session_engine::capability_policy::normalize_capability_id(name))
            .collect::<BTreeSet<_>>();
        let resolution = session_engine::capability_policy::resolve_skill_defaults(
            &self
                .skills
                .iter()
                .map(|skill| skill.invocation_name().to_string())
                .collect::<Vec<_>>(),
            &self.definition.allowed_skills,
            &defaults.iter().cloned().collect::<Vec<_>>(),
            self.definition.skill_policy,
        );
        if let Some(missing) = defaults.difference(&resolution.default_injected_ids).next() {
            bail!("默认技能不存在、已禁用或不在当前权限范围内：{missing}");
        }
        self.skills.retain(|skill| {
            resolution.visible_ids.contains(&skill.invocation_name().trim().to_lowercase())
        });
        let user_references =
            H::explicitly_referenced_skill_names(&self.skills, &self.query);
        let instruction_references = H::explicitly_referenced_skill_names(
            &self.skills,
            &self.instructions,
        );
        let mut explicit = user_references.clone();
        explicit.extend(instruction_references.iter().cloned());
        if H::user_explicitly_requests_skill_use(&self.query) {
            explicit.extend(H::user_invocable_skill_names(&self.skills));
        }
        let context = context
            .with_agent_name("main")
            .with_root(Some(self.workspace_dir.to_string_lossy().to_string()))
            .with_delegate_skill_policy(
                self.definition.skill_policy,
                self.definition.allowed_skills.clone(),
            )
            .with_explicitly_requested_skills(explicit);
        #[cfg(not(target_arch = "wasm32"))]
        if !self.skills.is_empty() {
            self.skills = H::accessible_skills(
                std::mem::take(&mut self.skills),
                Some(&context),
            );
        }
        if let Some(session) = &self.session_id {
            // The raw authoritative history, rather than a summary or model tags,
            // decides which invocation records belong to this request.
            self.state = SkillListingSessionState::default();
            for record in
                H::restore(session, "main", &self.source_history)
                    .await
                    .map_err(anyhow::Error::msg)?
            {
                self.state.register_invoked("main", record);
            }
        }
        self.state.validate_invoked_sources("main", &self.skills).map_err(anyhow::Error::msg)?;
        self.state.preload_defaults("main", &self.skills, &defaults).map_err(anyhow::Error::msg)?;
        self.state
            .preload_defaults("main", &self.skills, &instruction_references)
            .map_err(anyhow::Error::msg)?;
        self.state
            .preload_explicit("main", &self.skills, &user_references)
            .map_err(anyhow::Error::msg)?;
        self.save(&self.source_history).await?;
        Ok(context)
    }

    pub fn refresh_skill_spec(&self, specs: &mut [ToolSpec]) {
        for spec in specs.iter_mut().filter(|spec| spec.id == "skill") {
            spec.description = H::description_for_skills(
                &self.workspace_dir,
                self.skills.clone(),
            );
        }
    }

    pub fn messages(&mut self, skill_tool_enabled: bool) -> Result<Vec<ChatMessage>> {
        self.state.validate_invoked_sources("main", &self.skills).map_err(anyhow::Error::msg)?;
        self.state.prepare("main", &self.skills, skill_tool_enabled, self.context_window_tokens);
        let mut messages = Vec::new();
        if let Some((content, _)) = self.state.sent_catalog("main") {
            messages.push(ChatMessage { role: "developer".into(), content });
        }
        messages.extend(
            self.state
                .restore_invoked("main")
                .map_err(anyhow::Error::msg)?
                .into_iter()
                .map(|replay| ChatMessage::user(replay.content)),
        );
        Ok(messages)
    }

    pub async fn consume(
        &mut self,
        context: &ToolUseContext<H::Context>,
        history: &[ChatMessage],
        initial_len: usize,
    ) -> Result<()> {
        let records = context.take_invoked_skills();
        if records.is_empty() {
            return Ok(());
        }
        for record in records {
            self.state.register_invoked("main", record);
        }
        self.state.validate_invoked_sources("main", &self.skills).map_err(anyhow::Error::msg)?;
        let mut source = self.source_history.clone();
        source.extend(
            history[initial_len..]
                .iter()
                .map(|message| json!({"role":message.role,"content":message.content})),
        );
        self.save(&source).await
    }

    async fn save(&self, history: &[Value]) -> Result<()> {
        if let Some(session) = &self.session_id {
            H::save(
                session,
                "main",
                history,
                self.state.invoked_records("main"),
            )
            .await
            .map_err(anyhow::Error::msg)?;
        }
        Ok(())
    }
}


/// 实际宿主I/O与工具权限动作。技能状态和轮次规则始终由本模块持有。
#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
pub trait CoreSkillHost: Clone + Send + Sync + 'static {
    type Context: ContextHost<InvokedSkillRecord = InvokedSkillRecord>;
    fn current_tenant_scope() -> vw_api_transport::id::TenantScope;
    fn load_skills_with_config(directory: &std::path::Path, config: &Config) -> Vec<Skill>;
    async fn get_model(provider: &str, model: &str) -> std::result::Result<model_types::Model, agent_provider::provider::CatalogError>;
    fn explicitly_referenced_skill_names(skills: &[Skill], text: &str) -> BTreeSet<String>;
    fn user_explicitly_requests_skill_use(text: &str) -> bool;
    fn user_invocable_skill_names(skills: &[Skill]) -> BTreeSet<String>;
    #[cfg(not(target_arch = "wasm32"))]
    fn accessible_skills(skills: Vec<Skill>, context: Option<&ToolUseContext<Self::Context>>) -> Vec<Skill>;
    fn description_for_skills(directory: &std::path::Path, skills: Vec<Skill>) -> String;
    async fn restore(session: &str, agent: &str, history: &[Value]) -> std::result::Result<Vec<InvokedSkillRecord>, String>;
    async fn save(session: &str, agent: &str, history: &[Value], records: &[InvokedSkillRecord]) -> std::result::Result<(), String>;
}
