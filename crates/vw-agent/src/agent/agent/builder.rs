use super::core::Agent;
use crate::app::agent::agent::memory_loader::{DefaultMemoryLoader, MemoryLoader};
use crate::app::agent::agent::prompt::SystemPromptBuilder;
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::memory::Memory;
use crate::app::agent::observability::Observer;
use crate::app::agent::providers::Provider;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::Tool;
use anyhow::Result;
use chrono_tz::Tz;
use std::collections::HashMap;
use std::sync::Arc;

/// 代理构建器
///
/// 使用建造者模式创建 [`Agent`] 实例。该构建器允许灵活配置代理的各个组件，
/// 并在构建时验证必需的字段是否已设置。
///
/// # 必需字段
///
/// 以下字段必须在调用 `build()` 之前设置：
/// - `provider` - AI 模型提供商
/// - `tools` - 工具列表
/// - `memory` - 记忆存储
/// - `observer` - 可观测性观察者
///
/// # 可选字段
///
/// 其他字段都有合理的默认值：
/// - `model_name` - 默认为 "anthropic/claude-sonnet-4-20250514"
/// - `temperature` - 默认为 0.7
/// - `workspace_dir` - 默认为当前目录 "."
/// - `prompt_builder` - 默认使用 `SystemPromptBuilder::with_defaults()`
/// - 其他配置字段默认使用 `Default` trait 的实现
///
/// # 示例
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use vibe_window::app::agent::agent::{Agent, AgentBuilder};
/// use vibe_window::app::agent::providers::create_provider;
/// use vibe_window::app::agent::tools::all_tools;
/// use vibe_window::app::agent::memory::create_memory;
///
/// let provider = create_provider("openai", Some("api-key"), None)?;
/// let tools = all_tools();
/// let memory = Arc::new(create_memory("sqlite")?);
/// let observer = Arc::new(vibe_window::app::agent::observability::create_observer(&Default::default()));
///
/// let agent = AgentBuilder::new()
///     .provider(provider)
///     .tools(tools)
///     .memory(memory)
///     .observer(observer)
///     .model_name("gpt-4".to_string())
///     .temperature(0.8)
///     .build()?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct AgentBuilder {
    provider: Option<Box<dyn Provider>>,
    tools: Option<Vec<Box<dyn Tool>>>,
    memory: Option<Arc<dyn Memory>>,
    observer: Option<Arc<dyn Observer>>,
    security: Option<Arc<SecurityPolicy>>,
    prompt_builder: Option<SystemPromptBuilder>,
    approval: Option<Arc<ApprovalManager>>,
    memory_loader: Option<Box<dyn MemoryLoader>>,
    config: Option<crate::app::agent::config::AgentConfig>,
    model_name: Option<String>,
    temperature: Option<f64>,
    workspace_dir: Option<std::path::PathBuf>,
    identity_config: Option<crate::app::agent::config::IdentityConfig>,
    skills: Option<Vec<crate::app::agent::skills::Skill>>,
    skills_prompt_mode: Option<crate::app::agent::config::SkillsPromptInjectionMode>,
    auto_save: Option<bool>,
    classification_config: Option<crate::app::agent::config::QueryClassificationConfig>,
    available_hints: Option<Vec<String>>,
    route_model_by_hint: Option<HashMap<String, String>>,
    multimodal_config: Option<crate::app::agent::config::MultimodalConfig>,
    timezone: Option<Tz>,
}

impl AgentBuilder {
    /// 创建新的代理构建器实例
    ///
    /// 所有字段初始化为 `None`，需要通过链式调用设置必需字段后才能构建。
    ///
    /// # 返回值
    ///
    /// 返回一个空的 `AgentBuilder` 实例
    pub fn new() -> Self {
        Self {
            provider: None,
            tools: None,
            memory: None,
            observer: None,
            security: None,
            prompt_builder: None,
            approval: None,
            memory_loader: None,
            config: None,
            model_name: None,
            temperature: None,
            workspace_dir: None,
            identity_config: None,
            skills: None,
            skills_prompt_mode: None,
            auto_save: None,
            classification_config: None,
            available_hints: None,
            route_model_by_hint: None,
            multimodal_config: None,
            timezone: None,
        }
    }

    pub fn provider(mut self, provider: Box<dyn Provider>) -> Self {
        self.provider = Some(provider);
        self
    }

