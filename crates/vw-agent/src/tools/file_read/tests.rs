//! 真实宿主执行器读状态与代理/Provider 集成测试。
use super::*;
use tool_types::Tool;
use serde_json::json;
use std::sync::Arc;

#[allow(dead_code)]
mod tests {
    use super::*;
    use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
    use crate::app::agent::tools::{
        FileReadStateEntry, ToolUseContext, execute_tool_from_registry,
    };

    /// 创建基础测试安全策略
    ///
    /// 使用默认配置创建一个监督级别的安全策略，用于大多数基本测试。
    ///
    /// # 参数
    /// - `workspace`: 测试工作区目录路径
    ///
    /// # 返回
    /// 返回包装在 `Arc` 中的安全策略实例
    fn test_security(workspace: std::path::PathBuf) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            allowed_roots: vec![workspace.clone()],
            workspace_dir: workspace,
            ..SecurityPolicy::default()
        })
    }

    fn minimal_pdf_bytes() -> Vec<u8> {
        let stream = "BT\n/F1 12 Tf\n72 720 Td\n(Hello PDF) Tj\nET";
        let objects = [
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
            format!("<< /Length {} >>\nstream\n{}\nendstream", stream.len(), stream),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        ];

        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::with_capacity(objects.len() + 1);
        offsets.push(0usize);

        for (index, object) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{}\nendobj\n", index + 1, object).as_bytes());
        }

        let xref_offset = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets.iter().skip(1) {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                objects.len() + 1,
                xref_offset
            )
            .as_bytes(),
        );
        pdf
    }

    fn read_state_entry(
        context: &Arc<ToolUseContext>,
        root: &std::path::Path,
        path: &str,
    ) -> Option<FileReadStateEntry> {
        let handle = context.read_state_handle();
        let mut state = handle.lock().unwrap_or_else(|error| error.into_inner());
        state.get(Some(root), path)
    }

    #[tokio::test]
    async fn file_read_returns_file_unchanged_for_duplicate_runtime_read() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_unchanged");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("repeat.txt"), "same content").await.unwrap();

        let tools: Vec<Box<dyn Tool>> =
            vec![Box::new(FileReadTool::new(test_security(dir.clone())))];
        let context = Arc::new(ToolUseContext::new(
            "file-read-unchanged",
            Some(dir.to_string_lossy().to_string()),
        ));

        let first = execute_tool_from_registry(
            &tools,
            "file_read",
            json!({"path": "repeat.txt"}),
            context.clone(),
        )
        .await
        .expect("first read succeeds");
        assert_eq!(first.result.data["kind"], json!("text"));

        let second = execute_tool_from_registry(
            &tools,
            "file_read",
            json!({"path": "repeat.txt"}),
            context.clone(),
        )
        .await
        .expect("duplicate read succeeds");

        assert_eq!(second.result.data["kind"], json!("file_unchanged"));
        assert_eq!(second.result.data["result_kind"], json!("text"));
        assert_eq!(second.result.data["partial_view"], json!(false));
        assert!(second.result.model_result.as_str().unwrap_or("").contains("<file_unchanged>"));
        assert_eq!(context.read_state_snapshot().len(), 1);

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_read_marks_default_truncation_as_partial_in_read_state() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_partial_default_limit");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();

        let content = (1..=2105).map(|line| format!("line-{line}")).collect::<Vec<_>>().join("\n");
        tokio::fs::write(dir.join("large.txt"), content).await.unwrap();

        let tools: Vec<Box<dyn Tool>> =
            vec![Box::new(FileReadTool::new(test_security(dir.clone())))];
        let context = Arc::new(ToolUseContext::new(
            "file-read-partial-default-limit",
            Some(dir.to_string_lossy().to_string()),
        ));

        let first = execute_tool_from_registry(
            &tools,
            "file_read",
            json!({"path": "large.txt"}),
            context.clone(),
        )
        .await
        .expect("first read succeeds");

        assert_eq!(first.result.data["kind"], json!("text"));
        assert_eq!(first.result.data["has_more"], json!(true));

        let entry =
            read_state_entry(&context, dir.as_path(), "large.txt").expect("read_state entry");
        assert!(entry.partial_view, "default truncated read should be marked partial");
        assert_eq!(entry.offset, None);
        assert_eq!(entry.limit, None);

        let second = execute_tool_from_registry(
            &tools,
            "file_read",
            json!({"path": "large.txt"}),
            context.clone(),
        )
        .await
        .expect("duplicate read succeeds");

        assert_eq!(second.result.data["kind"], json!("file_unchanged"));
        assert_eq!(second.result.data["partial_view"], json!(true));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    #[tokio::test]
    async fn file_read_marks_full_read_even_when_limit_is_provided() {
        let dir = std::env::temp_dir().join("vibewindow_test_file_read_limit_full");
        let _ = tokio::fs::remove_dir_all(&dir).await;
        tokio::fs::create_dir_all(&dir).await.unwrap();
        tokio::fs::write(dir.join("short.txt"), "one\ntwo").await.unwrap();

        let tools: Vec<Box<dyn Tool>> =
            vec![Box::new(FileReadTool::new(test_security(dir.clone())))];
        let context = Arc::new(ToolUseContext::new(
            "file-read-limit-full",
            Some(dir.to_string_lossy().to_string()),
        ));

        let result = execute_tool_from_registry(
            &tools,
            "file_read",
            json!({"path": "short.txt", "limit": 10}),
            context.clone(),
        )
        .await
        .expect("read succeeds");

        assert_eq!(result.result.data["kind"], json!("text"));
        assert_eq!(result.result.data["has_more"], json!(false));

        let entry =
            read_state_entry(&context, dir.as_path(), "short.txt").expect("read_state entry");
        assert!(
            !entry.partial_view,
            "full read should not be marked partial just because limit was provided"
        );
        assert_eq!(entry.limit, Some(10));

        let _ = tokio::fs::remove_dir_all(&dir).await;
    }

    // ═══════════════════════════════════════════════════════════════════════════════
    // E2E 测试：完整代理流水线集成测试（真实 FileReadTool + PDF 提取）
    // ═══════════════════════════════════════════════════════════════════════════════

    /// E2E 测试辅助工具模块
    ///
    /// 提供端到端测试所需的基础设施：
    /// - `RecordingProvider`: 记录所有请求的模拟 Provider
    /// - `make_memory`: 创建测试用内存后端
    /// - `make_observer`: 创建空操作观察器
    mod e2e_helpers {
        use crate::app::agent::config::MemoryConfig;
        use crate::app::agent::memory::{self, Memory};
        use crate::app::agent::observability::{NoopObserver, Observer};
        use model_types::{ChatMessage, ChatRequest, ChatResponse, Provider};
        use std::sync::{Arc, Mutex};

        /// 共享请求记录类型
        ///
        /// 用于在测试中访问 Provider 接收到的所有聊天请求历史。
        pub type SharedRequests = Arc<Mutex<Vec<Vec<ChatMessage>>>>;

        /// 可录制请求的模拟 Provider
        ///
        /// 预设响应序列并记录所有接收到的请求，
        /// 用于验证代理与工具的交互流程。
        pub struct RecordingProvider {
            /// 预设的响应队列
            responses: Mutex<Vec<ChatResponse>>,
            /// 记录的所有请求历史
            pub requests: SharedRequests,
        }

        impl RecordingProvider {
            /// 创建新的录制 Provider
            ///
            /// # 参数
            /// - `responses`: 预设的响应序列，按先进先出顺序返回
            ///
            /// # 返回
            /// 返回 Provider 实例和共享请求记录器的元组
            pub fn new(responses: Vec<ChatResponse>) -> (Self, SharedRequests) {
                let requests: SharedRequests = Arc::new(Mutex::new(Vec::new()));
                let provider =
                    Self { responses: Mutex::new(responses), requests: requests.clone() };
                (provider, requests)
            }
        }

        #[async_trait::async_trait]
        impl Provider for RecordingProvider {
            async fn chat_with_system(
                &self,
                _system_prompt: Option<&str>,
                _message: &str,
                _model: &str,
                _temperature: f64,
            ) -> anyhow::Result<String> {
                Ok("fallback".into())
            }

            async fn chat(
                &self,
                request: ChatRequest<'_>,
                _model: &str,
                _temperature: f64,
            ) -> anyhow::Result<ChatResponse> {
                // 记录本次请求的消息历史
                self.requests.lock().unwrap().push(request.messages.to_vec());

                let mut guard = self.responses.lock().unwrap();
                if guard.is_empty() {
                    // 响应队列为空时返回默认完成响应
                    return Ok(ChatResponse {
                        text: Some("done".into()),
                        tool_calls: vec![],
                        usage: None,
                        reasoning_content: None,
                    });
                }
                // 返回队列中的下一个预设响应
                Ok(guard.remove(0))
            }
        }

        /// 创建测试用内存后端
        ///
        /// 使用 "none" 后端配置，适用于不需要持久化的测试场景。
        pub fn make_memory() -> Arc<dyn Memory> {
            let cfg = MemoryConfig { backend: "none".into(), ..MemoryConfig::default() };
            Arc::from(memory::create_memory(&cfg, &std::env::temp_dir(), None).unwrap())
        }

        /// 创建空操作观察器
        ///
        /// 用于不需要观测输出的测试场景。
        pub fn make_observer() -> Arc<dyn Observer> {
            Arc::from(NoopObserver {})
        }
    }

    /// E2E 测试：脚本化 Provider 调用 file_read 读取真实 PDF
    ///
    /// 测试完整代理流水线：
    /// 1. Provider 请求调用 `file_read` 工具
    /// 2. 工具通过 pdf-extract 提取 PDF 文本
    /// 3. 提取的内容出现在工具结果消息中
    /// 4. Provider 基于工具结果生成最终回复
    #[tokio::test]
    async fn e2e_agent_file_read_pdf_extraction() {
        use crate::app::agent::agent::agent::Agent;
        use model_types::{ChatResponse, Provider, ToolCall};
        use e2e_helpers::*;

        // ── 设置包含 PDF 测试文件的工作区 ──
        let workspace = std::env::temp_dir().join("vibewindow_test_e2e_file_read_pdf");
        let _ = tokio::fs::remove_dir_all(&workspace).await;
        tokio::fs::create_dir_all(&workspace).await.unwrap();

        tokio::fs::write(workspace.join("report.pdf"), minimal_pdf_bytes()).await.unwrap();

        // ── 构建真实 FileReadTool ──
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            allowed_roots: vec![workspace.clone()],
            workspace_dir: workspace.clone(),
            ..SecurityPolicy::default()
        });
        let file_read_tool: Box<dyn Tool> = Box::new(FileReadTool::new(security));

        // ── 脚本化 Provider：调用 file_read → 然后回答 ──
        let (provider, recorded) = RecordingProvider::new(vec![
            // 第 1 轮响应：Provider 请求读取 PDF
            ChatResponse {
                text: Some(String::new()),
                tool_calls: vec![ToolCall {
                    kind: Default::default(),
                    namespace: None,
                    id: "tc1".into(),
                    name: "file_read".into(),
                    arguments: r#"{"path": "report.pdf"}"#.into(),
                }],
                usage: None,
                reasoning_content: None,
            },
            // 第 1 轮继续：Provider 看到工具结果后回答
            ChatResponse {
                text: Some("The PDF contains a greeting: Hello PDF".into()),
                tool_calls: vec![],
                usage: None,
                reasoning_content: None,
            },
        ]);

        let mut agent = Agent::builder()
            .provider(Box::new(provider) as Box<dyn Provider>)
            .tools(vec![file_read_tool])
            .memory(make_memory())
            .observer(make_observer())
            .workspace_dir(workspace.clone())
            .build()
            .unwrap();

        // ── 执行代理 ──
        let response = agent.turn("Read report.pdf and tell me what it says").await.unwrap();

        // ── 验证最终响应 ──
        assert!(
            response.contains("Hello PDF"),
            "agent response must contain PDF content, got: {response}",
        );

        // ── 验证 Provider 在工具结果中收到提取的 PDF 文本 ──
        {
            let all_requests = recorded.lock().unwrap();
            assert!(
                all_requests.len() >= 2,
                "expected at least 2 provider requests (initial + after tool), got {}",
                all_requests.len(),
            );

            let second_request = &all_requests[1];
            let tool_result_msg = second_request
                .iter()
                .find(|m| m.role == "tool")
                .expect("second request must contain a tool result message");

            assert!(
                tool_result_msg.content.contains("Hello"),
                "tool result must contain extracted PDF text 'Hello', got: {}",
                tool_result_msg.content,
            );
        }

        let _ = tokio::fs::remove_dir_all(&workspace).await;
    }

    /// E2E 测试：代理读取二进制文件获得 lossy UTF-8 输出
    ///
    /// 验证工具结果中包含带替换字符的容错输出，
    /// 证明二进制文件能够被安全读取。
    #[tokio::test]
    async fn e2e_agent_file_read_lossy_binary() {
        use crate::app::agent::agent::agent::Agent;
        use model_types::{ChatResponse, Provider, ToolCall};
        use e2e_helpers::*;

        // ── 设置包含二进制文件的工作区 ──
        let workspace = std::env::temp_dir().join("vibewindow_test_e2e_file_read_lossy");
        let _ = tokio::fs::remove_dir_all(&workspace).await;
        tokio::fs::create_dir_all(&workspace).await.unwrap();

        let binary_data: Vec<u8> = vec![0x00, 0x80, 0xFF, 0xFE, b'v', b'a', b'l', b'i', b'd', 0x80];
        tokio::fs::write(workspace.join("data.bin"), &binary_data).await.unwrap();

        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            allowed_roots: vec![workspace.clone()],
            workspace_dir: workspace.clone(),
            ..SecurityPolicy::default()
        });
        let file_read_tool: Box<dyn Tool> = Box::new(FileReadTool::new(security));

        let (provider, recorded) = RecordingProvider::new(vec![
            ChatResponse {
                text: Some(String::new()),
                tool_calls: vec![ToolCall {
                    kind: Default::default(),
                    namespace: None,
                    id: "tc1".into(),
                    name: "file_read".into(),
                    arguments: r#"{"path": "data.bin"}"#.into(),
                }],
                usage: None,
                reasoning_content: None,
            },
            ChatResponse {
                text: Some("The file appears to be binary data.".into()),
                tool_calls: vec![],
                usage: None,
                reasoning_content: None,
            },
        ]);

        let mut agent = Agent::builder()
            .provider(Box::new(provider) as Box<dyn Provider>)
            .tools(vec![file_read_tool])
            .memory(make_memory())
            .observer(make_observer())
            .workspace_dir(workspace.clone())
            .build()
            .unwrap();

        let response = agent.turn("Read data.bin").await.unwrap();

        assert!(response.contains("binary"), "agent response must mention binary, got: {response}",);

        // 验证工具结果包含带替换字符的 lossy 输出
        {
            let all_requests = recorded.lock().unwrap();
            assert!(
                all_requests.len() >= 2,
                "expected at least 2 provider requests, got {}",
                all_requests.len(),
            );

            let tool_result_msg = all_requests[1]
                .iter()
                .find(|m| m.role == "tool")
                .expect("second request must contain a tool result message");

            assert!(
                tool_result_msg.content.contains("valid"),
                "tool result must preserve valid ASCII from binary file, got: {}",
                tool_result_msg.content,
            );
            assert!(
                tool_result_msg.content.contains('\u{FFFD}'),
                "tool result must contain replacement character for invalid bytes, got: {}",
                tool_result_msg.content,
            );
        }

        let _ = tokio::fs::remove_dir_all(&workspace).await;
    }

    /// 实时 E2E 测试：真实 OpenAI Codex Provider + 真实 FileReadTool + PDF 文件
    ///
    /// 验证模型能够接收提取的 PDF 文本并做出有意义的响应。
    /// 此测试需要有效的 OAuth 凭证配置在 `~/.shiju-harness/` 目录。
    ///
    /// 运行命令：
    /// ```bash
    /// cargo test --lib -- crate::app::agent::tools::file_read::tests::e2e_live_file_read_pdf --ignored --nocapture
    /// ```
    #[tokio::test]
    #[ignore = "requires valid OpenAI Codex OAuth credentials"]
    async fn e2e_live_file_read_pdf() {
        use crate::app::agent::agent::agent::Agent;
        use crate::app::agent::providers::{self, ProviderRuntimeOptions};
        use e2e_helpers::*;

        // ── 设置包含 PDF 测试文件的工作区 ──
        let workspace = std::env::temp_dir().join("vibewindow_test_e2e_live_file_read_pdf");
        let _ = tokio::fs::remove_dir_all(&workspace).await;
        tokio::fs::create_dir_all(&workspace).await.unwrap();

        tokio::fs::write(workspace.join("report.pdf"), minimal_pdf_bytes()).await.unwrap();

        // ── 构建真实 FileReadTool ──
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            allowed_roots: vec![workspace.clone()],
            workspace_dir: workspace.clone(),
            ..SecurityPolicy::default()
        });
        let file_read_tool: Box<dyn Tool> = Box::new(FileReadTool::new(security));

        // ── 真实 Provider（通过统一 providers 工厂解析）──
        let provider = providers::create_provider_with_options(
            "openai-codex",
            None,
            &ProviderRuntimeOptions::default(),
        )
        .expect("provider should initialize");

        let mut agent = Agent::builder()
            .provider(provider)
            .tools(vec![file_read_tool])
            .memory(make_memory())
            .observer(make_observer())
            .workspace_dir(workspace.clone())
            .model_name("gpt-5.3-codex".to_string())
            .build()
            .unwrap();

        // ── 执行代理 ──
        let response = agent
            .turn("Use the file_read tool to read report.pdf, then tell me what text it contains. Be concise.")
            .await
            .unwrap();

        eprintln!("=== Live e2e response ===\n{response}\n=========================");

        // ── 验证模型看到了实际的 PDF 内容（"Hello PDF"）──
        let lower = response.to_lowercase();
        assert!(
            lower.contains("hello"),
            "model response must reference extracted PDF text 'Hello PDF', got: {response}",
        );

        let _ = tokio::fs::remove_dir_all(&workspace).await;
    }

}
