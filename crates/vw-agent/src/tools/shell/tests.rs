//! Shell 工具的单元测试模块
//!
//! 本模块包含对 `ShellTool` 的全面测试用例，覆盖以下功能领域：
//! - 基本命令执行与结果捕获
//! - 安全策略验证（命令白名单、路径访问控制、重定向策略）
//! - 环境变量隔离与敏感信息保护
//! - 速率限制与配额管理
//! - 系统调用异常检测集成
//!
//! 所有测试均遵循最小权限原则，使用独立的安全策略配置。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::config::{AuditConfig, SyscallAnomalyConfig};
    use crate::app::agent::runtime::{NativeRuntime, RuntimeAdapter};
    use crate::app::agent::security::{
        AutonomyLevel, SecurityPolicy, ShellRedirectPolicy, SyscallAnomalyDetector,
    };
    use tempfile::TempDir;

    /// 创建用于测试的基础安全策略
    ///
    /// # 参数
    /// - `autonomy`: 自主级别，决定代理的执行权限范围
    ///
    /// # 返回
    /// 返回配置了指定自主级别的 `SecurityPolicy`，工作目录设置为系统临时目录
    fn test_security(autonomy: AutonomyLevel) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy,
            workspace_dir: std::env::temp_dir(),
            ..SecurityPolicy::default()
        })
    }

    #[tokio::test]
    async fn shell_call_preserves_real_exit_code_without_parsing_output() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
        for (command, expected) in
            [("grep '[' missing-recovery-fixture", 2), ("echo exit_code=2", 0)]
        {
            let security = Arc::new(SecurityPolicy {
                autonomy: AutonomyLevel::Full,
                workspace_only: false,
                workspace_dir: std::env::temp_dir(),
                ..SecurityPolicy::default()
            });
            let tool = ShellTool::new(security, test_runtime());
            let result = tool.call(json!({ "command": command, "description": "verify process status", "approved": true })).await.unwrap();
            assert_eq!(result.data["exit_code"], expected, "{:?}", result.data);
            assert_eq!(result.render_metadata()["exit_code"], expected);
            assert_eq!(result.is_success(), expected == 0);
        }
        let tool = ShellTool::new(test_security(AutonomyLevel::ReadOnly), test_runtime());
        let result = tool
            .call(json!({ "command": "touch denied", "description": "verify denied operation" }))
            .await
            .unwrap();
        assert!(!result.is_success());
        assert!(result.data.get("exit_code").is_none());
        assert!(result.render_metadata().get("exit_code").is_none());
        }).await
    }

    #[test]
    fn shell_full_access_disables_workspace_sandbox() {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Full,
            workspace_only: false,
            workspace_dir: std::env::temp_dir(),
            ..SecurityPolicy::default()
        });

        let tool = ShellTool::new(security, test_runtime());

        assert!(!tool.sandbox_config.enabled);
    }

    #[test]
    fn shell_workspace_scoped_full_autonomy_keeps_sandbox() {
        let tool = ShellTool::new(test_security(AutonomyLevel::Full), test_runtime());

        assert!(tool.sandbox_config.enabled);
    }

    /// 创建带有自定义重定向策略的测试安全策略
    ///
    /// # 参数
    /// - `autonomy`: 自主级别
    /// - `shell_redirect_policy`: Shell 重定向策略（Allow/Strip/Block）
    ///
    /// # 返回
    /// 返回配置了指定重定向策略的 `SecurityPolicy`
    fn test_security_with_redirect_policy(
        autonomy: AutonomyLevel,
        shell_redirect_policy: ShellRedirectPolicy,
    ) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy,
            workspace_dir: std::env::temp_dir(),
            shell_redirect_policy,
            ..SecurityPolicy::default()
        })
    }

    /// 创建本机运行时适配器用于测试
    ///
    /// # 返回
    /// 返回封装了 `NativeRuntime` 的 trait 对象
    fn test_runtime() -> Arc<dyn RuntimeAdapter> {
        Arc::new(NativeRuntime::new())
    }

    /// 创建用于测试的系统调用异常检测器
    ///
    /// # 参数
    /// - `tmp`: 临时目录引用，用于存储日志文件
    ///
    /// # 返回
    /// 返回配置了基准系统调用列表和日志路径的 `SyscallAnomalyDetector`
    ///
    /// # 配置说明
    /// - 基准系统调用：仅包含 read/write
    /// - 告警冷却时间：1 秒
    /// - 每分钟最大告警数：50
    fn test_syscall_detector(tmp: &TempDir) -> Arc<SyscallAnomalyDetector> {
        let log_path = tmp.path().join("shell-syscall-anomalies.log");
        let cfg = SyscallAnomalyConfig {
            enabled: true,
            alert_on_unknown_syscall: true,
            baseline_syscalls: vec!["read".into(), "write".into()],
            log_path: log_path.to_string_lossy().to_string(),
            alert_cooldown_secs: 1,
            max_alerts_per_minute: 50,
            ..SyscallAnomalyConfig::default()
        };
        let audit = AuditConfig { enabled: false, ..AuditConfig::default() };
        Arc::new(SyscallAnomalyDetector::new(cfg, tmp.path(), audit))
    }

    /// 测试 Shell 工具的名称是否正确
    #[test]
    fn shell_tool_name() {
        let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
        assert_eq!(tool.name(), "shell");
    }

    /// 测试模型侧工具规格是否统一暴露为 bash
    #[test]
    fn shell_tool_spec_uses_bash_id() {
        let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
        let spec = tool.spec();

        assert_eq!(spec.id, "bash");
        assert!(spec.aliases.iter().any(|alias| alias == "shell"));
    }

    /// 测试 Shell 工具的描述是否非空
    #[test]
    fn shell_tool_description() {
        let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
        assert!(!tool.description().is_empty());
    }

    /// 测试 Shell 工具的参数 schema 是否包含必需字段
    ///
    /// 验证内容：
    /// - schema 包含 `command` 属性
    /// - `command` 被标记为必需字段
    /// - schema 包含 `approved` 属性
    #[test]
    fn shell_tool_schema_has_command() {
        let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
        let schema = tool.parameters_schema();
        assert!(schema["properties"]["command"].is_object());
        assert!(schema["properties"]["description"].is_object());
        assert!(schema["properties"]["timeout"].is_object());
        assert!(schema["properties"]["workdir"].is_object());
        assert!(
            schema["required"]
                .as_array()
                .expect("schema required field should be an array")
                .contains(&json!("command"))
        );
        assert!(
            schema["required"]
                .as_array()
                .expect("schema required field should be an array")
                .contains(&json!("description"))
        );
        assert!(schema["properties"]["approved"].is_object());
    }

    /// 测试命令参数提取是否支持多种别名
    ///
    /// 验证 `extract_command_argument` 函数能够识别：
    /// - `cmd` 字段
    /// - `script` 字段
    /// - 直接传入字符串值
    #[test]
    fn extract_command_argument_supports_aliases() {
        assert_eq!(
            extract_command_argument(&json!({"cmd": "echo from-cmd"})).as_deref(),
            Some("echo from-cmd")
        );
        assert_eq!(
            extract_command_argument(&json!({"script": "echo from-script"})).as_deref(),
            Some("echo from-script")
        );
        assert_eq!(
            extract_command_argument(&json!("echo from-string")).as_deref(),
            Some("echo from-string")
        );
    }

    #[test]
    fn shell_argument_helpers_trim_defaults_and_reject_invalid_timeouts() {
        assert_eq!(
            extract_description_argument(&json!({"description": "  inspect files  "})).as_deref(),
            Some("inspect files")
        );
        assert_eq!(extract_description_argument(&json!({"description": "  "})), None);
        assert_eq!(parse_timeout_ms(&json!({})).unwrap(), None);
        assert_eq!(parse_timeout_ms(&json!({"timeout": 17})).unwrap(), Some(17));
        assert!(
            parse_timeout_ms(&json!({"timeout": 0}))
                .unwrap_err()
                .to_string()
                .contains("greater than 0")
        );
        assert!(parse_timeout_ms(&json!({"timeout": -1})).is_err());
        assert!(parse_timeout_ms(&json!({"timeout": "17"})).is_err());
    }

    #[test]
    fn output_helpers_normalize_merge_and_append_notes_without_extra_blank_lines() {
        assert_eq!(normalize_line_endings("one\r\ntwo\rthree".into()), "one\ntwo\nthree");
        assert_eq!(merge_shell_output("", ""), "");
        assert_eq!(merge_shell_output("out\n", ""), "out\n");
        assert_eq!(merge_shell_output("", "err\n"), "err\n");
        assert_eq!(merge_shell_output("out", "err"), "out\nerr");
        assert_eq!(append_note(String::new(), "[note]"), "[note]");
        assert_eq!(append_note("output".into(), "[note]"), "output\n[note]");
        assert_eq!(append_note("output\n".into(), "[note]"), "output\n[note]");
    }

    #[test]
    fn interpretation_results_keep_semantic_success_and_error_details() {
        for interpretation in [
            ExitInterpretation::Success,
            ExitInterpretation::ConditionTrue,
            ExitInterpretation::DifferencesFound,
        ] {
            let result =
                tool_result_from_interpretation(interpretation, "stdout".into(), String::new());
            assert!(result.success);
            assert_eq!(result.output, "stdout");
            assert!(result.error.is_none());
        }

        for (interpretation, note) in [
            (ExitInterpretation::NoMatches, "[No matches found]"),
            (ExitInterpretation::PartialSuccess, "[Command completed with partial success]"),
            (ExitInterpretation::ConditionFalse, "[Condition evaluated to false]"),
        ] {
            let result =
                tool_result_from_interpretation(interpretation, String::new(), String::new());
            assert!(result.success);
            assert_eq!(result.output, note);
            assert!(result.error.is_none());
        }

        let without_stderr = tool_result_from_interpretation(
            ExitInterpretation::Error { message: "failed".into() },
            String::new(),
            String::new(),
        );
        assert!(!without_stderr.success);
        assert_eq!(without_stderr.error.as_deref(), Some("failed"));

        let with_stderr = tool_result_from_interpretation(
            ExitInterpretation::Error { message: "failed".into() },
            String::new(),
            "details".into(),
        );
        assert!(!with_stderr.success);
        assert_eq!(with_stderr.output, "details");
        assert_eq!(with_stderr.error.as_deref(), Some("failed: details"));
    }

    #[test]
    fn dedupe_paths_preserves_first_occurrence_order() {
        let paths = vec![
            PathBuf::from("/workspace"),
            PathBuf::from("/allowed"),
            PathBuf::from("/workspace"),
        ];

        assert_eq!(
            dedupe_paths(paths),
            vec![PathBuf::from("/workspace"), PathBuf::from("/allowed")]
        );
    }

    /// 测试 Shell 工具能够执行允许的命令
    ///
    /// 验证在 Supervised 模式下，`echo hello` 命令能够成功执行，
    /// 且输出包含预期内容，无错误信息
    #[tokio::test]
    async fn shell_executes_allowed_command() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
            let result = tool
                .execute(json!({"command": "echo hello"}))
                .await
                .expect("echo command execution should succeed");
            assert!(result.success);
            assert!(result.output.trim().contains("hello"));
            assert!(result.error.is_none());
        })
        .await
    }

    /// 测试通过 cmd 别名执行命令
    ///
    /// 验证使用 `cmd` 字段而非 `command` 字段时，命令仍能正确执行
    #[tokio::test]
    async fn shell_executes_command_from_cmd_alias() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
            let result = tool
                .execute(json!({"cmd": "echo alias"}))
                .await
                .expect("cmd alias execution should succeed");
            assert!(result.success);
            assert!(result.output.trim().contains("alias"));
        })
        .await
    }

    #[tokio::test]
    async fn shell_workdir_blocks_external_absolute_path_without_allowlist() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let root = tempfile::tempdir().expect("temp root should be created");
            let workspace = root.path().join("workspace");
            let outside = root.path().join("outside");
            std::fs::create_dir_all(&workspace).expect("workspace should exist");
            std::fs::create_dir_all(&outside).expect("outside dir should exist");

            let security = Arc::new(SecurityPolicy {
                autonomy: AutonomyLevel::Full,
                workspace_dir: workspace,
                ..SecurityPolicy::default()
            });
            let tool = ShellTool::new(security, test_runtime());

            let result = tool
                .execute(json!({
                    "command": "pwd",
                    "description": "print current directory",
                    "workdir": outside.to_string_lossy().to_string()
                }))
                .await;

            let err = result.expect_err("external workdir must be rejected");
            assert!(
                err.to_string().contains("allowed_roots") || err.to_string().contains("workspace")
            );
        })
        .await
    }

    #[tokio::test]
    async fn shell_workdir_allows_absolute_path_in_allowed_roots() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let root = tempfile::tempdir().expect("temp root should be created");
            let workspace = root.path().join("workspace");
            let outside = root.path().join("outside");
            std::fs::create_dir_all(&workspace).expect("workspace should exist");
            std::fs::create_dir_all(&outside).expect("outside dir should exist");

            let security = Arc::new(SecurityPolicy {
                autonomy: AutonomyLevel::Full,
                workspace_dir: workspace,
                allowed_roots: vec![outside.clone()],
                ..SecurityPolicy::default()
            });
            let tool = ShellTool::new(security, test_runtime());

            let result = tool
                .execute(json!({
                    "command": "pwd",
                    "description": "print current directory",
                    "workdir": outside.to_string_lossy().to_string()
                }))
                .await
                .expect("allowed absolute workdir should succeed");

            assert!(result.success);
            assert!(result.output.contains(outside.to_string_lossy().as_ref()));
        })
        .await
    }

    /// 测试 Shell 工具阻止高风险命令
    ///
    /// 验证 `rm -rf /` 等危险命令被安全策略拦截，
    /// 返回失败结果且错误信息包含"not allowed"或"high-risk"
    #[tokio::test]
    async fn shell_blocks_disallowed_command() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
            let result = tool
                .execute(json!({"command": "rm -rf /"}))
                .await
                .expect("disallowed command execution should return a result");
            assert!(!result.success);
            let error = result.error.as_deref().unwrap_or("");
            assert!(error.contains("not allowed") || error.contains("high-risk"));
        })
        .await
    }

    /// 测试只读模式下显式只读命令自动放行
    ///
    /// 验证在 ReadOnly 自主级别下，`ls` 这类只读命令仍然可以执行
    #[tokio::test]
    async fn shell_allows_readonly_command_in_readonly_mode() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::ReadOnly), test_runtime());
            let result = tool
                .execute(json!({"command": "ls", "description": "list files"}))
                .await
                .expect("readonly command execution should return a result");
            assert!(result.success);
        })
        .await
    }

    /// 测试只读模式下写命令仍然被阻止
    #[tokio::test]
    async fn shell_blocks_non_readonly_command_in_readonly_mode() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::ReadOnly), test_runtime());
            let result = tool
                .execute(json!({"command": "touch created-by-test", "description": "create file"}))
                .await
                .expect("write command should return a result");
            assert!(!result.success);
            assert!(result.error.as_deref().unwrap_or("").contains("not allowed"));
        })
        .await
    }

    /// 测试缺少命令参数时返回错误
    ///
    /// 验证当输入 JSON 不包含 `command` 字段时，执行失败
    /// 且错误信息包含"command"
    #[tokio::test]
    async fn shell_missing_command_param() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
            let result = tool.execute(json!({})).await;
            assert!(result.is_err());
            assert!(result.unwrap_err().to_string().contains("command"));
        })
        .await
    }

    /// 测试命令参数类型错误时的处理
    ///
    /// 验证当 `command` 字段不是字符串类型时，执行失败
    #[tokio::test]
    async fn shell_wrong_type_param() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
            let result = tool.execute(json!({"command": 123})).await;
            assert!(result.is_err());
        })
        .await
    }

    /// 测试命令执行失败时正确捕获退出码
    ///
    /// 验证当命令访问不存在的路径时，返回失败结果
    #[tokio::test]
    async fn shell_captures_exit_code() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
            let result = tool
                .execute(json!({"command": "ls /nonexistent_dir_xyz"}))
                .await
                .expect("command with nonexistent path should return a result");
            assert!(!result.success);
        })
        .await
    }

    /// 测试阻止绝对路径参数
    ///
    /// 验证命令参数中包含绝对路径（如 /etc/passwd）时被阻止，
    /// 防止访问工作目录外的敏感文件
    #[tokio::test]
    async fn shell_blocks_absolute_path_argument() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
            let result = tool
                .execute(json!({"command": "cat /etc/passwd"}))
                .await
                .expect("absolute path argument should be blocked");
            assert!(!result.success);
            assert!(result.error.as_deref().unwrap_or("").contains("Path blocked"));
        })
        .await
    }

    /// 测试阻止选项赋值形式的路径参数
    ///
    /// 验证以 `--file=` 形式传递的绝对路径也被正确拦截
    #[tokio::test]
    async fn shell_blocks_option_assignment_path_argument() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
            let result = tool
                .execute(json!({"command": "grep --file=/etc/passwd root ./src"}))
                .await
                .expect("option-assigned forbidden path should be blocked");
            assert!(!result.success);
            assert!(result.error.as_deref().unwrap_or("").contains("Path blocked"));
        })
        .await
    }

    /// 测试阻止短选项紧贴形式的路径参数
    ///
    /// 验证以 `-f/etc/passwd` 形式（无空格）传递的绝对路径也被拦截
    #[tokio::test]
    async fn shell_blocks_short_option_attached_path_argument() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
            let result = tool
                .execute(json!({"command": "grep -f/etc/passwd root ./src"}))
                .await
                .expect("short option attached forbidden path should be blocked");
            assert!(!result.success);
            assert!(result.error.as_deref().unwrap_or("").contains("Path blocked"));
        })
        .await
    }

    /// 测试阻止波浪号用户路径参数
    ///
    /// 验证 `~root/.ssh/id_rsa` 等波浪号用户路径被拦截，
    /// 防止访问其他用户的敏感文件
    #[tokio::test]
    async fn shell_blocks_tilde_user_path_argument() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
            let result = tool
                .execute(json!({"command": "cat ~root/.ssh/id_rsa"}))
                .await
                .expect("tilde-user path should be blocked");
            assert!(!result.success);
            assert!(result.error.as_deref().unwrap_or("").contains("Path blocked"));
        })
        .await
    }

    /// 测试阻止输入重定向绕过
    ///
    /// 验证通过 `</etc/passwd` 形式的输入重定向访问敏感文件被阻止
    #[tokio::test]
    async fn shell_blocks_input_redirection_path_bypass() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Supervised), test_runtime());
            let result = tool
                .execute(json!({"command": "cat </etc/passwd"}))
                .await
                .expect("input redirection bypass should be blocked");
            assert!(!result.success);
            assert!(result.error.as_deref().unwrap_or("").contains("not allowed"));
        })
        .await
    }

    /// 测试 Strip 重定向策略允许常见的 stderr 重定向
    ///
    /// 验证在 Strip 策略下：
    /// - `2>&1`（合并 stderr 到 stdout）被允许
    /// - `2>/dev/null`（丢弃 stderr）被允许
    #[tokio::test]
    async fn shell_strip_policy_allows_common_stderr_redirects() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(
                test_security_with_redirect_policy(
                    AutonomyLevel::Supervised,
                    ShellRedirectPolicy::Strip,
                ),
                test_runtime(),
            );

            // 测试 stderr 合并到 stdout
            let merged = tool
                .execute(json!({"command": "echo redirect-ok 2>&1"}))
                .await
                .expect("2>&1 should be normalized under strip policy");
            assert!(merged.success);
            assert!(merged.output.contains("redirect-ok"));

            // 测试 stderr 重定向到 /dev/null
            let devnull = tool
                .execute(json!({"command": "ls definitely_missing_shell_redirect 2>/dev/null"}))
                .await
                .expect("2>/dev/null should be normalized under strip policy");
            assert!(!devnull.success);
            assert!(
                devnull
                    .error
                    .as_deref()
                    .unwrap_or("")
                    .contains("definitely_missing_shell_redirect")
            );
        })
        .await
    }

    /// 测试 Strip 策略仍阻止不支持的输出重定向
    ///
    /// 验证即使在 Strip 策略下，文件输出重定向（如 `> out.txt`）
    /// 仍然被阻止
    #[tokio::test]
    async fn shell_strip_policy_still_blocks_unsupported_redirects() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(
                test_security_with_redirect_policy(
                    AutonomyLevel::Supervised,
                    ShellRedirectPolicy::Strip,
                ),
                test_runtime(),
            );
            let result = tool
                .execute(json!({"command": "echo blocked > out.txt"}))
                .await
                .expect("unsupported redirect should still be blocked");
            assert!(!result.success);
            assert!(result.error.as_deref().unwrap_or("").contains("not allowed"));
        })
        .await
    }

    #[tokio::test]
    async fn shell_allow_policy_executes_file_redirect() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tmp = TempDir::new().unwrap();
            let output_path = tmp.path().join("allow-output.txt");
            let security = Arc::new(SecurityPolicy {
                autonomy: AutonomyLevel::Full,
                workspace_dir: tmp.path().to_path_buf(),
                allowed_commands: vec!["printf".into()],
                forbidden_paths: Vec::new(),
                require_approval_for_medium_risk: false,
                block_high_risk_commands: false,
                allow_unsafe_shell_patterns: true,
                shell_redirect_policy: ShellRedirectPolicy::Allow,
                ..SecurityPolicy::default()
            });
            let tool = ShellTool::new(security, test_runtime());
            let command = format!("printf redirected > {}", output_path.display());

            let result = tool
                .execute(json!({"command": command}))
                .await
                .expect("allow redirect command should execute");

            assert!(result.success, "{result:?}");
            assert_eq!(std::fs::read_to_string(output_path).unwrap(), "redirected");
        })
        .await
    }

    /// 创建允许 env 和 echo 命令的测试安全策略
    fn test_security_with_env_cmd() -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: std::env::temp_dir(),
            allowed_commands: vec!["env".into(), "echo".into()],
            ..SecurityPolicy::default()
        })
    }

    /// 创建带有环境变量透传配置的测试安全策略
    ///
    /// # 参数
    /// - `vars`: 允许透传到 shell 环境的变量名列表
    fn test_security_with_env_passthrough(vars: &[&str]) -> Arc<SecurityPolicy> {
        Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Supervised,
            workspace_dir: std::env::temp_dir(),
            allowed_commands: vec!["env".into()],
            shell_env_passthrough: vars.iter().map(|v| (*v).to_string()).collect(),
            ..SecurityPolicy::default()
        })
    }

    /// 环境变量 RAII 守卫
    ///
    /// 在创建时设置环境变量，在销毁时自动恢复原始值。
    /// 即使测试发生 panic，也能确保环境变量被正确清理。
    ///
    /// # 字段说明
    /// - `key`: 环境变量名
    /// - `original`: 变量的原始值（若存在）
    struct EnvGuard {
        key: &'static str,
        original: Option<String>,
    }

    impl EnvGuard {
        /// 设置环境变量并返回守卫
        ///
        /// # 参数
        /// - `key`: 环境变量名（必须是静态生命周期）
        /// - `value`: 要设置的值
        ///
        /// # 返回
        /// 返回一个 `EnvGuard`，在其生命周期结束后会自动恢复环境变量
        ///
        /// # 安全性
        /// 在多线程环境中修改环境变量是不安全的，仅在单线程测试中使用
        fn set(key: &'static str, value: &str) -> Self {
            let original = std::env::var(key).ok();
            unsafe { std::env::set_var(key, value) };
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        /// 析构时恢复环境变量到原始状态
        fn drop(&mut self) {
            match &self.original {
                Some(val) => unsafe { std::env::set_var(self.key, val) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    /// 测试 Shell 不泄露 API 密钥
    ///
    /// 验证 `API_KEY` 和 `VIBEWINDOW_API_KEY` 等敏感环境变量
    /// 不会出现在 `env` 命令的输出中
    #[tokio::test(flavor = "current_thread")]
    async fn shell_does_not_leak_api_key() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let _lock =
                crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
            let _g1 = EnvGuard::set("API_KEY", "sk-test-secret-12345");
            let _g2 = EnvGuard::set("VIBEWINDOW_API_KEY", "sk-test-secret-67890");

            let tool = ShellTool::new(test_security_with_env_cmd(), test_runtime());
            let result = tool
                .execute(json!({"command": "env"}))
                .await
                .expect("env command execution should succeed");
            assert!(result.success);
            assert!(
                !result.output.contains("sk-test-secret-12345"),
                "API_KEY leaked to shell command output"
            );
            assert!(
                !result.output.contains("sk-test-secret-67890"),
                "VIBEWINDOW_API_KEY leaked to shell command output"
            );
        })
        .await
    }

    /// 测试 Shell 环境保留 PATH 和 HOME 变量
    ///
    /// 验证在默认配置下，`PATH` 和 `HOME` 等基本环境变量
    /// 在 shell 环境中可用
    #[tokio::test]
    async fn shell_preserves_path_and_home_for_env_command() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security_with_env_cmd(), test_runtime());

            let result =
                tool.execute(json!({"command": "env"})).await.expect("env command should succeed");
            assert!(result.success);
            assert!(
                result.output.contains("HOME="),
                "HOME should be available in shell environment"
            );
            assert!(
                result.output.contains("PATH="),
                "PATH should be available in shell environment"
            );
        })
        .await
    }

    /// 测试阻止普通变量展开
    ///
    /// 验证 `$HOME` 等变量展开语法被阻止，防止信息泄露
    #[tokio::test]
    async fn shell_blocks_plain_variable_expansion() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security_with_env_cmd(), test_runtime());
            let result = tool
                .execute(json!({"command": "echo $HOME"}))
                .await
                .expect("plain variable expansion should be blocked");
            assert!(!result.success);
            assert!(result.error.as_deref().unwrap_or("").contains("not allowed"));
        })
        .await
    }

    /// 测试配置的环境变量透传功能
    ///
    /// 验证在 `shell_env_passthrough` 中配置的变量能够传递到 shell 环境
    #[tokio::test(flavor = "current_thread")]
    async fn shell_allows_configured_env_passthrough() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let _lock =
                crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
            let _guard = EnvGuard::set("VIBEWINDOW_TEST_PASSTHROUGH", "db://unit-test");
            let tool = ShellTool::new(
                test_security_with_env_passthrough(&["VIBEWINDOW_TEST_PASSTHROUGH"]),
                test_runtime(),
            );

            let result = tool
                .execute(json!({"command": "env"}))
                .await
                .expect("env command execution should succeed");
            assert!(result.success);
            assert!(result.output.contains("VIBEWINDOW_TEST_PASSTHROUGH=db://unit-test"));
        })
        .await
    }

    /// 测试无效的环境变量名被过滤
    ///
    /// 验证 `collect_allowed_shell_env_vars` 函数过滤掉不符合
    /// 环境变量命名规则的名称（如包含连字符、以数字开头）
    #[test]
    fn invalid_shell_env_passthrough_names_are_filtered() {
        let security = SecurityPolicy {
            shell_env_passthrough: vec![
                "VALID_NAME".into(),
                "BAD-NAME".into(),
                "1NOPE".into(),
                "ALSO_VALID".into(),
            ],
            ..SecurityPolicy::default()
        };
        let vars = collect_allowed_shell_env_vars(&security);
        assert!(vars.contains(&"VALID_NAME".to_string()));
        assert!(vars.contains(&"ALSO_VALID".to_string()));
        assert!(!vars.contains(&"BAD-NAME".to_string()));
        assert!(!vars.contains(&"1NOPE".to_string()));
    }

    #[tokio::test(flavor = "current_thread")]
    async fn apply_allowed_shell_environment_injects_effective_path() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let _lock =
                crate::test_support::global_env_lock().lock().unwrap_or_else(|e| e.into_inner());
            let path_guard = EnvGuard::set("PATH", "/usr/bin:/bin");
            let expected = allowed_shell_env_value("PATH").expect("effective PATH should resolve");

            let security = test_security_with_env_cmd();
            let mut cmd = tokio::process::Command::new("env");
            apply_allowed_shell_environment(&mut cmd, &security);
            let output = cmd.output().await.expect("env command should execute");
            let stdout = String::from_utf8_lossy(&output.stdout);

            drop(path_guard);

            let path_line = stdout
                .lines()
                .find(|line| line.starts_with("PATH="))
                .expect("PATH should be present in command environment");
            assert_eq!(path_line, format!("PATH={expected}"));
        })
        .await
    }

    /// 测试中风险命令需要显式批准
    ///
    /// 验证 `touch` 命令（在 allowed_commands 中）首次执行被拒绝，
    /// 需要通过 `approved: true` 参数才能执行
    #[tokio::test]
    async fn shell_requires_approval_for_medium_risk_command() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let security = Arc::new(SecurityPolicy {
                autonomy: AutonomyLevel::Supervised,
                allowed_commands: vec!["touch".into()],
                workspace_dir: std::env::temp_dir(),
                ..SecurityPolicy::default()
            });

            let tool = ShellTool::new(security.clone(), test_runtime());

            // 未批准时应该被拒绝
            let denied = tool
                .execute(json!({"command": "touch vibewindow_shell_approval_test"}))
                .await
                .expect("unapproved command should return a result");
            assert!(!denied.success);
            assert!(denied.error.as_deref().unwrap_or("").contains("explicit approval"));

            // 批准后应该成功执行
            let allowed = tool
                .execute(json!({
                    "command": "touch vibewindow_shell_approval_test",
                    "approved": true
                }))
                .await
                .expect("approved command execution should succeed");
            assert!(allowed.success);

            // 清理测试文件
            let _ =
                tokio::fs::remove_file(std::env::temp_dir().join("vibewindow_shell_approval_test"))
                    .await;
        })
        .await
    }

    // ── §5.2 Shell 超时执行测试 ─────────────────

    /// 测试 Shell 超时常量值是否合理
    ///
    /// 验证默认等待为 10000ms，与 schema 保持一致
    #[test]
    fn shell_yield_default_is_ten_seconds() {
        assert_eq!(DEFAULT_YIELD_TIME_MS, 10_000, "shell yield must default to 10000ms");
    }

    /// 等待期限越界时明确拒绝。
    #[test]
    fn shell_yield_rejects_out_of_range_values() {
        assert!(parse_yield_time_ms(&json!({"yield_time_ms": 249})).is_err());
        assert!(parse_yield_time_ms(&json!({"yield_time_ms": 30001})).is_err());
    }

    // ── §5.3 非 UTF-8 二进制输出测试 ────────────────────

    /// 测试安全环境变量列表不包含敏感词
    ///
    /// 验证 `SAFE_ENV_VARS` 中的所有变量名都不包含
    /// "key"、"secret"、"token" 等敏感关键词
    #[test]
    fn shell_safe_env_vars_excludes_secrets() {
        for var in SAFE_ENV_VARS {
            let lower = var.to_lowercase();
            assert!(
                !lower.contains("key") && !lower.contains("secret") && !lower.contains("token"),
                "SAFE_ENV_VARS must not include sensitive variable: {var}"
            );
        }
    }

    /// 测试安全环境变量列表包含基本变量
    ///
    /// 验证 `SAFE_ENV_VARS` 包含 PATH、HOME、TERM 等基本环境变量
    #[test]
    fn shell_safe_env_vars_includes_essentials() {
        assert!(SAFE_ENV_VARS.contains(&"PATH"), "PATH must be in safe env vars");
        assert!(SAFE_ENV_VARS.contains(&"HOME"), "HOME must be in safe env vars");
        assert!(SAFE_ENV_VARS.contains(&"TERM"), "TERM must be in safe env vars");
    }

    /// 测试速率限制功能
    ///
    /// 验证当 `max_actions_per_hour` 为 0 时，所有命令都被阻止
    #[tokio::test]
    async fn shell_blocks_rate_limited() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let security = Arc::new(SecurityPolicy {
                autonomy: AutonomyLevel::Supervised,
                max_actions_per_hour: 0,
                workspace_dir: std::env::temp_dir(),
                ..SecurityPolicy::default()
            });
            let tool = ShellTool::new(security, test_runtime());
            let result = tool
                .execute(json!({"command": "echo test"}))
                .await
                .expect("rate-limited command should return a result");
            assert!(!result.success);
            assert!(result.error.as_deref().unwrap_or("").contains("Rate limit"));
        })
        .await
    }

    /// 测试处理不存在的命令
    ///
    /// 验证执行不存在的命令时返回失败结果，而非 panic
    #[tokio::test]
    async fn shell_handles_nonexistent_command() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let security = Arc::new(SecurityPolicy {
                autonomy: AutonomyLevel::Full,
                workspace_dir: std::env::temp_dir(),
                ..SecurityPolicy::default()
            });
            let tool = ShellTool::new(security, test_runtime());
            let result =
                tool.execute(json!({"command": "nonexistent_binary_xyz_12345"})).await.unwrap();
            assert!(!result.success);
        })
        .await
    }

    /// 测试捕获 stderr 输出
    ///
    /// 验证命令的 stderr 输出会合并到 output 中，并进行 CRLF 归一化
    #[tokio::test]
    async fn shell_captures_stderr_output() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tool = ShellTool::new(test_security(AutonomyLevel::Full), test_runtime());
            let result = tool
                .execute(json!({
                    "command": "printf 'out\\r\\n'; printf 'err\\r\\n' >&2",
                    "description": "emit stdout and stderr"
                }))
                .await
                .unwrap();
            assert!(result.success);
            assert!(result.output.contains("out\n"));
            assert!(result.output.contains("err\n"));
            assert!(result.error.is_none());
        })
        .await
    }

    /// 测试动作配额耗尽时的行为
    ///
    /// 验证当小时配额耗尽后，后续命令被阻止
    #[tokio::test]
    async fn shell_record_action_budget_exhaustion() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let security = Arc::new(SecurityPolicy {
                autonomy: AutonomyLevel::Full,
                max_actions_per_hour: 1,
                workspace_dir: std::env::temp_dir(),
                ..SecurityPolicy::default()
            });
            let tool = ShellTool::new(security, test_runtime());

            // 第一次执行应该成功
            let r1 = tool.execute(json!({"command": "echo first"})).await.unwrap();
            assert!(r1.success);

            // 第二次执行应该因配额耗尽而失败
            let r2 = tool.execute(json!({"command": "echo second"})).await.unwrap();
            assert!(!r2.success);
            assert!(
                r2.error.as_deref().unwrap_or("").contains("Rate limit")
                    || r2.error.as_deref().unwrap_or("").contains("budget")
            );
        })
        .await
    }

    /// 测试系统调用异常检测器写入异常日志
    ///
    /// 验证当命令触发不在基准列表中的系统调用时，
    /// 异常检测器将记录写入日志文件
    #[tokio::test]
    async fn shell_syscall_detector_writes_anomaly_log() {
        let context = Arc::new(crate::tools::context::ToolUseContext::new(
            uuid::Uuid::new_v4().to_string(),
            Some(std::env::temp_dir().to_string_lossy().to_string()),
        ));
        crate::tools::scope_tool_use_context(context, async {
            let tmp = tempfile::tempdir().expect("temp dir should be created");
            let log_path = tmp.path().join("shell-syscall-anomalies.log");
            let detector = test_syscall_detector(&tmp);
            let tool = ShellTool::new_with_syscall_detector(
                test_security(AutonomyLevel::Full),
                test_runtime(),
                Some(detector),
            );

            // 执行会触发 openat 系统调用的命令
            let result = tool
                .execute(json!({"command": "echo seccomp denied syscall=openat"}))
                .await
                .expect("command execution should return result");
            assert!(result.success);
            assert!(result.output.contains("openat"));

            // 验证异常日志被正确写入
            let log = tokio::fs::read_to_string(&log_path)
                .await
                .expect("syscall anomaly log should be written");
            assert!(log.contains("\"kind\":\"unknown_syscall\""));
            assert!(log.contains("\"syscall\":\"openat\""));
        })
        .await
    }
}
