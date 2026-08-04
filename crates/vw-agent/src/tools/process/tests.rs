//! 进程工具（ProcessTool）的单元测试模块
//!
//! 本模块包含对进程管理工具的全面测试覆盖，验证以下核心功能：
//! - 进程生命周期管理（spawn、list、output、kill）
//! - 安全策略执行（命令白名单、路径限制、权限等级）
//! - 运行时兼容性检查
//! - 输出缓冲区的边界行为
//! - 系统调用异常检测集成
//!
//! # 测试组织
//! - 辅助函数：提供测试所需的安全策略、运行时和工具实例
//! - 基础测试：验证工具元数据（名称、描述、schema）
//! - 功能测试：验证各 action 的正确行为
//! - 安全测试：验证命令/路径阻断、权限控制、速率限制
//! - 边界测试：验证输出缓冲区的截断和增量读取逻辑
//! - 集成测试：验证与系统调用检测器的协作

use super::*;

/// 测试用例模块
///
/// 所有测试封装在此模块内，允许使用 `#[allow(dead_code)]` 抑制未使用警告
#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::config::{AuditConfig, SyscallAnomalyConfig};
    use crate::app::agent::runtime::NativeRuntime;
    use crate::app::agent::security::{AutonomyLevel, SecurityPolicy, SyscallAnomalyDetector};
    use std::path::PathBuf;
    use std::time::Duration;
    use tempfile::TempDir;

    /// 创建用于测试的安全策略
    ///
    /// 返回配置为以下设置的 `SecurityPolicy`：
    /// - `Full` 自主等级（允许所有操作）
    /// - 临时目录作为工作空间
    /// - `sleep` 命令在白名单中（用于长时间运行的测试进程）
    ///
    /// # 返回值
    /// 包装在 `Arc` 中的安全策略实例，可跨多个测试共享
    fn test_security() -> Arc<SecurityPolicy> {
        let mut policy = SecurityPolicy::default();
        policy.autonomy = AutonomyLevel::Full;
        policy.workspace_dir = std::env::temp_dir();
        policy.allowed_commands.push("sleep".into());
        Arc::new(policy)
    }

    /// 创建用于测试的原生运行时适配器
    ///
    /// # 返回值
    /// 包装在 `Arc` 中的 `NativeRuntime` 实例，作为 trait 对象使用
    fn test_runtime() -> Arc<dyn RuntimeAdapter> {
        Arc::new(NativeRuntime::new())
    }

    /// 创建用于测试的系统调用异常检测器
    ///
    /// 使用指定的临时目录存储日志文件，并配置：
    /// - 基线系统调用：`read`、`write`
    /// - 日志路径：临时目录下的 `process-syscall-anomalies.log`
    /// - 告警冷却时间：1 秒
    /// - 每分钟最大告警数：50
    ///
    /// # 参数
    /// - `tmp`: 临时目录引用，用于存放日志文件
    ///
    /// # 返回值
    /// 包装在 `Arc` 中的异常检测器实例
    fn test_syscall_detector(tmp: &TempDir) -> Arc<SyscallAnomalyDetector> {
        let log_path = tmp.path().join("process-syscall-anomalies.log");
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

    /// 创建配置好测试依赖的 ProcessTool 实例
    ///
    /// 使用 `test_security()` 和 `test_runtime()` 作为默认依赖
    ///
    /// # 返回值
    /// 可立即用于测试的 `ProcessTool` 实例
    fn make_tool() -> ProcessTool {
        ProcessTool::new(test_security(), test_runtime())
    }

    async fn wait_for_process_status(
        tool: &ProcessTool,
        command: &str,
        expected_status: &str,
    ) -> serde_json::Value {
        for _ in 0..50 {
            let list_result = tool.execute(json!({"action": "list"})).await.unwrap();
            assert!(list_result.success);
            let entries: Vec<serde_json::Value> =
                serde_json::from_str(&list_result.output).unwrap();
            if let Some(entry) = entries
                .into_iter()
                .find(|entry| entry["command"].as_str() == Some(command))
                .filter(|entry| entry["status"].as_str() == Some(expected_status))
            {
                return entry;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        panic!("process {command:?} did not reach status {expected_status:?}");
    }

    async fn wait_for_output(tool: &ProcessTool, id: u64, needle: &str) -> ToolResult {
        for _ in 0..50 {
            let result = tool.execute(json!({"action": "output", "id": id})).await.unwrap();
            assert!(result.success);
            if result.output.contains(needle) {
                return result;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        panic!("process {id} output did not contain {needle:?}");
    }

    async fn wait_for_file(path: &std::path::Path) -> String {
        for _ in 0..50 {
            if let Ok(contents) = tokio::fs::read_to_string(path).await {
                return contents;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }

        panic!("file did not appear: {}", path.display());
    }

    /// 测试工具名称是否正确
    ///
    /// 验证 `ProcessTool::name()` 返回字符串 `"process"`
    #[test]
    fn process_tool_name() {
        assert_eq!(make_tool().name(), "process");
    }

    /// 测试工具描述非空
    ///
    /// 验证 `ProcessTool::description()` 返回非空字符串，确保用户可见文档存在
    #[test]
    fn process_tool_description_not_empty() {
        assert!(!make_tool().description().is_empty());
    }

    /// 测试参数 schema 包含 action 字段
    ///
    /// 验证：
    /// - schema 中存在 `action` 属性定义
    /// - `action` 被标记为必需字段
    #[test]
    fn process_tool_schema_has_action() {
        let schema = make_tool().parameters_schema();
        assert!(schema["properties"]["action"].is_object());
        assert!(schema["required"].as_array().unwrap().contains(&json!("action")));
    }

    /// 测试常量值是否符合预期
    ///
    /// 验证：
    /// - `MAX_OUTPUT_BYTES` = 524,288（512 KB）
    /// - `MAX_PROCESSES` = 8（最大并发进程数）
    #[test]
    fn constants_are_correct() {
        assert_eq!(MAX_OUTPUT_BYTES, 524_288);
        assert_eq!(MAX_PROCESSES, 8);
    }

    #[test]
    fn process_status_as_str_covers_every_state() {
        assert_eq!(ProcessStatus::Running.as_str(), "running");
        assert_eq!(ProcessStatus::Completed.as_str(), "completed");
        assert_eq!(ProcessStatus::Failed.as_str(), "failed");
        assert_eq!(ProcessStatus::Killed.as_str(), "killed");
    }

    #[test]
    fn parse_id_accepts_unsigned_ids_and_rejects_other_json_types() {
        assert_eq!(parse_id(&json!({"id": 7}), "output").unwrap(), 7);

        for args in [json!({}), json!({"id": -1}), json!({"id": "7"})] {
            let error = parse_id(&args, "output").unwrap_err();
            assert_eq!(error.to_string(), "Missing 'id' parameter for output action");
        }
    }

    /// 测试 spawn 操作启动后台进程
    ///
    /// 验证：
    /// - spawn 命令执行成功
    /// - 返回的 JSON 包含有效的 `id`（进程内部标识）
    /// - 返回的 JSON 包含有效的 `pid`（系统进程 ID）
    #[tokio::test]
    async fn spawn_starts_background_process() {
        let tool = make_tool();
        let result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo hello_process_test"
            }))
            .await
            .unwrap();
        assert!(result.success, "spawn should succeed: {:?}", result.error);
        let output: serde_json::Value = serde_json::from_str(&result.output).unwrap();
        assert!(output["id"].is_number());
        assert!(output["pid"].is_number());
    }

    /// 测试 list 操作显示已启动的进程
    ///
    /// 流程：
    /// 1. 先 spawn 一个进程
    /// 2. 执行 list 操作
    /// 3. 验证列表中包含刚启动的进程命令
    #[tokio::test]
    async fn list_shows_spawned_process() {
        let tool = make_tool();
        tool.execute(json!({
            "action": "spawn",
            "command": "echo list_test"
        }))
        .await
        .unwrap();

        let result = tool.execute(json!({"action": "list"})).await.unwrap();
        assert!(result.success);
        assert!(result.output.contains("list_test"));
    }

    /// 测试 list 操作保留已退出进程的历史记录
    ///
    /// 流程：
    /// 1. spawn 一个短生命进程（echo 命令会立即退出）
    /// 2. 等待进程退出完成
    /// 3. 执行 list 操作
    /// 4. 验证已退出的进程仍出现在列表中，且状态变为 completed
    ///
    /// 这确保了 Task* 可以读取完成后的进程任务对象
    #[tokio::test]
    async fn list_keeps_exited_process_history() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo prune_test"
            }))
            .await
            .unwrap();
        assert!(spawn_result.success);

        let entry = wait_for_process_status(&tool, "echo prune_test", "completed").await;
        assert_eq!(entry["status"], "completed");
    }

    /// 测试 output 操作返回进程的标准输出
    ///
    /// 流程：
    /// 1. spawn 一个产生输出的进程
    /// 2. 从 spawn 结果中提取进程 ID
    /// 3. 等待进程完成并捕获输出
    /// 4. 使用 output 操作获取输出内容
    /// 5. 验证输出包含预期内容
    #[tokio::test]
    async fn output_returns_stdout() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo output_capture_test"
            }))
            .await
            .unwrap();

        let spawn_output: serde_json::Value = serde_json::from_str(&spawn_result.output).unwrap();
        let id = spawn_output["id"].as_u64().unwrap();

        let result = wait_for_output(&tool, id, "output_capture_test").await;
        assert!(result.output.contains("output_capture_test"));
    }

    /// 测试 kill 操作终止进程
    ///
    /// 流程：
    /// 1. spawn 一个长时间运行的进程（sleep 60）
    /// 2. 使用 kill 操作终止该进程
    /// 3. 验证 kill 操作返回成功
    #[tokio::test]
    async fn kill_terminates_process() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "sleep 60"
            }))
            .await
            .unwrap();
        assert!(spawn_result.success);

        let spawn_output: serde_json::Value = serde_json::from_str(&spawn_result.output).unwrap();
        let id = spawn_output["id"].as_u64().unwrap();

        let kill_result = tool
            .execute(json!({
                "action": "kill",
                "id": id
            }))
            .await
            .unwrap();
        assert!(kill_result.success);
    }

    /// 测试 kill 操作保留进程条目并标记为 killed
    ///
    /// 流程：
    /// 1. spawn 一个长时间运行的进程
    /// 2. kill 该进程
    /// 3. 执行 list 操作
    /// 4. 验证被 kill 的进程仍出现在列表中，但状态为 killed
    #[tokio::test]
    async fn kill_marks_process_entry_as_killed() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "sleep 60"
            }))
            .await
            .unwrap();
        assert!(spawn_result.success);

        let spawn_output: serde_json::Value = serde_json::from_str(&spawn_result.output).unwrap();
        let id = spawn_output["id"].as_u64().unwrap();

        let kill_result = tool
            .execute(json!({
                "action": "kill",
                "id": id
            }))
            .await
            .unwrap();
        assert!(kill_result.success);

        let list_result = tool.execute(json!({"action": "list"})).await.unwrap();
        assert!(list_result.success);
        let entries: Vec<serde_json::Value> = serde_json::from_str(&list_result.output).unwrap();
        let entry = entries
            .iter()
            .find(|entry| entry["id"].as_u64() == Some(id))
            .expect("被 kill 的进程仍应保留在列表中");
        assert_eq!(entry["status"], "killed");
    }

    /// 测试进程元数据可更新并反映到快照中
    #[tokio::test]
    async fn process_metadata_update_changes_snapshot() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo metadata_test"
            }))
            .await
            .unwrap();
        assert!(spawn_result.success);
        let output: serde_json::Value = serde_json::from_str(&spawn_result.output).unwrap();
        let id = output["id"].as_u64().unwrap() as usize;

        assert!(tool.update_metadata(
            id,
            Some(Some("更清晰的标题".to_string())),
            Some(json!({ "priority": "high" })),
        ));

        let snapshot = tool.get_snapshot(id).expect("snapshot should exist");
        assert_eq!(snapshot.title.as_deref(), Some("更清晰的标题"));
        assert_eq!(snapshot.metadata["priority"], "high");
    }

    #[tokio::test]
    async fn process_metadata_update_returns_false_for_unknown_process() {
        let tool = make_tool();

        assert!(!tool.update_metadata(99, Some(Some("missing".to_string())), Some(json!({}))));
    }

    #[tokio::test]
    async fn process_metadata_update_can_clear_title_without_replacing_metadata() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo metadata_clear_test",
                "title": "original title",
                "metadata": { "priority": "high" }
            }))
            .await
            .unwrap();
        assert!(spawn_result.success);
        let output: serde_json::Value = serde_json::from_str(&spawn_result.output).unwrap();
        let id = output["id"].as_u64().unwrap() as usize;

        assert!(tool.update_metadata(id, Some(None), None));

        let snapshot = tool.get_snapshot(id).expect("snapshot should exist");
        assert_eq!(snapshot.title, None);
        assert_eq!(snapshot.metadata["priority"], "high");
    }

    #[tokio::test]
    async fn spawn_trims_non_empty_title_and_defaults_metadata() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "sleep 60",
                "title": "  background job  ",
                "approved": true
            }))
            .await
            .unwrap();
        assert!(spawn_result.success, "spawn should succeed: {:?}", spawn_result.error);
        let output: serde_json::Value = serde_json::from_str(&spawn_result.output).unwrap();
        let id = output["id"].as_u64().unwrap() as usize;

        let snapshot = tool.get_snapshot(id).expect("snapshot should exist");
        assert_eq!(snapshot.title.as_deref(), Some("background job"));
        assert_eq!(snapshot.metadata, json!({}));

        let kill_result = tool.execute(json!({"action": "kill", "id": id})).await.unwrap();
        assert!(kill_result.success);
    }

    #[tokio::test]
    async fn spawn_ignores_blank_title() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo blank_title_test",
                "title": "   "
            }))
            .await
            .unwrap();
        assert!(spawn_result.success, "spawn should succeed: {:?}", spawn_result.error);
        let output: serde_json::Value = serde_json::from_str(&spawn_result.output).unwrap();
        let id = output["id"].as_u64().unwrap() as usize;

        let snapshot = tool.get_snapshot(id).expect("snapshot should exist");
        assert_eq!(snapshot.title, None);
    }

    #[tokio::test]
    async fn spawn_missing_command_returns_error() {
        let tool = make_tool();

        let error = tool.execute(json!({"action": "spawn"})).await.unwrap_err();

        assert_eq!(error.to_string(), "Missing 'command' parameter for spawn action");
    }

    #[tokio::test]
    async fn spawn_rejects_when_max_concurrent_processes_reached() {
        let tool = make_tool();
        let mut ids = Vec::new();

        for _ in 0..MAX_PROCESSES {
            let spawn_result = tool
                .execute(json!({
                    "action": "spawn",
                    "command": "sleep 60"
                }))
                .await
                .unwrap();
            assert!(spawn_result.success, "spawn should succeed: {:?}", spawn_result.error);
            let output: serde_json::Value = serde_json::from_str(&spawn_result.output).unwrap();
            ids.push(output["id"].as_u64().unwrap());
        }

        let rejected = tool
            .execute(json!({
                "action": "spawn",
                "command": "sleep 60"
            }))
            .await
            .unwrap();
        assert!(!rejected.success);
        assert!(
            rejected.error.as_deref().unwrap_or_default().contains("Maximum concurrent processes")
        );

        for id in ids {
            let _ = tool.execute(json!({"action": "kill", "id": id})).await;
        }
    }

    #[tokio::test]
    async fn refresh_processes_marks_failed_processes() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "grep"
            }))
            .await
            .unwrap();
        assert!(spawn_result.success, "spawn should succeed: {:?}", spawn_result.error);

        let entry = wait_for_process_status(&tool, "grep", "failed").await;

        assert_eq!(entry["status"], "failed");
        assert!(entry["exit_code"].as_i64().unwrap_or_default() != 0);
    }

    #[tokio::test]
    async fn refresh_processes_removes_stale_completed_entries() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo stale_process_test"
            }))
            .await
            .unwrap();
        assert!(spawn_result.success);
        let output: serde_json::Value = serde_json::from_str(&spawn_result.output).unwrap();
        let id = output["id"].as_u64().unwrap() as usize;

        let _ = wait_for_process_status(&tool, "echo stale_process_test", "completed").await;
        {
            let mut processes = tool.processes.write().unwrap();
            let entry = processes.get_mut(&id).expect("process entry should exist");
            entry.completed_at =
                Some(Utc::now() - chrono::Duration::seconds(PROCESS_MAX_AGE_SECS + 1));
        }

        assert!(tool.list_snapshots().into_iter().all(|entry| entry.id != id));
    }

    /// 测试未知 action 返回错误
    ///
    /// 验证当传入不支持的 action 时，工具返回失败状态并包含明确的错误信息
    #[tokio::test]
    async fn unknown_action_returns_error() {
        let tool = make_tool();
        let result = tool.execute(json!({"action": "restart"})).await.unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("Unknown action"));
    }

    /// 测试 spawn 阻断禁止的命令
    ///
    /// 验证即使自主等级为 Full，危险命令（如 `rm -rf /`）仍会被安全策略阻断
    #[tokio::test]
    async fn spawn_blocks_disallowed_command() {
        let tool = make_tool();
        let result = tool
            .execute(json!({
                "action": "spawn",
                "command": "rm -rf /"
            }))
            .await
            .unwrap();
        assert!(!result.success);
    }

    /// 测试 spawn 阻断禁止的路径
    ///
    /// 验证尝试读取敏感系统文件（如 `/etc/passwd`）会被路径限制阻断
    #[tokio::test]
    async fn spawn_blocks_forbidden_path() {
        let tool = make_tool();
        let result = tool
            .execute(json!({
                "action": "spawn",
                "command": "cat /etc/passwd"
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("Path blocked"));
    }

    /// 测试 kill 操作在只读模式下被阻断
    ///
    /// 验证当安全策略的自主等级为 `ReadOnly` 时，kill 操作被拒绝
    #[tokio::test]
    async fn kill_blocks_readonly() {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::ReadOnly,
            workspace_dir: std::env::temp_dir(),
            ..SecurityPolicy::default()
        });
        let tool = ProcessTool::new(security, test_runtime());
        let result = tool
            .execute(json!({
                "action": "kill",
                "id": 0
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("read-only"));
    }

    /// 测试 output 操作缺少 ID 参数时返回错误
    ///
    /// 验证当 output 请求未提供必需的 `id` 字段时，返回解析错误
    #[tokio::test]
    async fn output_missing_id_returns_error() {
        let tool = make_tool();
        let result = tool.execute(json!({"action": "output"})).await;
        assert!(result.is_err());
    }

    /// 测试 output 操作查询不存在的进程 ID 时返回错误
    ///
    /// 验证当请求一个不存在的进程 ID 时，返回失败状态并包含明确的错误信息
    #[tokio::test]
    async fn output_nonexistent_id_returns_error() {
        let tool = make_tool();
        let result = tool
            .execute(json!({
                "action": "output",
                "id": 9999
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("No process"));
    }

    /// 测试 spawn 操作在速率限制下被阻断
    ///
    /// 验证当安全策略设置 `max_actions_per_hour = 0` 时，任何 spawn 操作都会被速率限制拒绝
    #[tokio::test]
    async fn spawn_blocks_rate_limited() {
        let security = Arc::new(SecurityPolicy {
            autonomy: AutonomyLevel::Full,
            max_actions_per_hour: 0,
            workspace_dir: std::env::temp_dir(),
            ..SecurityPolicy::default()
        });
        let tool = ProcessTool::new(security, test_runtime());
        let result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo test"
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("Rate limit"));
    }

    /// 模拟不支持长运行进程的运行时适配器
    ///
    /// 用于测试当运行时不支持长运行进程时，spawn 操作应被拒绝
    ///
    /// # 实现细节
    /// - `supports_long_running()` 返回 `false`
    /// - 其他方法保持与真实运行时相似的行为
    struct NoLongRunningRuntime;

    impl RuntimeAdapter for NoLongRunningRuntime {
        /// 返回 self 的 Any 引用，用于 trait 对象类型检查
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        /// 返回运行时名称标识
        fn name(&self) -> &str {
            "test-restricted"
        }

        /// 表示此运行时具有 shell 访问能力
        fn has_shell_access(&self) -> bool {
            true
        }

        /// 表示此运行时具有文件系统访问能力
        fn has_filesystem_access(&self) -> bool {
            true
        }

        /// 返回存储路径
        fn storage_path(&self) -> PathBuf {
            PathBuf::from("/tmp")
        }

        /// 关键：表示此运行时不支持长运行进程
        fn supports_long_running(&self) -> bool {
            false
        }

        /// 构建可执行的 shell 命令
        ///
        /// # 参数
        /// - `command`: 要执行的 shell 命令字符串
        /// - `workspace_dir`: 命令执行的工作目录
        ///
        /// # 返回值
        /// 配置好的 `tokio::process::Command` 实例
        fn build_shell_command(
            &self,
            command: &str,
            workspace_dir: &std::path::Path,
        ) -> anyhow::Result<tokio::process::Command> {
            let mut cmd = tokio::process::Command::new("sh");
            cmd.arg("-c").arg(command).current_dir(workspace_dir);
            Ok(cmd)
        }
    }

    /// 测试当运行时不支持长运行进程时，spawn 操作被拒绝
    ///
    /// 验证使用 `NoLongRunningRuntime` 时，spawn 请求会失败并返回明确的错误信息
    #[tokio::test]
    async fn spawn_rejects_when_runtime_unsupported() {
        let tool = ProcessTool::new(test_security(), Arc::new(NoLongRunningRuntime));
        let result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo test"
            }))
            .await
            .unwrap();
        assert!(!result.success);
        assert!(result.error.as_deref().unwrap().contains("long-running"));
    }

    struct FailingBuildRuntime;

    impl RuntimeAdapter for FailingBuildRuntime {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn name(&self) -> &str {
            "test-failing-build"
        }

        fn has_shell_access(&self) -> bool {
            true
        }

        fn has_filesystem_access(&self) -> bool {
            true
        }

        fn storage_path(&self) -> PathBuf {
            PathBuf::from("/tmp")
        }

        fn supports_long_running(&self) -> bool {
            true
        }

        fn build_shell_command(
            &self,
            _command: &str,
            _workspace_dir: &std::path::Path,
        ) -> anyhow::Result<tokio::process::Command> {
            Err(anyhow::anyhow!("runtime unavailable"))
        }
    }

    struct MissingExecutableRuntime;

    impl RuntimeAdapter for MissingExecutableRuntime {
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }

        fn name(&self) -> &str {
            "test-missing-executable"
        }

        fn has_shell_access(&self) -> bool {
            true
        }

        fn has_filesystem_access(&self) -> bool {
            true
        }

        fn storage_path(&self) -> PathBuf {
            PathBuf::from("/tmp")
        }

        fn supports_long_running(&self) -> bool {
            true
        }

        fn build_shell_command(
            &self,
            _command: &str,
            workspace_dir: &std::path::Path,
        ) -> anyhow::Result<tokio::process::Command> {
            let mut cmd = tokio::process::Command::new("__vw_process_missing_binary__");
            cmd.current_dir(workspace_dir);
            Ok(cmd)
        }
    }

    #[tokio::test]
    async fn spawn_reports_runtime_command_build_errors() {
        let tool = ProcessTool::new(test_security(), Arc::new(FailingBuildRuntime));
        let result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo test"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(
            result.error.as_deref().unwrap_or_default().contains("Failed to build runtime command")
        );
    }

    #[tokio::test]
    async fn spawn_reports_process_spawn_errors() {
        let tool = ProcessTool::new(test_security(), Arc::new(MissingExecutableRuntime));
        let result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo test"
            }))
            .await
            .unwrap();

        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or_default().contains("Failed to spawn process"));
    }

    /// 测试 `append_bounded` 函数在缓冲区溢出时截断旧数据
    ///
    /// 流程：
    /// 1. 创建一个超过 `MAX_OUTPUT_BYTES` 限制的数据字符串
    /// 2. 调用 `append_bounded` 将数据写入缓冲区
    /// 3. 验证缓冲区大小不超过 `MAX_OUTPUT_BYTES`
    /// 4. 验证 `dropped_prefix_bytes` 记录了被丢弃的字节数
    ///
    /// 这确保了输出缓冲区有界，防止内存无限增长
    #[test]
    fn append_bounded_truncates_old_data() {
        let buf = Mutex::new(OutputBuffer::default());
        // 创建超过限制的数据
        let data = "x".repeat(MAX_OUTPUT_BYTES + 100);
        append_bounded(&buf, &data);
        let guard = buf.lock().unwrap();
        // 验证缓冲区大小被限制
        assert!(guard.data.len() <= MAX_OUTPUT_BYTES);
        // 验证丢弃的字节数被正确记录
        assert!(guard.dropped_prefix_bytes >= 100);
    }

    #[test]
    fn append_bounded_preserves_utf8_when_cutoff_splits_a_character() {
        let buf = Mutex::new(OutputBuffer::default());
        let data = format!("é{}", "x".repeat(MAX_OUTPUT_BYTES - 1));

        append_bounded(&buf, &data);

        let guard = buf.lock().unwrap();
        assert_eq!(guard.data.len(), MAX_OUTPUT_BYTES - 1);
        assert!(guard.data.chars().all(|character| character == 'x'));
        assert_eq!(guard.dropped_prefix_bytes, 2);
    }

    /// 测试 `slice_unseen_output` 在缓冲区滚动后正确追踪未读数据
    ///
    /// 场景：当缓冲区发生滚动（丢弃前缀数据）时，需要正确计算增量输出
    ///
    /// 流程：
    /// 1. 设置 `analyzed` 为缓冲区最大大小（表示之前已分析完所有数据）
    /// 2. 创建新的当前缓冲区内容，末尾包含 "tail"
    /// 3. 设置 `dropped` 为已丢弃的前缀字节数
    /// 4. 调用 `slice_unseen_output` 获取未读数据
    /// 5. 验证返回的增量仅包含 "tail"
    /// 6. 验证 `analyzed` 偏移量被正确更新
    ///
    /// 这确保了增量读取在缓冲区滚动场景下仍能正确工作
    #[test]
    fn slice_unseen_output_tracks_new_tail_after_rollover() {
        // 初始化为已分析完整个缓冲区
        let mut analyzed = u64::try_from(MAX_OUTPUT_BYTES).expect("size should fit in u64");
        // 创建包含 "tail" 末尾的缓冲区内容
        let current = format!("{}tail", "x".repeat(MAX_OUTPUT_BYTES.saturating_sub(4)));
        let dropped = 4_u64;

        // 获取未读的增量数据
        let delta = slice_unseen_output(&current, dropped, &mut analyzed);

        // 验证增量仅包含新增的 "tail" 部分
        assert_eq!(delta, "tail");
        // 验证 analyzed 偏移量被更新到新位置
        assert_eq!(
            analyzed,
            dropped.saturating_add(u64::try_from(current.len()).expect("len should fit in u64"))
        );
    }

    #[test]
    fn slice_unseen_output_restarts_when_analyzed_offset_was_dropped() {
        let mut analyzed = 2;

        let delta = slice_unseen_output("tail", 5, &mut analyzed);

        assert_eq!(delta, "tail");
        assert_eq!(analyzed, 9);
    }

    #[test]
    fn slice_unseen_output_advances_to_next_utf8_boundary() {
        let mut analyzed = 1;

        let delta = slice_unseen_output("étail", 0, &mut analyzed);

        assert_eq!(delta, "tail");
        assert_eq!(analyzed, 6);
    }

    #[tokio::test]
    async fn output_waits_for_running_process_without_output() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "sleep 1"
            }))
            .await
            .unwrap();
        assert!(spawn_result.success);
        let output: serde_json::Value = serde_json::from_str(&spawn_result.output).unwrap();
        let id = output["id"].as_u64().unwrap();

        let result = tool.execute(json!({"action": "output", "id": id})).await.unwrap();

        assert!(result.success);
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&result.output).unwrap(),
            json!({
                "stdout": "",
                "stderr": ""
            })
        );
        let _ = tool.execute(json!({"action": "kill", "id": id})).await;
    }

    #[tokio::test]
    async fn kill_missing_id_returns_parse_error() {
        let tool = make_tool();

        let error = tool.execute(json!({"action": "kill"})).await.unwrap_err();

        assert_eq!(error.to_string(), "Missing 'id' parameter for kill action");
    }

    #[tokio::test]
    async fn kill_nonexistent_id_returns_error_result() {
        let tool = make_tool();

        let result = tool.execute(json!({"action": "kill", "id": 9999})).await.unwrap();

        assert!(!result.success);
        assert!(result.error.as_deref().unwrap_or_default().contains("No process with id 9999"));
    }

    #[tokio::test]
    async fn kill_completed_process_reports_already_done() {
        let tool = make_tool();
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo kill_completed_test"
            }))
            .await
            .unwrap();
        assert!(spawn_result.success);
        let output: serde_json::Value = serde_json::from_str(&spawn_result.output).unwrap();
        let id = output["id"].as_u64().unwrap();

        let _ = wait_for_process_status(&tool, "echo kill_completed_test", "completed").await;

        let kill_result = tool.execute(json!({"action": "kill", "id": id})).await.unwrap();

        assert!(kill_result.success);
        assert!(kill_result.output.contains("already completed"));
    }

    /// 测试进程输出集成系统调用异常检测器的增量分析
    ///
    /// 验证点：
    /// 1. 进程输出中的系统调用异常能被检测并记录
    /// 2. 增量偏移量机制能防止对同一输出的重复检测
    ///
    /// 流程：
    /// 1. 创建带系统调用检测器的 ProcessTool
    /// 2. spawn 一个输出包含 "seccomp denied syscall=openat" 的进程
    /// 3. 等待进程完成
    /// 4. 第一次调用 output：验证异常被检测并记录到日志
    /// 5. 第二次调用 output：验证没有产生重复的日志条目
    ///
    /// 这确保了系统调用检测的增量性，避免对已分析数据的重复处理
    #[tokio::test]
    async fn process_output_runs_syscall_detector_incrementally() {
        // 创建临时目录存放检测器日志
        let tmp = tempfile::tempdir().expect("temp dir should be created");
        let log_path = tmp.path().join("process-syscall-anomalies.log");

        // 创建带系统调用检测器的工具实例
        let tool = ProcessTool::new_with_syscall_detector(
            test_security(),
            test_runtime(),
            Some(test_syscall_detector(&tmp)),
        );

        // 启动一个会输出系统调用拒绝信息的进程
        let spawn_result = tool
            .execute(json!({
                "action": "spawn",
                "command": "echo seccomp denied syscall=openat"
            }))
            .await
            .expect("spawn should return result");
        assert!(spawn_result.success);

        // 提取进程 ID
        let spawn_output: serde_json::Value =
            serde_json::from_str(&spawn_result.output).expect("spawn output should be json");
        let id = spawn_output["id"].as_u64().expect("process id should exist");

        // 第一次获取输出：应触发异常检测
        let first_output = wait_for_output(&tool, id, "seccomp denied syscall=openat").await;
        assert!(first_output.success);

        // 验证异常日志被创建并包含未知系统调用记录
        let first_log = wait_for_file(&log_path).await;
        let first_lines = first_log.lines().count();
        assert!(first_lines >= 1);
        assert!(first_log.contains("\"kind\":\"unknown_syscall\""));

        // 第二次获取输出：由于增量机制，不应产生重复日志
        let second_output = tool
            .execute(json!({"action": "output", "id": id}))
            .await
            .expect("second output should return result");
        assert!(second_output.success);

        // 验证日志行数未增加（增量偏移量防止了重复检测）
        let second_log = tokio::fs::read_to_string(&log_path)
            .await
            .expect("second anomaly log should still exist");
        let second_lines = second_log.lines().count();
        assert_eq!(second_lines, first_lines, "增量偏移量应防止对未变更输出的重复检测");
    }
}
