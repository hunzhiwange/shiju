use super::builder::AgentBuilder;
use crate::app::agent::agent::memory_loader::{DefaultMemoryLoader, MemoryLoader};
use crate::app::agent::agent::prompt::{PromptContext, SystemPromptBuilder};
use crate::app::agent::approval::ApprovalManager;
use crate::app::agent::config::{Config, ResearchPhaseConfig};
use crate::app::agent::memory::{self, Memory};
use crate::app::agent::observability::{self, Observer};
use crate::app::agent::providers::{ChatMessage, Provider};
use crate::app::agent::runtime;
use crate::app::agent::security::SecurityPolicy;
use crate::app::agent::tools::{Tool, ToolSpec};
use anyhow::Result;
use chrono_tz::Tz;
use std::collections::HashMap;
use std::sync::Arc;

/// 核心代理结构体
///
/// Agent 是 VibeWindow 的主要交互接口，负责：
/// - 管理 AI 模型提供商的调用
/// - 执行工具调用循环
/// - 维护对话历史
/// - 处理记忆与上下文加载
/// - 执行查询分类和模型路由
///
/// # 字段说明
///
/// * `provider` - AI 模型提供商，用于生成响应
/// * `tools` - 可用工具列表
/// * `tool_specs` - 工具规格列表，用于向模型声明可用工具
/// * `memory` - 记忆存储后端
/// * `observer` - 可观测性观察者，用于记录事件
/// * `prompt_builder` - 系统提示词构建器
/// * `approval` - 可选的审批管理器，用于敏感操作确认
/// * `memory_loader` - 记忆加载器，负责从记忆中加载相关上下文
/// * `config` - 代理配置
/// * `model_name` - 当前使用的模型名称
/// * `temperature` - 模型温度参数，控制响应的随机性
/// * `workspace_dir` - 工作目录路径
/// * `identity_config` - 身份配置，定义代理的身份特征
/// * `skills` - 已加载的技能列表
/// * `skills_prompt_mode` - 技能提示注入模式
/// * `auto_save` - 是否自动保存对话到记忆
/// * `history` - 对话历史记录
/// * `classification_config` - 查询分类配置
/// * `available_hints` - 可用的路由提示列表
/// * `route_model_by_hint` - 提示到模型的映射表
/// * `research_config` - 研究阶段配置
/// * `multimodal_config` - 多模态配置
///
/// # 示例
///
/// ```rust,no_run
/// use std::sync::Arc;
/// use vibe_window::app::agent::agent::Agent;
/// use vibe_window::app::agent::config::Config;
///
/// let config = Config::load("config.toml")?;
/// let mut agent = Agent::from_config(&config)?;
///
/// let response = agent.turn("请帮我写一个排序函数").await?;
/// # Ok::<(), anyhow::Error>(())
/// ```
pub struct Agent {
    pub(super) provider: Box<dyn Provider>,
    pub(super) tools: Vec<Box<dyn Tool>>,
    pub(super) tool_specs: Vec<ToolSpec>,
    pub(super) memory: Arc<dyn Memory>,
    pub(super) observer: Arc<dyn Observer>,
    pub(super) security: Option<Arc<SecurityPolicy>>,
    pub(super) prompt_builder: SystemPromptBuilder,
    pub(super) approval: Option<Arc<ApprovalManager>>,
    pub(super) memory_loader: Box<dyn MemoryLoader>,
    pub(super) config: crate::app::agent::config::AgentConfig,
    pub(super) model_name: String,
    pub(super) temperature: f64,
    pub(super) workspace_dir: std::path::PathBuf,
    pub(super) identity_config: crate::app::agent::config::IdentityConfig,
    pub(super) skills: Vec<crate::app::agent::skills::Skill>,
    pub(super) skills_prompt_mode: crate::app::agent::config::SkillsPromptInjectionMode,
    pub(super) auto_save: bool,
    pub(super) history: Vec<ChatMessage>,
    pub(super) classification_config: crate::app::agent::config::QueryClassificationConfig,
    pub(super) available_hints: Vec<String>,
    pub(super) route_model_by_hint: HashMap<String, String>,
    pub(super) research_config: ResearchPhaseConfig,
    pub(super) multimodal_config: crate::app::agent::config::MultimodalConfig,
    pub(super) timezone: Tz,
}

