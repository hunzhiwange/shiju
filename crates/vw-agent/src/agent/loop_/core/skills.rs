//! The core entry points reuse the processor's trusted catalog and source replay.

use crate::config::Config;
use crate::providers::ChatMessage;
use crate::skills::{Skill, SkillListingSessionState};
use crate::tools::{ToolSpec, ToolUseContext};
use anyhow::{Result, bail};
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;
use vw_config_types::agent::AgentDefinitionConfig;

#[derive(Clone)]
pub(crate) struct CoreSkillSession {
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

impl CoreSkillSession {
    pub(crate) fn new(
        workspace_dir: PathBuf,
        skills: Vec<Skill>,
        definition: AgentDefinitionConfig,
    ) -> Self {
        Self {
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

    pub(crate) fn from_config(
        config: Arc<Config>,
        session_id: &str,
        context_window_tokens: usize,
    ) -> Self {
        let mut session = Self::new(
            config.workspace_dir.clone(),
            Vec::new(),
            vw_config_types::agent::effective_main_agent_config(&config.agents),
        );
        session.config = Some(config);
        session.session_id = Some(session_id.to_string());
        session.context_window_tokens = context_window_tokens;
        session
    }

    pub(crate) fn begin_turn(&mut self, query: &str, instructions: &str, history: &[ChatMessage]) {
        if let Some(config) = &self.config {
            self.skills = crate::skills::load_skills_with_config(&self.workspace_dir, config);
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

    pub(crate) async fn set_model_context(&mut self, model: &str) -> Result<()> {
        if self.skills.is_empty() && self.definition.allowed_skills.is_empty() {
            return Ok(());
        }
        let parsed = crate::provider::provider::parse_model(model);
        self.context_window_tokens =
            match crate::provider::provider::get_model(&parsed.provider_id, &parsed.model_id).await
            {
                Ok(model) => usize::try_from(model.limit.context).unwrap_or(usize::MAX),
                Err(error @ crate::provider::provider::CatalogError::Configuration(_)) => {
                    return Err(error.into());
                }
                Err(crate::provider::provider::CatalogError::ModelNotFound(_)) => 0,
            };
        Ok(())
    }

    pub(crate) async fn project_context(
        &self,
    ) -> Result<Option<crate::project::instance::Context>> {
        if (self.config.is_none()
            && self.skills.is_empty()
            && self.definition.allowed_skills.is_empty())
            || std::path::Path::new(&crate::project::instance::directory()) == self.workspace_dir
        {
            return Ok(None);
        }
        let tenant = crate::session::ui_store::current_tenant_scope();
        let (project, worktree) =
            Box::pin(crate::project::from_directory(&tenant, &self.workspace_dir)).await?;
        Ok(Some(crate::project::instance::Context {
            directory: self.workspace_dir.to_string_lossy().to_string(),
            worktree,
            project,
        }))
    }

    pub(crate) fn clear(&mut self) {
        self.state = SkillListingSessionState::default();
    }

    pub(crate) async fn configure(&mut self, context: ToolUseContext) -> Result<ToolUseContext> {
        let defaults = self
            .definition
            .allowed_skills
            .iter()
            .filter_map(|name| crate::session::capability_policy::normalize_capability_id(name))
            .collect::<BTreeSet<_>>();
        let resolution = crate::session::capability_policy::resolve_skill_capabilities(
            self.skills.iter().map(|skill| skill.invocation_name()),
            defaults.iter().map(String::as_str),
            std::iter::empty::<&str>(),
            self.definition.skill_policy,
            std::iter::empty::<&str>(),
        );
        if let Some(missing) = defaults.difference(&resolution.default_injected_ids).next() {
            bail!("默认技能不存在、已禁用或不在当前权限范围内：{missing}");
        }
        self.skills.retain(|skill| {
            resolution.visible_ids.contains(&skill.invocation_name().trim().to_lowercase())
        });
        let user_references =
            crate::tools::skill::explicitly_referenced_skill_names(&self.skills, [&*self.query]);
        let instruction_references = crate::tools::skill::explicitly_referenced_skill_names(
            &self.skills,
            [&*self.instructions],
        );
        let mut explicit = user_references.clone();
        explicit.extend(instruction_references.iter().cloned());
        if crate::tools::skill::user_explicitly_requests_skill_use(&self.query) {
            explicit.extend(crate::tools::skill::user_invocable_skill_names(&self.skills));
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
            self.skills = crate::tools::skill::accessible_skills(
                std::mem::take(&mut self.skills),
                Some(&context),
            );
        }
        if let Some(session) = &self.session_id {
            // The raw authoritative history, rather than a summary or model tags,
            // decides which invocation records belong to this request.
            self.state = SkillListingSessionState::default();
            for record in
                crate::session::skill_checkpoint::restore(session, "main", &self.source_history)
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

    pub(crate) fn refresh_skill_spec(&self, specs: &mut [ToolSpec]) {
        for spec in specs.iter_mut().filter(|spec| spec.id == "skill") {
            spec.description = crate::tools::skill::description_for_skills(
                &self.workspace_dir,
                self.skills.clone(),
            );
        }
    }

    pub(crate) fn messages(&mut self, skill_tool_enabled: bool) -> Result<Vec<ChatMessage>> {
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

    pub(crate) async fn consume(
        &mut self,
        context: &ToolUseContext,
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
            crate::session::skill_checkpoint::save(
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
