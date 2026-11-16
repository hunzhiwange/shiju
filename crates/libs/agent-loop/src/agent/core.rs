use super::builder::AgentBuilder;
use crate::memory_loader::MemoryLoader;
use crate::prompt::{PromptContext, SystemPromptBuilder};
use agent_approval::ApprovalManager;
use agent_config::schema::Config;
use agent_memory::Memory;
use agent_observability::Observer;
use model_types::{ChatMessage, Provider};
use agent_security::SecurityPolicy;
use tool_types::{Tool, ToolSpec};
use model_selection::{
    AutoModelSelectionRequest, ModelCandidate, ModelReference, select_auto_model,
};
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
pub struct Agent<H: super::AgentHost> {
    pub(crate) provider: Box<dyn Provider>,
    pub(crate) tools: Vec<Box<dyn Tool>>,
    pub(crate) tool_specs: Vec<ToolSpec>,
    pub(crate) memory: Arc<dyn Memory>,
    pub(crate) observer: Arc<dyn Observer>,
    pub(crate) security: Option<Arc<SecurityPolicy>>,
    pub(crate) prompt_builder: SystemPromptBuilder,
    pub(crate) approval: Option<Arc<ApprovalManager>>,
    pub(crate) memory_loader: Box<dyn MemoryLoader>,
    pub(crate) config: config_agent::AgentDefinitionConfig,
    pub(crate) model_name: String,
    pub(crate) temperature: f64,
    pub(crate) workspace_dir: std::path::PathBuf,
    pub(crate) identity_config: agent_config::schema::IdentityConfig,
    pub(crate) skills: Vec<agent_skills::types::Skill>,
    pub(crate) skills_prompt_mode: agent_config::schema::SkillsPromptInjectionMode,
    pub(crate) auto_save: bool,
    pub(crate) history: Vec<ChatMessage>,
    pub(crate) classification_config: config_routing::QueryClassificationConfig,
    pub(crate) available_hints: Vec<String>,
    pub(crate) route_model_by_hint: HashMap<String, String>,
    pub(crate) multimodal_config: config_tools::MultimodalConfig,
    pub(crate) timezone: Tz,
    pub(crate) skill_session: crate::core::skills::CoreSkillSession<H>,
}

impl<H: super::AgentHost> Agent<H> {
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
    pub fn builder() -> AgentBuilder<H> {
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
        self.skill_session.clear();
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
        H::from_config(config)
    }

    pub(crate) fn trim_history(&mut self) {
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

    pub(crate) fn build_system_prompt(&self) -> Result<String> {
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

    pub(crate) async fn classify_model(&self, user_message: &str) -> Result<String> {
        if !is_auto_model_request(&self.model_name) {
            return Ok(self.model_name.clone());
        }

        let models = H::models().await?;
        let candidates = models
            .iter()
            .map(|model| ModelCandidate { provider_id: &model.provider_id, model })
            .collect::<Vec<_>>();

        self.classify_model_with_candidates(user_message, &candidates)
            .map(|selected| format!("{}/{}", selected.provider, selected.model))
            .map_err(anyhow::Error::msg)
    }

    pub(crate) fn classify_model_with_candidates(
        &self,
        user_message: &str,
        candidates: &[ModelCandidate<'_>],
    ) -> std::result::Result<ModelReference, String> {
        let model_routes = self
            .route_model_by_hint
            .iter()
            .filter(|(hint, _)| self.available_hints.contains(*hint))
            .filter_map(|(hint, model)| {
                let (provider, model) = model.split_once('/')?;
                Some(config_routing::ModelRouteConfig {
                    hint: hint.clone(),
                    provider: provider.to_string(),
                    model: model.to_string(),
                    max_tokens: None,
                    api_key: None,
                })
            })
            .collect();
        select_auto_model(
            &AutoModelSelectionRequest {
                message: user_message,
                required_context: 0,
                needs_tools: true,
                needs_image_input: false,
                needs_attachments: false,
                soft_preference: None,
                query_classification: self.classification_config.clone(),
                model_routes,
            },
            candidates,
        )
        .map(|decision| decision.model)
        .map_err(|error| error.to_string())
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

fn is_auto_model_request(model: &str) -> bool {
    let model = model.trim();
    model.is_empty() || model.eq_ignore_ascii_case("auto")
}

impl<H: super::AgentHost> Agent<H> {
    /// 宿主工厂绑定经过验证的完整配置，技能会话不向外暴露可写字段。
    pub fn bind_skill_runtime_config(&mut self, config: std::sync::Arc<agent_config::runtime::Config>) {
        self.skill_session.config = Some(config);
    }
    #[cfg(any(test, feature = "test-support"))]
    pub fn skill_context_window_tokens(&self) -> usize { self.skill_session.context_window_tokens }
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_auto_save(&mut self, enabled: bool) { self.auto_save = enabled; }
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_timezone(&mut self, timezone: chrono_tz::Tz) { self.timezone = timezone; }
    #[cfg(any(test, feature = "test-support"))]
    pub fn fixture_routing(&mut self, classification: config_routing::QueryClassificationConfig, hints: Vec<String>, routes: std::collections::HashMap<String,String>) {
        self.classification_config = classification;
        self.available_hints = hints;
        self.route_model_by_hint = routes;
    }
}
