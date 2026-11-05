//! Agent 模块单元测试
//!
//! 本模块包含 `Agent` 核心结构的单元测试，验证代理的主要行为：
//!
//! - 无工具调用时的纯文本响应处理
//! - 带工具调用的请求-响应循环
//! - 基于查询分类提示（hint）的模型路由
//!
//! # 测试策略
//!
//! 本测试模块采用依赖注入与 Mock 对象模式，确保测试的确定性和隔离性：
//!
//! - **MockProvider**: 模拟 LLM 提供者，支持预设响应队列，按 FIFO 顺序消费
//! - **ModelCaptureProvider**: 扩展的 Mock，额外记录模型选择用于验证路由逻辑
//! - **MockTool**: 模拟工具执行，始终返回成功结果
//! - **NoopObserver**: 空操作观察者，避免测试中的副作用
//! - **none 内存后端**: 禁用持久化，避免文件系统依赖
//!
//! # 线程安全
//!
//! 所有 Mock 对象使用 `parking_lot::Mutex` 保护内部状态，支持多线程测试场景。
//! 注意：实际测试均为单线程，但 Mock 实现保持线程安全以确保与生产代码的一致性。
//!
//! # WASM 兼容性
//!
//! 测试代码使用条件编译 `#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]`
//! 确保 Mock 实现在 WASM 目标架构下也能编译通过。这允许在需要时进行
//! WASM 环境下的集成测试。
//!
//! # 测试覆盖范围
//!
//! | 功能点 | 测试用例 |
//! |--------|----------|
//! | 纯文本响应 | `turn_without_tools_returns_text` |
//! | 工具调用循环 | `turn_with_native_dispatcher_handles_tool_results_variant` |
//! | 查询分类路由 | `turn_routes_with_hint_when_query_classification_matches` |

#[allow(dead_code)]
mod tests {
    use async_trait::async_trait;
    use parking_lot::Mutex;
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::super::Agent;
    use crate::app::agent::memory::Memory;
    use crate::app::agent::observability::Observer;
    use crate::app::agent::providers::ChatRequest;
    use crate::app::agent::providers::Provider;
    use crate::app::agent::tools::Tool;
    use anyhow::Result;

    /// Mock Provider 实现
    ///
    /// 用于测试的模拟提供者，支持预设响应队列。每次调用 `chat` 时
    /// 从队列前端取出一个响应返回；队列为空时返回默认响应。
    ///
    /// # 设计说明
    ///
    /// 响应队列设计允许模拟多轮对话场景：
    /// - 第一轮：Provider 返回包含工具调用的响应
    /// - 后续轮次：Provider 处理工具结果并返回最终文本
    ///
    /// 这种设计使得测试可以验证 Agent 的完整工具调用循环，
    /// 而不依赖真实的 LLM API。
    ///
    /// # 线程安全
    ///
    /// 内部使用 `parking_lot::Mutex` 保护响应队列，支持异步上下文中的安全访问。
    /// 锁的持有时间尽可能短（仅在取出响应时）。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 创建返回两轮响应的 MockProvider
    /// let provider = MockProvider {
    ///     responses: Mutex::new(vec![
    ///         // 第一轮：触发工具调用
    ///         ChatResponse {
    ///             text: None,
    ///             tool_calls: vec![ToolCall { kind: Default::default(), namespace: None, name: "echo", ... }],
    ///             ...
    ///         },
    ///         // 第二轮：返回最终结果
    ///         ChatResponse {
    ///             text: Some("final result".into()),
    ///             tool_calls: vec![],
    ///             ...
    ///         },
    ///     ]),
    /// };
    /// ```
    struct MockProvider {
        /// 预设的响应队列，按 FIFO 顺序消费
        ///
        /// 每次调用 `chat` 方法时，从此队列前端移除并返回一个响应。
        /// 队列为空时返回默认响应（text="done", tool_calls=[]）。
        responses: Mutex<Vec<crate::app::agent::providers::ChatResponse>>,
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Provider for MockProvider {
        /// 简化的聊天接口，返回固定字符串 "ok"
        ///
        /// 此方法主要用于测试基本的消息传递路径，不涉及响应队列。
        /// 在大多数测试中，应优先使用 `chat` 方法以测试完整行为。
        ///
        /// # 参数
        ///
        /// - `_system_prompt`: 系统提示（被忽略）
        /// - `_message`: 用户消息（被忽略）
        /// - `_model`: 模型标识符（被忽略）
        /// - `_temperature`: 温度参数（被忽略）
        ///
        /// # 返回
        ///
        /// 始终返回 `Ok("ok".into())`
        ///
        /// # 注意
        ///
        /// 所有参数均被忽略，因为此实现仅用于测试基本流程。
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> Result<String> {
            Ok("ok".into())
        }

        /// 带响应队列的聊天接口
        ///
        /// 从预设队列中取出下一个响应。若队列为空，返回默认响应：
        /// - `text`: "done"
        /// - `tool_calls`: 空向量
        /// - `usage`: None
        /// - `reasoning_content`: None
        ///
        /// # 参数
        ///
        /// - `_request`: 聊天请求对象（被忽略，因为响应已预设）
        /// - `_model`: 模型标识符（被忽略）
        /// - `_temperature`: 温度参数（被忽略）
        ///
        /// # 返回
        ///
        /// 队列中的下一个 `ChatResponse`，或默认响应（队列为空时）
        ///
        /// # 锁定行为
        ///
        /// 此方法会短暂持有 `responses` 互斥锁，仅用于取出一个响应。
        /// 锁在 `guard` 离开作用域时自动释放。
        ///
        /// # 示例
        ///
        /// ```ignore
        /// // 队列中有响应时
        /// let response = provider.chat(request, "gpt-4", 0.7).await?;
        /// assert_eq!(response.text, Some("hello".into()));
        ///
        /// // 队列为空时
        /// let response = provider.chat(request, "gpt-4", 0.7).await?;
        /// assert_eq!(response.text, Some("done".into()));
        /// ```
        async fn chat(
            &self,
            _request: ChatRequest<'_>,
            _model: &str,
            _temperature: f64,
        ) -> Result<crate::app::agent::providers::ChatResponse> {
            // 获取响应队列的互斥锁
            let mut guard = self.responses.lock();

            // 检查队列是否为空
            if guard.is_empty() {
                // 返回默认响应，表示对话结束
                return Ok(crate::app::agent::providers::ChatResponse {
                    text: Some("done".into()),
                    tool_calls: vec![],
                    usage: None,
                    reasoning_content: None,
                });
            }

            // 移除并返回队列中的第一个响应（FIFO 顺序）
            Ok(guard.remove(0))
        }
    }

