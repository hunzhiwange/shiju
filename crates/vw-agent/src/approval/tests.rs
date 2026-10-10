//! 审批管理器测试模块
//!
//! 本模块包含 `ApprovalManager` 及其相关类型的全面单元测试。
//! 测试覆盖以下核心功能：
//!
//! - **审批决策逻辑**：验证不同自主级别下工具是否需要审批
//! - **会话允许列表**：测试 CLI 渠道的会话级审批缓存
//! - **非 CLI 会话审批**：测试 Telegram/Discord 等渠道的待审批请求管理
//! - **运行时策略更新**：验证运行时动态修改审批策略的行为
//! - **审计日志**：确保审批决策被正确记录
//! - **参数摘要**：测试工具参数的友好格式化输出
//!
//! # 测试组织结构
//!
//! 测试按功能域分组，每组通过注释分隔标记：
//! - `needs_approval` - 审批决策判断测试
//! - `session allowlist` - CLI 会话允许列表测试
//! - `audit log` - 审计日志测试
//! - `summarize_args` - 参数摘要测试
//!
//! # 配置预设
//!
//! 测试使用两个主要配置预设：
//! - `supervised_config()` - 监督模式，部分工具自动批准
//! - `full_config()` - 完全自主模式，所有工具无需批准

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use crate::app::agent::config::AutonomyConfig;
    use crate::app::agent::config::NonCliNaturalLanguageApprovalMode;
    use crate::app::agent::security::AutonomyLevel;
    use chrono::{Duration, Utc};
    use std::collections::HashMap;

    /// 创建监督模式配置
    ///
    /// 返回一个预设的 `AutonomyConfig`，用于测试监督模式下的审批行为：
    /// - `level` 设置为 `Supervised`（监督模式）
    /// - `auto_approve` 包含 `file_read` 和 `memory_recall`（自动批准）
    /// - `always_ask` 包含 `shell`（始终询问）
    ///
    /// # 返回值
    ///
    /// 配置好的 `AutonomyConfig` 实例
    fn supervised_config() -> AutonomyConfig {
        AutonomyConfig {
            level: AutonomyLevel::Supervised,
            auto_approve: vec!["file_read".into(), "memory_recall".into()],
            always_ask: vec!["shell".into()],
            ..AutonomyConfig::default()
        }
    }

    /// 创建完全自主模式配置
    ///
    /// 返回一个 `level` 设置为 `Full` 的配置，
    /// 在此模式下所有工具都不需要审批。
    ///
    /// # 返回值
    ///
    /// 配置为完全自主模式的 `AutonomyConfig` 实例
    fn full_config() -> AutonomyConfig {
        AutonomyConfig { level: AutonomyLevel::Full, ..AutonomyConfig::default() }
    }

    // ═══════════════════════════════════════════════════════════
    // needs_approval 审批决策判断测试
    // ═══════════════════════════════════════════════════════════

    /// 测试 auto_approve 列表中的工具跳过审批提示
    ///
    /// 验证在监督模式下，配置在 `auto_approve` 列表中的工具
    ///（`file_read`、`memory_recall`）调用 `needs_approval` 返回 `false`。
    #[test]
    fn auto_approve_tools_skip_prompt() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // file_read 和 memory_recall 在 auto_approve 列表中，无需审批
        assert!(!mgr.needs_approval("file_read"));
        assert!(!mgr.needs_approval("memory_recall"));
    }

    /// 测试 always_ask 列表中的工具始终需要审批
    ///
    /// 验证在监督模式下，配置在 `always_ask` 列表中的工具
    ///（`shell`）调用 `needs_approval` 返回 `true`。
    #[test]
    fn always_ask_tools_always_prompt() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // shell 在 always_ask 列表中，始终需要审批
        assert!(mgr.needs_approval("shell"));
    }

    /// 测试监督模式下未知工具需要审批
    ///
    /// 验证在监督模式下，既不在 `auto_approve` 也不在 `always_ask` 列表中
    /// 的未知工具（如 `file_write`、`http_request`）默认需要审批。
    #[test]
    fn unknown_tool_needs_approval_in_supervised() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // file_write 和 http_request 不在任何列表中，监督模式下需要审批
        assert!(mgr.needs_approval("file_write"));
        assert!(mgr.needs_approval("http_request"));
    }

    /// 测试完全自主模式下从不触发审批提示
    ///
    /// 验证在 `Full` 自主级别下，即使是高风险工具（`shell`、`file_write`）
    /// 或任意工具都无需审批。
    #[test]
    fn full_autonomy_never_prompts() {
        let mgr = ApprovalManager::from_config(&full_config());
        // 完全自主模式下，所有工具都无需审批
        assert!(!mgr.needs_approval("shell"));
        assert!(!mgr.needs_approval("file_write"));
        assert!(!mgr.needs_approval("anything"));
    }

    /// 测试只读模式从不触发审批
    ///
    /// 验证在 `ReadOnly` 级别下，即使请求执行 shell 命令
    /// 也返回不需要审批（因为只读模式会阻止执行而非请求审批）。
    #[test]
    fn readonly_never_prompts() {
        let config = AutonomyConfig { level: AutonomyLevel::ReadOnly, ..AutonomyConfig::default() };
        let mgr = ApprovalManager::from_config(&config);
        // 只读模式下不触发审批流程（而是直接拒绝执行）
        assert!(!mgr.needs_approval("shell"));
    }

    // ═══════════════════════════════════════════════════════════
    // session allowlist CLI 会话允许列表测试
    // ═══════════════════════════════════════════════════════════

    /// 测试 "Always" 响应将工具添加到会话允许列表
    ///
    /// 验证当用户选择 "Always" 批准时，该工具会被添加到会话级允许列表，
    /// 后续对同一工具的调用将自动批准，无需再次提示。
    #[test]
    fn always_response_adds_to_session_allowlist() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 初始状态：file_write 需要审批
        assert!(mgr.needs_approval("file_write"));

        // 用户选择 "Always" 批准 file_write
        mgr.record_decision(
            "file_write",
            &serde_json::json!({"path": "test.txt"}),
            ApprovalResponse::Always,
            "cli",
        );

        // 现在 file_write 已在会话允许列表中，无需再次审批
        assert!(!mgr.needs_approval("file_write"));
    }

    /// 测试 always_ask 配置覆盖会话允许列表
    ///
    /// 验证即使工具被 "Always" 批准加入会话允许列表，
    /// 如果该工具同时配置在 `always_ask` 列表中，
    /// 仍然需要每次审批（`always_ask` 优先级更高）。
    #[test]
    fn always_ask_overrides_session_allowlist() {
        let mgr = ApprovalManager::from_config(&supervised_config());

        // 即使对 shell 选择 "Always"，仍应提示审批
        mgr.record_decision(
            "shell",
            &serde_json::json!({"command": "ls"}),
            ApprovalResponse::Always,
            "cli",
        );

        // shell 在 always_ask 列表中，因此仍需审批
        assert!(mgr.needs_approval("shell"));
    }

    /// 测试 "Yes" 响应不会将工具添加到允许列表
    ///
    /// 验证当用户选择单次批准（"Yes"）时，该工具仅对当前调用有效，
    /// 不会被添加到会话允许列表，后续调用仍需审批。
    #[test]
    fn yes_response_does_not_add_to_allowlist() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 用户选择单次批准（Yes），而非 Always
        mgr.record_decision("file_write", &serde_json::json!({}), ApprovalResponse::Yes, "cli");
        // file_write 仍需审批，因为 Yes 不加入允许列表
        assert!(mgr.needs_approval("file_write"));
    }

    /// 测试非 CLI 会话授权在多次检查间持久化
    ///
    /// 验证通过 `grant_non_cli_session` 授予的会话级权限
    /// 在多次调用 `is_non_cli_session_granted` 时保持一致。
    #[test]
    fn non_cli_session_approval_persists_across_checks() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 初始状态：shell 未被授予非 CLI 会话权限
        assert!(!mgr.is_non_cli_session_granted("shell"));

        // 授予非 CLI 会话权限
        mgr.grant_non_cli_session("shell");
        // 权限应持久存在
        assert!(mgr.is_non_cli_session_granted("shell"));
        assert!(mgr.is_non_cli_session_granted("shell"));
    }

    /// 测试非 CLI 会话授权可以被撤销
    ///
    /// 验证 `revoke_non_cli_session` 能够正确撤销已授予的权限，
    /// 且对已撤销的权限再次撤销返回 `false`。
    #[test]
    fn non_cli_session_approval_can_be_revoked() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 授予权限
        mgr.grant_non_cli_session("shell");
        assert!(mgr.is_non_cli_session_granted("shell"));

        // 撤销权限，应返回 true 表示撤销成功
        assert!(mgr.revoke_non_cli_session("shell"));
        // 权限已被撤销
        assert!(!mgr.is_non_cli_session_granted("shell"));
        // 再次撤销返回 false，因为权限已不存在
        assert!(!mgr.revoke_non_cli_session("shell"));
    }

    /// 测试非 CLI 会话允许列表快照能列出已授权工具
    ///
    /// 验证 `non_cli_session_allowlist` 返回当前会话中
    /// 所有被授予非 CLI 会话权限的工具列表。
    #[test]
    fn non_cli_session_allowlist_snapshot_lists_granted_tools() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 授予两个工具的非 CLI 会话权限
        mgr.grant_non_cli_session("shell");
        mgr.grant_non_cli_session("file_write");

        // 获取允许列表快照
        let allowlist = mgr.non_cli_session_allowlist();
        assert!(allowlist.contains("shell"));
        assert!(allowlist.contains("file_write"));
    }

    /// 测试非 CLI "全部允许一次"令牌的计数和消费
    ///
    /// 验证 `grant_non_cli_allow_all_once` 和 `consume_non_cli_allow_all_once`
    /// 的令牌计数行为：
    /// - 授予令牌增加剩余计数
    /// - 消费令牌减少计数并返回是否成功
    /// - 计数归零后消费失败
    #[test]
    fn non_cli_allow_all_once_tokens_are_counted_and_consumed() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 初始状态：无令牌
        assert_eq!(mgr.non_cli_allow_all_once_remaining(), 0);
        assert!(!mgr.consume_non_cli_allow_all_once());

        // 授予令牌，返回当前剩余数
        assert_eq!(mgr.grant_non_cli_allow_all_once(), 1);
        assert_eq!(mgr.grant_non_cli_allow_all_once(), 2);
        assert_eq!(mgr.non_cli_allow_all_once_remaining(), 2);

        // 消费令牌
        assert!(mgr.consume_non_cli_allow_all_once());
        assert_eq!(mgr.non_cli_allow_all_once_remaining(), 1);
        assert!(mgr.consume_non_cli_allow_all_once());
        assert_eq!(mgr.non_cli_allow_all_once_remaining(), 0);
        // 无令牌时消费失败
        assert!(!mgr.consume_non_cli_allow_all_once());
    }

    /// 测试持久运行时授权立即更新策略
    ///
    /// 验证 `apply_persistent_runtime_grant` 能够立即将工具
    /// 添加到 `auto_approve` 列表并从 `always_ask` 列表中移除。
    #[test]
    fn persistent_runtime_grant_updates_policy_immediately() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 初始状态：shell 在 always_ask 中，需要审批
        assert!(mgr.needs_approval("shell"));

        // 应用持久运行时授权
        mgr.apply_persistent_runtime_grant("shell");
        // shell 现在无需审批
        assert!(!mgr.needs_approval("shell"));
        // shell 已加入 auto_approve 列表
        assert!(mgr.auto_approve_tools().contains("shell"));
        // shell 已从 always_ask 列表移除
        assert!(!mgr.always_ask_tools().contains("shell"));
    }

    /// 测试持久运行时撤销立即更新策略
    ///
    /// 验证 `apply_persistent_runtime_revoke` 能够立即将工具
    /// 从 `auto_approve` 列表中移除，使其需要审批。
    /// 再次撤销已撤销的工具返回 `false`。
    #[test]
    fn persistent_runtime_revoke_updates_policy_immediately() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 初始状态：file_read 在 auto_approve 中，无需审批
        assert!(!mgr.needs_approval("file_read"));

        // 应用持久运行时撤销，返回 true 表示撤销成功
        assert!(mgr.apply_persistent_runtime_revoke("file_read"));
        // file_read 现在需要审批
        assert!(mgr.needs_approval("file_read"));
        // 再次撤销返回 false
        assert!(!mgr.apply_persistent_runtime_revoke("file_read"));
    }

    /// 测试创建和确认待审批的非 CLI 请求
    ///
    /// 验证完整的待审批请求生命周期：
    /// 1. `create_non_cli_pending_request` 创建请求并生成唯一 ID
    /// 2. `confirm_non_cli_pending_request` 确认请求
    /// 3. 已确认的请求无法再次确认
    #[test]
    fn create_and_confirm_pending_non_cli_approval_request() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 创建待审批请求
        let req = mgr.create_non_cli_pending_request(
            "shell",
            "alice",
            "telegram",
            "chat-1",
            None,
            serde_json::json!({"command": "pwd"}),
            Some("msg-1".to_string()),
            Some("call-1".to_string()),
        );
        assert_eq!(req.tool_name, "shell");
        assert_eq!(req.arguments, serde_json::json!({"command": "pwd"}));
        assert_eq!(req.message_id.as_deref(), Some("msg-1"));
        assert_eq!(req.call_id.as_deref(), Some("call-1"));
        // 请求 ID 应以 "apr-" 前缀开头
        assert!(req.request_id.starts_with("apr-"));

        // 确认待审批请求
        let confirmed = mgr
            .confirm_non_cli_pending_request(&req.request_id, "alice", "telegram", "chat-1")
            .expect("request should confirm");
        assert_eq!(confirmed.request_id, req.request_id);
        // 已确认的请求无法再次确认
        assert!(
            mgr.confirm_non_cli_pending_request(&req.request_id, "alice", "telegram", "chat-1")
                .is_err()
        );
    }

    /// 测试创建和拒绝待审批的非 CLI 请求
    ///
    /// 验证 `reject_non_cli_pending_request` 能够正确拒绝请求，
    /// 拒绝后请求不再存在于待审批列表中。
    #[test]
    fn create_and_reject_pending_non_cli_approval_request() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 创建待审批请求
        let req = mgr.create_non_cli_pending_request(
            "shell",
            "alice",
            "telegram",
            "chat-1",
            None,
            serde_json::json!({"command": "pwd"}),
            None,
            None,
        );

        // 拒绝待审批请求
        let rejected = mgr
            .reject_non_cli_pending_request(&req.request_id, "alice", "telegram", "chat-1")
            .expect("request should reject");
        assert_eq!(rejected.request_id, req.request_id);
        // 拒绝后请求不再存在
        assert!(!mgr.has_non_cli_pending_request(&req.request_id));
    }

    /// 测试待审批请求的解决结果被记录和消费
    ///
    /// 验证 `record_non_cli_pending_resolution` 记录解决结果，
    /// `take_non_cli_pending_resolution` 消费结果（消费后结果被清除）。
    #[test]
    fn pending_non_cli_resolution_is_recorded_and_consumed() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        let req = mgr.create_non_cli_pending_request(
            "shell",
            "alice",
            "telegram",
            "chat-1",
            None,
            serde_json::json!({"command": "pwd"}),
            None,
            None,
        );

        // 记录解决结果
        mgr.record_non_cli_pending_resolution(&req.request_id, ApprovalResponse::Yes);
        // 消费解决结果
        assert_eq!(
            mgr.take_non_cli_pending_resolution(&req.request_id),
            Some(ApprovalResponse::Yes)
        );
        // 消费后结果被清除
        assert_eq!(mgr.take_non_cli_pending_resolution(&req.request_id), None);
    }

    /// 测试待审批请求确认需要相同的发送者和渠道
    ///
    /// 验证确认/拒绝待审批请求时，必须使用与创建请求时
    /// 相同的发送者（sender）、渠道（channel）和回复目标（reply_target），
    /// 否则返回 `PendingApprovalError::RequesterMismatch` 错误。
    /// 不匹配的确认不会移除待审批请求。
    #[test]
    fn pending_non_cli_approval_requires_same_sender_and_channel() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        let req = mgr.create_non_cli_pending_request(
            "shell",
            "alice",
            "telegram",
            "chat-1",
            None,
            serde_json::json!({"command": "pwd"}),
            None,
            None,
        );

        // 发送者不匹配应失败
        let err = mgr
            .confirm_non_cli_pending_request(&req.request_id, "bob", "telegram", "chat-1")
            .expect_err("mismatched sender should fail");
        assert_eq!(err, PendingApprovalError::RequesterMismatch);

        // 不匹配后请求仍处于待审批状态
        let pending =
            mgr.list_non_cli_pending_requests(Some("alice"), Some("telegram"), Some("chat-1"));
        assert_eq!(pending.len(), 1);

        // 渠道不匹配应失败
        let err = mgr
            .confirm_non_cli_pending_request(&req.request_id, "alice", "discord", "chat-1")
            .expect_err("mismatched channel should fail");
        assert_eq!(err, PendingApprovalError::RequesterMismatch);

        // 回复目标不匹配应失败
        let err = mgr
            .confirm_non_cli_pending_request(&req.request_id, "alice", "telegram", "chat-2")
            .expect_err("mismatched reply target should fail");
        assert_eq!(err, PendingApprovalError::RequesterMismatch);
    }

    /// 测试列出待审批请求支持作用域过滤
    ///
    /// 验证 `list_non_cli_pending_requests` 支持按发送者、渠道、
    /// 回复目标进行过滤，返回匹配的待审批请求列表。
    #[test]
    fn list_pending_non_cli_approvals_filters_scope() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 创建多个不同作用域的待审批请求
        mgr.create_non_cli_pending_request(
            "shell",
            "alice",
            "telegram",
            "chat-1",
            None,
            serde_json::json!({"command": "pwd"}),
            None,
            None,
        );
        mgr.create_non_cli_pending_request(
            "file_write",
            "bob",
            "telegram",
            "chat-1",
            None,
            serde_json::json!({"filePath": "notes.txt"}),
            None,
            None,
        );
        mgr.create_non_cli_pending_request(
            "browser_open",
            "alice",
            "discord",
            "chat-9",
            None,
            serde_json::json!({"url": "https://example.com"}),
            None,
            None,
        );
        mgr.create_non_cli_pending_request(
            "schedule",
            "alice",
            "telegram",
            "chat-2",
            None,
            serde_json::json!({"cron": "0 * * * *"}),
            None,
            None,
        );

        // 按 alice + telegram + chat-1 过滤
        let alice_telegram =
            mgr.list_non_cli_pending_requests(Some("alice"), Some("telegram"), Some("chat-1"));
        assert_eq!(alice_telegram.len(), 1);
        assert_eq!(alice_telegram[0].tool_name, "shell");

        // 按 telegram + chat-1 过滤（不限发送者）
        let telegram_chat1 =
            mgr.list_non_cli_pending_requests(None, Some("telegram"), Some("chat-1"));
        assert_eq!(telegram_chat1.len(), 2);
    }

    /// 测试过期的待审批请求会被清理
    ///
    /// 验证当待审批请求的 `expires_at` 时间已过时：
    /// 1. 不会出现在列表查询结果中
    /// 2. 确认请求返回 `PendingApprovalError::NotFound`
    #[test]
    fn pending_non_cli_approval_expiry_is_pruned() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        let req = mgr.create_non_cli_pending_request(
            "shell",
            "alice",
            "telegram",
            "chat-1",
            None,
            serde_json::json!({"command": "pwd"}),
            None,
            None,
        );

        // 手动将请求设置为已过期（过期时间设为 1 分钟前）
        {
            let mut pending = mgr.pending_non_cli_requests.lock();
            let row = pending.get_mut(&req.request_id).expect("request row");
            row.expires_at = (Utc::now() - Duration::minutes(1)).to_rfc3339();
        }

        // 过期请求不在列表中
        let rows = mgr.list_non_cli_pending_requests(None, None, None);
        assert!(rows.is_empty());
        // 确认过期请求返回 NotFound 错误
        let err = mgr
            .confirm_non_cli_pending_request(&req.request_id, "alice", "telegram", "chat-1")
            .expect_err("expired request should not confirm");
        assert_eq!(err, PendingApprovalError::NotFound);
    }

    /// 测试未配置审批者时默认允许所有行为者
    ///
    /// 验证当 `non_cli_approval_approvers` 配置为空时，
    /// `is_non_cli_approval_actor_allowed` 对任何渠道和用户都返回 `true`。
    #[test]
    fn non_cli_approval_actor_defaults_to_allow_when_not_configured() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        // 未配置审批者列表时，默认允许所有行为者
        assert!(mgr.is_non_cli_approval_actor_allowed("telegram", "alice"));
        assert!(mgr.is_non_cli_approval_actor_allowed("discord", "bob"));
    }

    #[test]
    fn pending_non_cli_approval_deduplicates_same_arguments_only() {
        let mgr = ApprovalManager::from_config(&supervised_config());

        let first = mgr.create_non_cli_pending_request(
            "shell",
            "alice",
            "telegram",
            "chat-1",
            Some("need approval".to_string()),
            serde_json::json!({"command": "pwd"}),
            Some("msg-1".to_string()),
            Some("call-1".to_string()),
        );
        let duplicate = mgr.create_non_cli_pending_request(
            "shell",
            "alice",
            "telegram",
            "chat-1",
            Some("need approval".to_string()),
            serde_json::json!({"command": "pwd"}),
            Some("msg-1".to_string()),
            Some("call-1".to_string()),
        );
        let distinct = mgr.create_non_cli_pending_request(
            "shell",
            "alice",
            "telegram",
            "chat-1",
            Some("need approval".to_string()),
            serde_json::json!({"command": "git status"}),
            Some("msg-1".to_string()),
            Some("call-2".to_string()),
        );

        assert_eq!(first.request_id, duplicate.request_id);
        assert_ne!(first.request_id, distinct.request_id);
    }

    /// 测试非 CLI 自然语言审批模式默认为 Direct
    ///
    /// 验证未显式配置时，`non_cli_natural_language_approval_mode`
    /// 返回 `NonCliNaturalLanguageApprovalMode::Direct`。
    #[test]
    fn non_cli_natural_language_approval_mode_defaults_to_direct() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        assert_eq!(
            mgr.non_cli_natural_language_approval_mode(),
            NonCliNaturalLanguageApprovalMode::Direct
        );
    }

    /// 测试审批者允许列表支持精确匹配和通配符
    ///
    /// 验证 `non_cli_approval_approvers` 配置支持以下格式：
    /// - `"alice"` - 全局允许用户 alice（所有渠道）
    /// - `"telegram:bob"` - 仅允许 telegram 渠道的 bob
    /// - `"discord:*"` - 允许 discord 渠道的所有用户
    /// - `"*:carol"` - 允许所有渠道的 carol
    #[test]
    fn non_cli_approval_actor_allowlist_supports_exact_and_wildcards() {
        let mut cfg = supervised_config();
        // 配置审批者允许列表：支持精确匹配和通配符
        cfg.non_cli_approval_approvers = vec![
            "alice".to_string(),        // 全局允许 alice
            "telegram:bob".to_string(), // telegram 渠道的 bob
            "discord:*".to_string(),    // discord 渠道的所有用户
            "*:carol".to_string(),      // 所有渠道的 carol
        ];
        let mgr = ApprovalManager::from_config(&cfg);

        // 验证各种匹配情况
        assert!(mgr.is_non_cli_approval_actor_allowed("telegram", "alice")); // 全局 alice
        assert!(mgr.is_non_cli_approval_actor_allowed("telegram", "bob")); // telegram:bob
        assert!(mgr.is_non_cli_approval_actor_allowed("discord", "anyone")); // discord:*
        assert!(mgr.is_non_cli_approval_actor_allowed("slack", "carol")); // *:carol

        // 验证拒绝情况
        assert!(!mgr.is_non_cli_approval_actor_allowed("telegram", "mallory")); // 不匹配
        assert!(!mgr.is_non_cli_approval_actor_allowed("slack", "bob")); // bob 仅限 telegram
    }

    /// 测试自然语言审批模式遵循配置覆盖
    ///
    /// 验证当 `non_cli_natural_language_approval_mode` 配置项
    /// 被显式设置为非默认值时，`non_cli_natural_language_approval_mode()`
    /// 返回配置的值。
    #[test]
    fn non_cli_natural_language_approval_mode_honors_config_override() {
        let mut cfg = supervised_config();
        // 显式设置自然语言审批模式
        cfg.non_cli_natural_language_approval_mode =
            NonCliNaturalLanguageApprovalMode::RequestConfirm;
        let mgr = ApprovalManager::from_config(&cfg);
        assert_eq!(
            mgr.non_cli_natural_language_approval_mode(),
            NonCliNaturalLanguageApprovalMode::RequestConfirm
        );
    }

    /// 测试自然语言审批模式支持按渠道覆盖
    ///
    /// 验证 `non_cli_natural_language_approval_mode_by_channel` 配置
    /// 可以为特定渠道设置不同的审批模式，覆盖全局默认值。
    #[test]
    fn non_cli_natural_language_approval_mode_supports_per_channel_override() {
        let mut cfg = supervised_config();
        // 全局默认为 Direct 模式
        cfg.non_cli_natural_language_approval_mode = NonCliNaturalLanguageApprovalMode::Direct;
        // 为 discord 渠道单独设置 RequestConfirm 模式
        cfg.non_cli_natural_language_approval_mode_by_channel
            .insert("discord".to_string(), NonCliNaturalLanguageApprovalMode::RequestConfirm);
        let mgr = ApprovalManager::from_config(&cfg);

        // telegram 使用全局默认模式
        assert_eq!(
            mgr.non_cli_natural_language_approval_mode_for_channel("telegram"),
            NonCliNaturalLanguageApprovalMode::Direct
        );
        // discord 使用渠道专属覆盖模式
        assert_eq!(
            mgr.non_cli_natural_language_approval_mode_for_channel("discord"),
            NonCliNaturalLanguageApprovalMode::RequestConfirm
        );
    }

    /// 测试运行时替换非 CLI 策略更新所有模式和审批者
    ///
    /// 验证 `replace_runtime_non_cli_policy` 能够一次性更新：
    /// - 自动批准工具列表
    /// - 始终询问工具列表
    /// - 审批者允许列表
    /// - 全局自然语言审批模式
    /// - 按渠道的审批模式覆盖
    #[test]
    fn replace_runtime_non_cli_policy_updates_modes_and_approvers() {
        let cfg = supervised_config();
        let mgr = ApprovalManager::from_config(&cfg);

        // 准备按渠道的模式覆盖
        let mut mode_overrides = HashMap::new();
        mode_overrides.insert("telegram".to_string(), NonCliNaturalLanguageApprovalMode::Disabled);
        mode_overrides
            .insert("discord".to_string(), NonCliNaturalLanguageApprovalMode::RequestConfirm);

        // 执行运行时策略替换
        mgr.replace_runtime_non_cli_policy(
            &["mock_price".to_string()],               // 新的自动批准列表
            &["shell".to_string()],                    // 新的始终询问列表
            &["telegram:alice".to_string()],           // 新的审批者列表
            NonCliNaturalLanguageApprovalMode::Direct, // 全局模式
            &mode_overrides,                           // 按渠道模式覆盖
        );

        // 验证自动批准列表已更新
        assert!(!mgr.needs_approval("mock_price"));
        // 验证始终询问列表已更新
        assert!(mgr.needs_approval("shell"));
        // 验证审批者列表已更新
        assert!(mgr.is_non_cli_approval_actor_allowed("telegram", "alice"));
        assert!(!mgr.is_non_cli_approval_actor_allowed("telegram", "bob"));
        // 验证按渠道模式覆盖
        assert_eq!(
            mgr.non_cli_natural_language_approval_mode_for_channel("telegram"),
            NonCliNaturalLanguageApprovalMode::Disabled
        );
        assert_eq!(
            mgr.non_cli_natural_language_approval_mode_for_channel("discord"),
            NonCliNaturalLanguageApprovalMode::RequestConfirm
        );
        // 验证未覆盖渠道使用全局模式
        assert_eq!(
            mgr.non_cli_natural_language_approval_mode_for_channel("slack"),
            NonCliNaturalLanguageApprovalMode::Direct
        );
    }

    // ═══════════════════════════════════════════════════════════
    // audit log 审计日志测试
    // ═══════════════════════════════════════════════════════════

    /// 测试审计日志记录审批决策
    ///
    /// 验证 `record_decision` 将审批决策正确记录到审计日志，
    /// 日志条目包含工具名称、决策类型等信息，按记录顺序排列。
    #[test]
    fn audit_log_records_decisions() {
        let mgr = ApprovalManager::from_config(&supervised_config());

        // 记录两个审批决策
        mgr.record_decision(
            "shell",
            &serde_json::json!({"command": "rm -rf ./build/"}),
            ApprovalResponse::No,
            "cli",
        );
        mgr.record_decision(
            "file_write",
            &serde_json::json!({"path": "out.txt", "content": "hello"}),
            ApprovalResponse::Yes,
            "cli",
        );

        // 验证审计日志内容
        let log = mgr.audit_log();
        assert_eq!(log.len(), 2);
        assert_eq!(log[0].tool_name, "shell");
        assert_eq!(log[0].decision, ApprovalResponse::No);
        assert_eq!(log[1].tool_name, "file_write");
        assert_eq!(log[1].decision, ApprovalResponse::Yes);
    }

    /// 测试审计日志包含时间戳和渠道信息
    ///
    /// 验证每个审计日志条目包含非空的时间戳
    /// 和正确的渠道标识（如 "telegram"）。
    #[test]
    fn audit_log_contains_timestamp_and_channel() {
        let mgr = ApprovalManager::from_config(&supervised_config());
        mgr.record_decision(
            "shell",
            &serde_json::json!({"command": "ls"}),
            ApprovalResponse::Yes,
            "telegram",
        );

        let log = mgr.audit_log();
        assert_eq!(log.len(), 1);
        // 时间戳应非空
        assert!(!log[0].timestamp.is_empty());
        // 渠道应为 telegram
        assert_eq!(log[0].channel, "telegram");
    }

    // ═══════════════════════════════════════════════════════════
    // summarize_args 参数摘要测试
    // ═══════════════════════════════════════════════════════════

    /// 测试对象类型参数的摘要生成
    ///
    /// 验证 `summarize_args` 将 JSON 对象格式化为
    /// "key: value" 形式的可读字符串。
    #[test]
    fn summarize_args_object() {
        let args = serde_json::json!({"command": "ls -la", "cwd": "/tmp"});
        let summary = summarize_args(&args);
        assert!(summary.contains("command: ls -la"));
        assert!(summary.contains("cwd: /tmp"));
    }

    /// 测试长值被截断
    ///
    /// 验证 `summarize_args` 对过长的值进行截断处理，
    /// 在截断处添加省略号（…），确保输出不会过长。
    #[test]
    fn summarize_args_truncates_long_values() {
        let long_val = "x".repeat(200);
        let args = serde_json::json!({ "content": long_val });
        let summary = summarize_args(&args);
        // 截断后应包含省略号
        assert!(summary.contains('…'));
        // 总长度应小于原始值长度
        assert!(summary.len() < 200);
    }

    /// 测试 Unicode 安全截断
    ///
    /// 验证 `summarize_args` 在截断包含多字节 Unicode 字符
    ///（如 emoji）的字符串时，不会在字符中间截断导致乱码。
    #[test]
    fn summarize_args_unicode_safe_truncation() {
        let long_val = "🦀".repeat(120);
        let args = serde_json::json!({ "content": long_val });
        let summary = summarize_args(&args);
        assert!(summary.contains("content:"));
        // Unicode 截断后仍应包含省略号
        assert!(summary.contains('…'));
    }

    /// 测试非对象类型参数的摘要生成
    ///
    /// 验证当参数不是 JSON 对象（如字符串）时，
    /// `summarize_args` 直接返回该值的字符串表示。
    #[test]
    fn summarize_args_non_object() {
        let args = serde_json::json!("just a string");
        let summary = summarize_args(&args);
        assert!(summary.contains("just a string"));
    }

    // ═══════════════════════════════════════════════════════════
    // ApprovalResponse serde 序列化/反序列化测试
    // ═══════════════════════════════════════════════════════════

    /// 测试 ApprovalResponse 枚举的序列化/反序列化往返
    ///
    /// 验证 `ApprovalResponse` 枚举能够正确序列化为小写字符串
    ///（如 "always"、"no"），并能从该格式反序列化还原。
    #[test]
    fn approval_response_serde_roundtrip() {
        // 序列化测试
        let json = serde_json::to_string(&ApprovalResponse::Always).unwrap();
        assert_eq!(json, "\"always\"");
        // 反序列化测试
        let parsed: ApprovalResponse = serde_json::from_str("\"no\"").unwrap();
        assert_eq!(parsed, ApprovalResponse::No);
    }

    // ═══════════════════════════════════════════════════════════
    // ApprovalRequest 序列化测试
    // ═══════════════════════════════════════════════════════════

    /// 测试 ApprovalRequest 结构体的序列化/反序列化
    ///
    /// 验证 `ApprovalRequest` 结构体能够正确序列化为 JSON
    /// 并从 JSON 反序列化还原，字段值保持一致。
    #[test]
    fn approval_request_serde() {
        let req = ApprovalRequest {
            tool_name: "shell".into(),
            arguments: serde_json::json!({"command": "echo hi"}),
        };
        // 序列化后再反序列化
        let json = serde_json::to_string(&req).unwrap();
        let parsed: ApprovalRequest = serde_json::from_str(&json).unwrap();
        // 验证字段一致
        assert_eq!(parsed.tool_name, "shell");
    }
}