impl Agent {
    /// 创建代理构建器
    ///
    /// # 返回值
    ///
    /// 返回新的 `AgentBuilder` 实例
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use vibe_window::app::agent::agent::Agent;
    /// let builder = Agent::builder();
    /// ```
    pub fn builder() -> AgentBuilder {
        AgentBuilder::new()
    }

    /// 获取对话历史的不可变引用
    ///
    /// # 返回值
    ///
    /// 返回对话历史消息切片的引用
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use vibe_window::app::agent::agent::Agent;
    /// # let agent = todo!();
    /// for msg in agent.history() {
    ///     println!("{}: {:?}", msg.role, msg.content);
    /// }
    /// ```
    pub fn history(&self) -> &[ChatMessage] {
        &self.history
    }

    /// 清空对话历史
    ///
    /// 清除所有历史消息，重置为新的会话状态。
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use vibe_window::app::agent::agent::Agent;
    /// # let mut agent = todo!();
    /// agent.clear_history();
    /// ```
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// 从配置创建代理实例
    ///
    /// 这是创建代理实例的推荐方式，会自动初始化所有必需的组件。
    ///
    /// # 参数
    ///
    /// * `config` - 配置对象的引用
    ///
    /// # 返回值
    ///
    /// 成功时返回配置好的 `Agent` 实例，失败时返回错误
    ///
    /// # 初始化流程
    ///
    /// 1. 创建可观测性观察者
    /// 2. 创建运行时适配器
    /// 3. 创建安全策略
    /// 4. 创建记忆存储
    /// 5. 加载所有工具
    /// 6. 创建模型提供商
    /// 7. 创建审批管理器
    /// 8. 构建并返回代理实例
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use vibe_window::app::agent::config::Config;
    /// use vibe_window::app::agent::agent::Agent;
    ///
    /// let config = Config::load("config.toml")?;
    /// let mut agent = Agent::from_config(&config)?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub fn from_config(config: &Config) -> Result<Self> {
        let timezone = crate::app::agent::util::locale::parse_timezone(
            &config.app_ui.system_settings.timezone,
        )?;
        let observer: Arc<dyn Observer> =
            Arc::from(observability::create_observer(&config.observability));
        let runtime: Arc<dyn runtime::RuntimeAdapter> =
            Arc::from(runtime::create_runtime(&config.runtime)?);
        let security =
            Arc::new(SecurityPolicy::from_configs(
                &config.autonomy,
                &config.security,
                &config.workspace_dir,
            ));

        let memory: Arc<dyn Memory> = Arc::from(memory::create_memory_with_storage_and_routes(
            &config.memory,
            &config.embedding_routes,
            Some(&config.storage.provider.config),
            &config.workspace_dir,
            config.api_key.as_deref(),
        )?);

        let composio_key =
            if config.composio.enabled { config.composio.api_key.as_deref() } else { None };
        let composio_entity_id =
            if config.composio.enabled { Some(config.composio.entity_id.as_str()) } else { None };

        let tools = crate::app::agent::tools::all_tools_with_runtime(
            Arc::new(config.clone()),
            &security,
            runtime,
            memory.clone(),
            composio_key,
            composio_entity_id,
            &config.browser,
            &config.http_request,
            &config.web_fetch,
            &config.workspace_dir,
            &config.agents,
            config.api_key.as_deref(),
            config,
            None,
        );

        let provider_name = config.default_provider.as_deref().unwrap_or("zhipuai-coding-plan");
        let model_name =
            config.default_model.as_deref().unwrap_or("zhipuai-coding-plan/glm-5").to_string();

        let provider: Box<dyn Provider> = crate::app::agent::providers::create_routed_provider(
            provider_name,
            config.api_key.as_deref(),
            config.api_url.as_deref(),
            &config.reliability,
            &config.model_routes,
            &model_name,
        )?;

        let approval = ApprovalManager::from_config(&config.autonomy);

