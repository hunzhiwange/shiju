use super::core::Agent;
use crate::app::agent::agent::loop_::run_tool_call_loop;
use crate::app::agent::agent::research;
use crate::app::agent::memory::MemoryCategory;
use crate::app::agent::providers::ChatMessage;
use anyhow::Result;

impl Agent {
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
        if self.auto_save {
            let _ = self
                .memory
                .store("user_msg", user_message, MemoryCategory::Conversation, None)
                .await;
        }

        let context = self
            .memory_loader
            .load_context(self.memory.as_ref(), user_message)
            .await
            .unwrap_or_default();

        let research_context = if research::should_trigger(&self.research_config, user_message) {
            if self.research_config.show_progress {
                println!("[Research] Gathering information...");
            }

            match research::run_research_phase(
                &self.research_config,
                self.provider.as_ref(),
                &self.tools,
                user_message,
                &self.model_name,
                self.temperature,
                self.observer.clone(),
            )
            .await
            {
                Ok(result) => {
                    if self.research_config.show_progress {
                        println!(
                            "[Research] Complete: {} tool calls, {} chars context",
                            result.tool_call_count,
                            result.context.len()
                        );
                        for summary in &result.tool_summaries {
                            println!("  - {}: {}", summary.tool_name, summary.result_preview);
                        }
                    }
                    if result.context.is_empty() { None } else { Some(result.context) }
                }
                Err(e) => {
                    tracing::warn!("Research phase failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let now = crate::app::agent::util::locale::configured_datetime(self.timezone);
        let stamped_user_message = format!("[{now}] {user_message}");

        let enriched = match (&context, &research_context) {
            (c, Some(r)) if !c.is_empty() => format!("{c}\n\n{r}\n\n{stamped_user_message}"),
            (_, Some(r)) => format!("{r}\n\n{stamped_user_message}"),
            (c, None) if !c.is_empty() => format!("{c}{stamped_user_message}"),
            _ => stamped_user_message,
        };

        self.history.push(ChatMessage::user(enriched));

        let effective_model = self.classify_model(user_message).await?;
        let persistent_len = self.history.len();
        let mut request_history = self.ephemeral_request_history()?;
        let transient_len = request_history.len();
        request_history.extend(self.history.iter().cloned());

        let final_response = run_tool_call_loop(
            self.provider.as_ref(),
            &mut request_history,
            &self.tools,
            self.observer.as_ref(),
            "agent",
            &effective_model,
            self.temperature,
            false,
            self.approval.clone(),
            "cli",
            &self.multimodal_config,
            self.config.max_tool_iterations,
            None,
            None,
            None,
            self.security.clone(),
            &[],
        )
        .await?;

        self.history.extend(request_history.into_iter().skip(transient_len + persistent_len));

        self.trim_history();

        Ok(final_response)
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
        if self.auto_save {
            let _ = self
                .memory
                .store("user_msg", user_message, MemoryCategory::Conversation, None)
                .await;
        }

        let context = self
            .memory_loader
            .load_context(self.memory.as_ref(), user_message)
            .await
            .unwrap_or_default();

        let research_context = if research::should_trigger(&self.research_config, user_message) {
            if self.research_config.show_progress {
                println!("[Research] Gathering information...");
            }

            match research::run_research_phase(
                &self.research_config,
                self.provider.as_ref(),
                &self.tools,
                user_message,
                &self.model_name,
                self.temperature,
                self.observer.clone(),
            )
            .await
            {
                Ok(result) => {
                    if self.research_config.show_progress {
                        println!(
                            "[Research] Complete: {} tool calls, {} chars context",
                            result.tool_call_count,
                            result.context.len()
                        );
                        for summary in &result.tool_summaries {
                            println!("  - {}: {}", summary.tool_name, summary.result_preview);
                        }
                    }
                    if result.context.is_empty() { None } else { Some(result.context) }
                }
                Err(e) => {
                    tracing::warn!("Research phase failed: {}", e);
                    None
                }
            }
        } else {
            None
        };

        let now = crate::app::agent::util::locale::configured_datetime(self.timezone);
        let stamped_user_message = format!("[{now}] {user_message}");

        let enriched = match (&context, &research_context) {
            (c, Some(r)) if !c.is_empty() => format!("{c}\n\n{r}\n\n{stamped_user_message}"),
            (_, Some(r)) => format!("{r}\n\n{stamped_user_message}"),
            (c, None) if !c.is_empty() => format!("{c}{stamped_user_message}"),
            _ => stamped_user_message,
        };

        self.history.push(ChatMessage::user(enriched));

        let effective_model = self.classify_model(user_message).await?;
        let persistent_len = self.history.len();
        let mut request_history = self.ephemeral_request_history()?;
        let transient_len = request_history.len();
        request_history.extend(self.history.iter().cloned());

        let final_response = run_tool_call_loop(
            self.provider.as_ref(),
            &mut request_history,
            &self.tools,
            self.observer.as_ref(),
            "agent",
            &effective_model,
            self.temperature,
            false,
            self.approval.clone(),
            "cli",
            &self.multimodal_config,
            self.config.max_tool_iterations,
            None,
            Some(on_delta),
            None,
            self.security.clone(),
            &[],
        )
        .await?;

        self.history.extend(request_history.into_iter().skip(transient_len + persistent_len));

        self.trim_history();

        Ok(final_response)
    }

    fn ephemeral_request_history(&self) -> Result<Vec<ChatMessage>> {
        let protocol = self.build_system_prompt()?;
        Ok(vec![ChatMessage::user(format!(
            "<system-reminder>\n{protocol}\n</system-reminder>"
        ))])
    }
}