    pub fn tools(mut self, tools: Vec<Box<dyn Tool>>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn memory(mut self, memory: Arc<dyn Memory>) -> Self {
        self.memory = Some(memory);
        self
    }

    pub fn observer(mut self, observer: Arc<dyn Observer>) -> Self {
        self.observer = Some(observer);
        self
    }

    pub fn security(mut self, security: Arc<SecurityPolicy>) -> Self {
        self.security = Some(security);
        self
    }

    pub fn prompt_builder(mut self, prompt_builder: SystemPromptBuilder) -> Self {
        self.prompt_builder = Some(prompt_builder);
        self
    }

    pub fn approval(mut self, approval: ApprovalManager) -> Self {
        self.approval = Some(Arc::new(approval));
        self
    }

    pub fn memory_loader(mut self, memory_loader: Box<dyn MemoryLoader>) -> Self {
        self.memory_loader = Some(memory_loader);
        self
    }

    pub fn config(mut self, config: crate::app::agent::config::AgentConfig) -> Self {
        self.config = Some(config);
        self
    }

    pub fn model_name(mut self, model_name: String) -> Self {
        self.model_name = Some(model_name);
        self
    }

    pub fn temperature(mut self, temperature: f64) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn workspace_dir(mut self, workspace_dir: std::path::PathBuf) -> Self {
        self.workspace_dir = Some(workspace_dir);
        self
    }

    pub fn identity_config(
        mut self,
        identity_config: crate::app::agent::config::IdentityConfig,
    ) -> Self {
        self.identity_config = Some(identity_config);
        self
    }

    pub fn skills(mut self, skills: Vec<crate::app::agent::skills::Skill>) -> Self {
        self.skills = Some(skills);
        self
    }

    pub fn skills_prompt_mode(
        mut self,
        skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode,
    ) -> Self {
        self.skills_prompt_mode = Some(skills_prompt_mode);
        self
    }

    pub fn auto_save(mut self, auto_save: bool) -> Self {
        self.auto_save = Some(auto_save);
        self
    }

    pub fn classification_config(
        mut self,
        classification_config: crate::app::agent::config::QueryClassificationConfig,
    ) -> Self {
        self.classification_config = Some(classification_config);
        self
    }

    pub fn available_hints(mut self, available_hints: Vec<String>) -> Self {
        self.available_hints = Some(available_hints);
        self
    }

    pub fn route_model_by_hint(mut self, route_model_by_hint: HashMap<String, String>) -> Self {
        self.route_model_by_hint = Some(route_model_by_hint);
        self
    }

    pub fn multimodal_config(
        mut self,
        multimodal_config: crate::app::agent::config::MultimodalConfig,
    ) -> Self {
        self.multimodal_config = Some(multimodal_config);
        self
    }

    pub fn timezone(mut self, timezone: Tz) -> Self {
        self.timezone = Some(timezone);
        self
    }

    /// 构建代理实例
    ///
    /// 验证必需字段并创建 `Agent` 实例。如果必需字段未设置，将返回错误。
    ///
    /// # 错误
    ///
    /// 如果以下必需字段未设置，将返回错误：
    /// - `tools` - 工具列表
    /// - `provider` - 模型提供商
    /// - `memory` - 记忆存储
    /// - `observer` - 观察者
    ///
    /// # 返回值
    ///
    /// 成功时返回构建好的 `Agent` 实例，失败时返回错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use std::sync::Arc;
    /// # use vibe_window::app::agent::agent::AgentBuilder;
    /// # let provider = todo!();
    /// # let tools = todo!();
    /// # let memory = todo!();
    /// # let observer = todo!();
    /// let agent = AgentBuilder::new()
    ///     .provider(provider)
    ///     .tools(tools)
    ///     .memory(memory)
    ///     .observer(observer)
    ///     .model_name("gpt-4".to_string())
    ///     .build()?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn build(self) -> Result<Agent> {
        let tools = self.tools.ok_or_else(|| anyhow::anyhow!("tools are required"))?;
        let tool_specs = tools.iter().map(|tool| tool.spec()).collect();

        Ok(Agent {
            provider: self.provider.ok_or_else(|| anyhow::anyhow!("provider is required"))?,
            tools,
            tool_specs,
            memory: self.memory.ok_or_else(|| anyhow::anyhow!("memory is required"))?,
            observer: self.observer.ok_or_else(|| anyhow::anyhow!("observer is required"))?,
            security: self.security,
            prompt_builder: self.prompt_builder.unwrap_or_else(SystemPromptBuilder::with_defaults),
            approval: self.approval,
            memory_loader: self
                .memory_loader
                .unwrap_or_else(|| Box::new(DefaultMemoryLoader::default())),
            config: self.config.unwrap_or_default(),
            model_name: self
                .model_name
                .unwrap_or_else(|| "anthropic/claude-sonnet-4-20250514".into()),
            temperature: self.temperature.unwrap_or(0.7),
            workspace_dir: self.workspace_dir.unwrap_or_else(|| std::path::PathBuf::from(".")),
            identity_config: self.identity_config.unwrap_or_default(),
            skills: self.skills.unwrap_or_default(),
            skills_prompt_mode: self.skills_prompt_mode.unwrap_or_default(),
            auto_save: self.auto_save.unwrap_or(false),
            history: Vec::new(),
            classification_config: self.classification_config.unwrap_or_default(),
            available_hints: self.available_hints.unwrap_or_default(),
            route_model_by_hint: self.route_model_by_hint.unwrap_or_default(),
            multimodal_config: self.multimodal_config.unwrap_or_default(),
            timezone: self.timezone.unwrap_or(chrono_tz::Asia::Shanghai),
        })
    }
}