        let route_model_by_hint: HashMap<String, String> = config
            .model_routes
            .iter()
            .map(|route| (route.hint.clone(), route.model.clone()))
            .collect();
        let available_hints: Vec<String> = route_model_by_hint.keys().cloned().collect();

        Agent::builder()
            .provider(provider)
            .tools(tools)
            .memory(memory)
            .observer(observer)
            .security(security.clone())
            .approval(approval)
            .memory_loader(Box::new(DefaultMemoryLoader::new(5, config.memory.min_relevance_score)))
            .prompt_builder(SystemPromptBuilder::with_defaults())
            .config(config.agent.clone())
            .model_name(model_name)
            .temperature(config.default_temperature)
            .workspace_dir(config.workspace_dir.clone())
            .classification_config(config.query_classification.clone())
            .available_hints(available_hints)
            .route_model_by_hint(route_model_by_hint)
            .identity_config(config.identity.clone())
            .skills(crate::app::agent::skills::load_skills_with_config(
                &config.workspace_dir,
                config,
            ))
            .skills_prompt_mode(config.skills.prompt_injection_mode)
            .auto_save(config.memory.auto_save)
            .research_config(config.research.clone())
            .multimodal_config(config.multimodal.clone())
            .timezone(timezone)
            .build()
    }

    pub(super) fn trim_history(&mut self) {
        let max = self.config.max_history_messages;
        if self.history.len() <= max {
            return;
        }

        let mut system_messages = Vec::new();
        let mut other_messages = Vec::new();

        for msg in self.history.drain(..) {
            if msg.role == "system" {
                system_messages.push(msg);
            } else {
                other_messages.push(msg);
            }
        }

        if other_messages.len() > max {
            let drop_count = other_messages.len() - max;
            other_messages.drain(0..drop_count);
        }

        self.history = system_messages;
        self.history.extend(other_messages);
    }

    pub(super) fn build_system_prompt(&self) -> Result<String> {
        let instructions = if self.provider.supports_native_tools() {
            String::new()
        } else {
            let mut s = String::new();
            s.push_str("## Tool Use Protocol\n\n");
            s.push_str("To use a tool, wrap a JSON object in <tool> tags:\n\n");
            s.push_str("```\n<tool>\n{\"name\": \"tool_name\", \"arguments\": {\"param\": \"value\"}}\n</tool>\n```\n\n");
            s
        };

        let ctx = PromptContext {
            workspace_dir: &self.workspace_dir,
            model_name: &self.model_name,
            tools: &self.tools,
            skills: &self.skills,
            skills_prompt_mode: self.skills_prompt_mode,
            identity_config: Some(&self.identity_config),
            dispatcher_instructions: &instructions,
            timezone: self.timezone,
        };

        self.prompt_builder.build(&ctx)
    }

    pub(super) fn classify_model(&self, user_message: &str) -> String {
        if let Some(decision) = super::super::classifier::classify_with_decision(
            &self.classification_config,
            user_message,
        ) {
            if self.available_hints.contains(&decision.hint) {
                let resolved_model = self
                    .route_model_by_hint
                    .get(&decision.hint)
                    .map(String::as_str)
                    .unwrap_or("unknown");
                tracing::info!(
                    target: "query_classification",
                    hint = decision.hint.as_str(),
                    model = resolved_model,
                    rule_priority = decision.priority,
                    message_length = user_message.len(),
                    "Classified message route"
                );
                return format!("hint:{}", decision.hint);
            }
        }

        self.model_name.clone()
    }

    /// 运行单次对话（便捷方法）
    ///
    /// 这是 [`turn`](Agent::turn) 方法的简单包装，提供更直观的方法名。
    ///
    /// # 参数
    ///
    /// * `message` - 用户输入的消息内容
    ///
    /// # 返回值
    ///
    /// 成功时返回代理的响应字符串，失败时返回错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use vibe_window::app::agent::agent::Agent;
    /// # use vibe_window::app::agent::config::Config;
    /// # let config = Config::load("config.toml")?;
    /// let mut agent = Agent::from_config(&config)?;
    ///
    /// let response = agent.run_single("你好").await?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub async fn run_single(&mut self, message: &str) -> Result<String> {
        self.turn(message).await
    }
}