    /// 带模型捕获功能的 Mock Provider
    ///
    /// 与 `MockProvider` 类似，但额外记录每次 `chat` 调用使用的模型名称。
    /// 主要用于验证查询分类和模型路由逻辑。
    ///
    /// # 设计动机
    ///
    /// 在 Agent 的查询分类功能中，系统会根据用户输入的关键词选择不同的模型。
    /// 为了验证这个路由逻辑是否正确工作，我们需要检查 Provider 实际接收到的
    /// 模型名称。`ModelCaptureProvider` 通过 `seen_models` 字段记录所有调用
    /// 中的模型参数，使测试能够断言路由行为。
    ///
    /// # 用例
    ///
    /// 验证特定提示（hint）是否正确路由到预期模型：
    /// ```ignore
    /// // 创建共享的模型记录器
    /// let seen_models = Arc::new(Mutex::new(Vec::new()));
    ///
    /// // 创建捕获 Provider
    /// let provider = ModelCaptureProvider {
    ///     responses: Mutex::new(vec![...]),
    ///     seen_models: seen_models.clone(),
    /// };
    ///
    /// // 执行 Agent 操作...
    /// agent.turn("quick summary").await?;
    ///
    /// // 验证模型路由
    /// let models = seen_models.lock();
    /// assert!(models.contains(&"hint:fast".to_string()));
    /// ```
    ///
    /// # 线程安全
    ///
    /// 与 `MockProvider` 相同，使用 `parking_lot::Mutex` 保护内部状态。
    /// `seen_models` 使用 `Arc<Mutex<...>>` 以便测试代码能够访问捕获的数据。
    struct ModelCaptureProvider {
        /// 预设的响应队列，按 FIFO 顺序消费
        ///
        /// 行为与 `MockProvider::responses` 完全相同
        responses: Mutex<Vec<crate::app::agent::providers::ChatResponse>>,

