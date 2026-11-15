use super::core::Agent;
use agent_memory::MemoryCategory;
use model_types::ChatMessage;
use anyhow::Result;

impl<H: super::AgentHost> Agent<H> {
    /// 执行单轮对话
    ///
    /// 处理用户消息并生成响应。这是代理的主要交互方法，执行完整的对话流程：
    /// 1. 初始化系统提示词（如果需要）
    /// 2. 保存用户消息到记忆（如果启用自动保存）
    /// 3. 加载相关记忆上下文
    /// 4. 执行研究阶段（如果触发条件满足）
    /// 5. 构建增强的用户消息（包含上下文和时间戳）
    /// 6. 分类查询并确定使用的模型
    /// 7. 运行工具调用循环
    /// 8. 裁剪历史记录
    ///
    /// # 参数
    ///
    /// * `user_message` - 用户输入的消息内容
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
    /// let response = agent.turn("请帮我写一个排序函数").await?;
    /// println!("{}", response);
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub async fn turn(&mut self, user_message: &str) -> Result<String> {
        let context = Box::pin(self.skill_session.project_context()).await?;
        let execution = Box::pin(self.turn_in_project(user_message));
        match context {
            Some(context) => agent_project::instance::provide_context(context, execution).await,
            None => execution.await,
        }
    }

    async fn turn_in_project(&mut self, user_message: &str) -> Result<String> {
        crate::turn::turn(self, user_message, None).await
    }

    /// 执行单轮对话（带流式输出）
    ///
    /// 与 [`turn`](Agent::turn) 方法类似，但支持流式输出响应内容。
    /// 响应的增量内容会通过提供的通道发送，适合需要实时显示响应的场景。
    ///
    /// # 参数
    ///
    /// * `user_message` - 用户输入的消息内容
    /// * `on_delta` - 用于发送增量响应内容的 Tokio MPSC 发送端
    ///
    /// # 返回值
    ///
    /// 成功时返回代理的完整响应字符串，失败时返回错误
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// # use vibe_window::app::agent::agent::Agent;
    /// # use vibe_window::app::agent::config::Config;
    /// # let config = Config::load("config.toml")?;
    /// let mut agent = Agent::from_config(&config)?;
    ///
    /// let (tx, mut rx) = tokio::sync::mpsc::channel(100);
    ///
    /// tokio::spawn(async move {
    ///     while let Some(delta) = rx.recv().await {
    ///         print!("{}", delta);
    ///     }
    /// });
    ///
    /// let response = agent.turn_with_stream("请帮我写一个排序函数", tx).await?;
    /// # Ok::<(), anyhow::Error>(())
    /// ```
    pub async fn turn_with_stream(
        &mut self,
        user_message: &str,
        on_delta: tokio::sync::mpsc::Sender<String>,
    ) -> Result<String> {
        let context = Box::pin(self.skill_session.project_context()).await?;
        let execution = Box::pin(self.turn_with_stream_in_project(user_message, on_delta));
        match context {
            Some(context) => agent_project::instance::provide_context(context, execution).await,
            None => execution.await,
        }
    }

    async fn turn_with_stream_in_project(
        &mut self,
        user_message: &str,
        on_delta: tokio::sync::mpsc::Sender<String>,
    ) -> Result<String> {
        crate::turn::turn(self, user_message, Some(on_delta)).await
    }


}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl<H: super::AgentHost> crate::turn::TurnAgent for Agent<H> {
    fn auto_save(&self) -> bool { self.auto_save }
    async fn store_user_message(&self, message: &str) -> Result<()> {
        self.memory.store("user_msg", message, MemoryCategory::Conversation, None).await
    }
    async fn load_memory_context(&self, message: &str) -> Result<String> {
        self.memory_loader.load_context(self.memory.as_ref(), message).await
    }
    fn configured_datetime(&self) -> String {
        agent_util::locale::configured_datetime(self.timezone)
    }
    fn turn_history(&mut self) -> &mut Vec<ChatMessage> { &mut self.history }
    async fn select_model(&self, message: &str) -> Result<String> {
        self.classify_model(message).await
    }
    fn system_prompt(&self) -> Result<String> { self.build_system_prompt() }
    fn begin_skill_turn(&mut self, message: &str, instructions: &str) {
        self.skill_session.begin_turn(message, instructions, &self.history);
    }
    async fn set_skill_model(&mut self, model: &str) -> Result<()> {
        Box::pin(self.skill_session.set_model_context(model)).await
    }
    async fn execute_loop(
        &mut self,
        history: &mut Vec<ChatMessage>,
        model: &str,
        on_delta: Option<tokio::sync::mpsc::Sender<String>>,
    ) -> Result<String> {
        H::run_tool_call_loop_with_skills(
            self.provider.as_ref(), history, &self.tools, self.observer.as_ref(),
            "agent", model, self.temperature, false, self.approval.clone(), "cli",
            &self.multimodal_config, self.config.max_iterations, None, on_delta,
            None, self.security.clone(), &[], Some(&mut self.skill_session),
        ).await
    }
    fn trim_turn_history(&mut self) { self.trim_history(); }
}