        /// 记录所有调用中传入的模型名称
        ///
        /// 每次 `chat` 调用时，`model` 参数会被追加到此向量中。
        /// 测试代码通过克隆的 `Arc` 引用访问此数据以验证路由行为。
        ///
        /// # 访问模式
        ///
        /// - Provider 实现：只写（push）
        /// - 测试代码：只读（通过 `lock()` 获取快照）
        seen_models: Arc<Mutex<Vec<String>>>,
    }

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Provider for ModelCaptureProvider {
        /// 简化的聊天接口，返回固定字符串 "ok"
        ///
        /// 注意：此方法不记录模型名称，因为它不用于路由测试。
        /// 路由验证应使用 `chat` 方法。
        async fn chat_with_system(
            &self,
            _system_prompt: Option<&str>,
            _message: &str,
            _model: &str,
            _temperature: f64,
        ) -> Result<String> {
            Ok("ok".into())
        }

        /// 带模型捕获的聊天接口
        ///
        /// 在返回响应前，先将模型名称记录到 `seen_models` 队列中。
        /// 其余行为与 `MockProvider::chat` 相同。
        ///
        /// # 参数
        ///
        /// - `_request`: 聊天请求对象（被忽略，因为响应已预设）
        /// - `model`: 模型标识符（**会被记录到 seen_models**）
        /// - `_temperature`: 温度参数（被忽略）
        ///
        /// # 返回
        ///
        /// 队列中的下一个 `ChatResponse`，或默认响应（队列为空时）
        ///
        /// # 捕获行为
        ///
        /// 模型名称在处理响应队列之前被捕获，确保即使队列为空
        /// 也能记录模型选择。
        ///
        /// # 示例
        ///
        /// ```ignore
        /// // 多次调用后检查捕获的模型
        /// agent.turn("query 1").await?;
        /// agent.turn("query 2").await?;
        ///
        /// let models = seen_models.lock();
        /// assert_eq!(models.len(), 2);
        /// assert_eq!(models[0], "hint:fast");
        /// ```
        async fn chat(
            &self,
            _request: ChatRequest<'_>,
            model: &str,
            _temperature: f64,
        ) -> Result<crate::app::agent::providers::ChatResponse> {
            // 在处理响应前先捕获模型名称，确保即使后续操作失败也能记录
            self.seen_models.lock().push(model.to_string());

            // 获取响应队列
            let mut guard = self.responses.lock();

            // 队列为空时返回默认响应
            if guard.is_empty() {
                return Ok(crate::app::agent::providers::ChatResponse {
                    text: Some("done".into()),
                    tool_calls: vec![],
                    usage: None,
                    reasoning_content: None,
                });
            }

            // 返回队列中的第一个响应
            Ok(guard.remove(0))
        }
    }

    /// Mock Tool 实现
    ///
    /// 一个简单的测试工具，名为 "echo"，执行时始终返回成功结果。
    /// 用于验证代理的工具调用流程，包括：
    ///
    /// - 工具发现（通过 `name()` 和 `description()`）
    /// - 参数校验（通过 `parameters_schema()`）
    /// - 执行和结果处理（通过 `execute()`）
    ///
    /// # 设计说明
    ///
    /// 此工具不执行任何实际操作，仅返回固定结果。这使得测试能够：
    /// 1. 验证工具调用机制是否正常工作
    /// 2. 检查工具结果是否被正确传递给下一轮对话
    /// 3. 确认历史记录中工具消息的格式
    ///
    /// 对于需要验证参数处理或执行不同结果的测试，应创建专门的 Mock 工具。
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 在 Agent 中注册 MockTool
    /// let mut agent = Agent::builder()
    ///     .provider(provider)
    ///     .tools(vec![Box::new(MockTool)])
    ///     .build()?;
    ///
    /// // 当 Provider 返回工具调用 { name: "echo", ... } 时，
    /// // Agent 将执行 MockTool 并获得固定结果 "tool-out"
    /// ```
    struct MockTool;

    #[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
    #[cfg_attr(not(target_arch = "wasm32"), async_trait)]
    impl Tool for MockTool {
        /// 返回工具名称 "echo"
        ///
        /// 此名称必须与 Provider 返回的 `ToolCall::name` 匹配，
        /// Agent 才能正确路由到此工具。
        fn name(&self) -> &str {
            "echo"
        }

        /// 返回工具描述 "echo"
        ///
        /// 在实际应用中，此描述会显示给 LLM 帮助其理解工具用途。
        /// 测试中使用简化描述。
        fn description(&self) -> &str {
            "echo"
        }

        /// 返回空的 JSON Schema 参数定义
        ///
        /// 定义为空对象类型，表示此工具不接受任何参数。
        /// 这简化了测试，避免了参数验证的复杂性。
        ///
        /// # 格式
        ///
        /// ```json
        /// {"type": "object"}
        /// ```
        ///
        /// # 注意
        ///
        /// 在生产代码中，参数 schema 应该详细描述所有参数及其约束。
        fn parameters_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }

        /// 执行工具操作
        ///
        /// # 参数
        ///
        /// - `_args`: 工具参数（被忽略，因为此工具不需要参数）
        ///
        /// # 返回
        ///
        /// 始终返回成功的 `ToolResult`：
        /// - `success`: true
        /// - `output`: "tool-out"
        /// - `error`: None
        ///
        /// # 确定性
        ///
        /// 此实现保证每次调用返回相同结果，确保测试的可重复性。
        /// 没有任何外部依赖（网络、文件系统、随机数等）。
        async fn execute(
            &self,
            _args: serde_json::Value,
        ) -> Result<crate::app::agent::tools::ToolResult> {
            // 返回固定的成功结果，用于验证工具调用流程
            Ok(crate::app::agent::tools::ToolResult {
                success: true,
                output: "tool-out".into(),
                error: None,
            })
        }
    }

    /// 测试：无工具调用时返回纯文本
    ///
    /// 验证当 Provider 返回纯文本响应（无 tool_calls）时，
    /// Agent 正确提取并返回文本内容。
    ///
    /// # 测试场景
    ///
    /// 模拟最简单的对话场景：用户提问，LLM 直接回答，不调用任何工具。
    /// 这是 Agent 的基本工作模式。
    ///
    /// # 测试流程
    ///
    /// 1. 创建返回 "hello" 文本的 MockProvider
    ///    - `text`: Some("hello")
    ///    - `tool_calls`: 空向量
    /// 2. 构建带有 MockTool 的 Agent（工具存在但不被调用）
    /// 3. 调用 `agent.turn("hi")`
    /// 4. 断言返回值为 "hello"
    ///
    /// # 验证点
    ///
    /// - Agent 能正确解析 `ChatResponse::text` 字段
    /// - 无工具调用时不进入工具执行流程
    /// - 响应直接返回，无额外处理
    ///
    /// # 预期行为
    ///
    /// ```text
    /// User: "hi"
    ///   ↓
    /// Agent → Provider
    ///   ↓
    /// Provider: { text: "hello", tool_calls: [] }
    ///   ↓
    /// Agent: 检测到 tool_calls 为空，直接返回 text
    ///   ↓
    /// 返回: "hello"
    /// ```
    #[tokio::test]
    async fn turn_without_tools_returns_text() {
        // 创建返回纯文本响应的 Mock Provider
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![crate::app::agent::providers::ChatResponse {
                text: Some("hello".into()),
                tool_calls: vec![], // 无工具调用
                usage: None,
                reasoning_content: None,
            }]),
        });

        // 配置 none 后端的内存，避免文件系统依赖
        let memory_cfg = crate::app::agent::config::MemoryConfig {
            backend: "none".into(),
            ..crate::app::agent::config::MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> = Arc::from(
            crate::app::agent::memory::create_memory(
                &memory_cfg,
                std::path::Path::new("/tmp"),
                None,
            )
            .expect("memory creation should succeed with valid config"),
        );

        // 使用 NoopObserver 避免观测副作用
        let observer: Arc<dyn Observer> =
            Arc::from(crate::app::agent::observability::NoopObserver {});

        // 构建 Agent 实例
        // 注意：MockTool 被注册但在此测试中不会被调用
        let mut agent = Agent::builder()
            .provider(provider)
            .tools(vec![Box::new(MockTool)])
            .memory(mem)
            .observer(observer)
            .workspace_dir(std::path::PathBuf::from("/tmp"))
            .build()
            .expect("agent builder should succeed with valid config");

        // 执行单轮对话
        let response = agent.turn("hi").await.unwrap();

        // 验证返回的文本内容
        assert_eq!(response, "hello");
    }

    /// 测试：原生调度器处理工具调用结果
    ///
    /// 验证当 Provider 返回包含 tool_calls 的响应时，
    /// Agent 能够：
    /// 1. 正确执行工具
    /// 2. 将工具结果反馈给 Provider
    /// 3. 继续循环直到获得最终文本响应
    /// 4. 在历史记录中保留工具消息
    ///
    /// # 测试场景
    ///
    /// 模拟 Agent 的工具调用循环：
    /// - 用户提问
    /// - LLM 决定调用工具
    /// - Agent 执行工具并返回结果
    /// - LLM 基于工具结果生成最终回答
    ///
    /// # 测试流程
    ///
    /// 1. 创建返回两个响应的 MockProvider：
    ///    - **第一个响应**：包含 echo 工具调用
    ///      - `text`: 空字符串
    ///      - `tool_calls`: [ToolCall { kind: Default::default(), namespace: None, id: "tc1", name: "echo", arguments: "{}" }]
    ///    - **第二个响应**：纯文本最终结果
    ///      - `text`: "done"
    ///      - `tool_calls`: 空向量
    /// 2. 调用 `agent.turn("hi")`
    /// 3. 断言最终返回 "done"
    /// 4. 断言历史记录中存在 role="tool" 的消息
    ///
    /// # 验证点
    ///
    /// - 工具调用被正确识别和执行
    /// - 工具结果被正确格式化并添加到对话历史
    /// - Agent 循环继续直到收到无工具调用的响应
    /// - 历史记录包含完整的工具消息（用于后续对话）
    ///
    /// # 预期行为
    ///
    /// ```text
    /// User: "hi"
    ///   ↓
    /// Agent → Provider: ChatRequest { messages: ["hi"] }
    ///   ↓
    /// Provider 响应 1: { text: "", tool_calls: [echo] }
    ///   ↓
    /// Agent: 执行 echo 工具
    ///   ↓
    /// Agent → Provider: ChatRequest { messages: ["hi", tool_result("tool-out")] }
    ///   ↓
    /// Provider 响应 2: { text: "done", tool_calls: [] }
    ///   ↓
    /// Agent: 检测到 tool_calls 为空，返回 text
    ///   ↓
    /// 返回: "done"
    /// 历史记录: [user("hi"), assistant(tool_call), tool("tool-out"), assistant("done")]
    /// ```
    #[tokio::test]
    async fn turn_with_native_dispatcher_handles_tool_results_variant() {
        // 创建返回两轮响应的 Mock Provider
        let provider = Box::new(MockProvider {
            responses: Mutex::new(vec![
                // 第一轮响应：触发工具调用
                crate::app::agent::providers::ChatResponse {
                    text: Some(String::new()), // 文本为空，表示需要工具调用
                    tool_calls: vec![crate::app::agent::providers::ToolCall {
                        kind: Default::default(),
                        namespace: None,
                        id: "tc1".into(),
                        name: "echo".into(),    // 与 MockTool::name() 匹配
                        arguments: "{}".into(), // 空参数，与 MockTool 的 schema 兼容
                    }],
                    usage: None,
                    reasoning_content: None,
                },
                // 第二轮响应：返回最终结果
                crate::app::agent::providers::ChatResponse {
                    text: Some("done".into()), // 最终文本响应
                    tool_calls: vec![],        // 无更多工具调用，结束循环
                    usage: None,
                    reasoning_content: None,
                },
            ]),
        });

        // 配置 none 后端的内存
        let memory_cfg = crate::app::agent::config::MemoryConfig {
            backend: "none".into(),
            ..crate::app::agent::config::MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> = Arc::from(
            crate::app::agent::memory::create_memory(
                &memory_cfg,
                std::path::Path::new("/tmp"),
                None,
            )
            .expect("memory creation should succeed with valid config"),
        );

        // 使用 NoopObserver
        let observer: Arc<dyn Observer> =
            Arc::from(crate::app::agent::observability::NoopObserver {});

        // 构建 Agent
        let mut agent = Agent::builder()
            .provider(provider)
            .tools(vec![Box::new(MockTool)]) // 注册 MockTool 以响应 "echo" 调用
            .memory(mem)
            .observer(observer)
            .workspace_dir(std::path::PathBuf::from("/tmp"))
            .build()
            .expect("agent builder should succeed with valid config");

        // 执行对话，触发工具调用循环
        let response = agent.turn("hi").await.unwrap();

        // 验证最终响应
        assert_eq!(response, "done");

        // 验证历史记录中包含工具消息
        // 这确保工具调用结果被正确记录，可用于后续对话
        assert!(agent.history().iter().any(|msg| msg.role == "tool"));
    }

    /// 测试：查询分类匹配时按 hint 路由模型
    ///
    /// 验证查询分类功能：
    /// - 当用户查询包含配置的关键词时，Agent 使用对应 hint 路由的模型
    /// - 模型名称格式为 `hint:<hint值>`
    ///
    /// # 测试场景
    ///
    /// 模拟查询分类和模型路由：
    /// 1. 配置分类规则：关键词 "quick" -> hint "fast"
    /// 2. 配置路由映射：hint "fast" -> 模型 "anthropic/claude-haiku-4-5"
    /// 3. 输入包含关键词的查询
    /// 4. 验证 Provider 接收到正确的模型标识符
    ///
    /// # 测试配置
    ///
    /// - **分类规则**：
    ///   - 关键词: "quick"
    ///   - hint: "fast"
    ///   - 优先级: 10
    /// - **路由映射**：
    ///   - "fast" -> "anthropic/claude-haiku-4-5"
    /// - **输入查询**：
    ///   - "quick summary please"（包含关键词 "quick"）
    ///
    /// # 验证点
    ///
    /// - 关键词匹配正确识别（"quick" 在查询中）
    /// - hint 正确提取（"fast"）
    /// - 模型标识符正确格式化（"hint:fast"）
    /// - Provider 接收到格式化的模型名称
    /// - 最终响应正确返回
    ///
    /// # 预期行为
    ///
    /// ```text
    /// User: "quick summary please"
    ///   ↓
    /// Agent: 分析查询
    ///   ↓
    /// 分类器: 匹配到关键词 "quick" -> hint "fast"
    ///   ↓
    /// 路由器: hint "fast" -> 模型 "anthropic/claude-haiku-4-5"
    ///   ↓
    /// Agent → Provider: ChatRequest { model: "hint:fast" }
    ///   ↓
    /// Provider: { text: "classified", tool_calls: [] }
    ///   ↓
    /// 返回: "classified"
    /// ```
    ///
    /// # 注意事项
    ///
    /// - 模型标识符格式 "hint:fast" 是内部表示，用于路由器识别 hint 分类
    /// - 实际发送到 LLM API 的模型名称由底层 Provider 解析
    /// - 此测试使用 ModelCaptureProvider 捕获内部模型标识符
    #[tokio::test]
    async fn turn_routes_with_hint_when_query_classification_matches() {
        // 创建模型名称捕获器
        let seen_models = Arc::new(Mutex::new(Vec::new()));

        // 创建捕获 Provider
        let provider = Box::new(ModelCaptureProvider {
            responses: Mutex::new(vec![crate::app::agent::providers::ChatResponse {
                text: Some("classified".into()),
                tool_calls: vec![],
                usage: None,
                reasoning_content: None,
            }]),
            seen_models: seen_models.clone(), // 共享引用以供后续检查
        });

        // 配置 none 内存后端
        let memory_cfg = crate::app::agent::config::MemoryConfig {
            backend: "none".into(),
            ..crate::app::agent::config::MemoryConfig::default()
        };
        let mem: Arc<dyn Memory> = Arc::from(
            crate::app::agent::memory::create_memory(
                &memory_cfg,
                std::path::Path::new("/tmp"),
                None,
            )
            .expect("memory creation should succeed with valid config"),
        );

        // 使用 NoopObserver
        let observer: Arc<dyn Observer> =
            Arc::from(crate::app::agent::observability::NoopObserver {});

        // 配置路由映射：hint -> 实际模型
        let mut route_model_by_hint = HashMap::new();
        route_model_by_hint.insert("fast".to_string(), "anthropic/claude-haiku-4-5".to_string());

        // 构建 Agent，启用查询分类
        let mut agent = Agent::builder()
            .provider(provider)
            .tools(vec![Box::new(MockTool)])
            .memory(mem)
            .observer(observer)
            .workspace_dir(std::path::PathBuf::from("/tmp"))
            // 启用查询分类并配置规则
            .classification_config(crate::app::agent::config::QueryClassificationConfig {
                enabled: true,
                rules: vec![crate::app::agent::config::ClassificationRule {
                    hint: "fast".to_string(),            // 分类后的 hint
                    keywords: vec!["quick".to_string()], // 触发关键词
                    patterns: vec![],                    // 无正则模式
                    min_length: None,                    // 无最小长度限制
                    max_length: None,                    // 无最大长度限制
                    priority: 10,                        // 优先级
                }],
            })
            // 声明可用的 hint 值
            .available_hints(vec!["fast".to_string()])
            // 设置 hint 到模型的映射
            .route_model_by_hint(route_model_by_hint)
            .build()
            .expect("agent builder should succeed with valid config");

        // 执行包含分类关键词的查询
        let response = agent.turn("quick summary please").await.unwrap();

        // 验证响应内容
        assert_eq!(response, "classified");

        // 验证模型路由：Provider 应收到 "hint:fast" 而非原始模型名称
        let seen = seen_models.lock();
        assert_eq!(seen.as_slice(), &["hint:fast".to_string()]);
    }
}
