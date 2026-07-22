//! Git 操作工具测试模块
//!
//! 本模块包含对 `GitOperationsTool` 的全面测试套件，主要覆盖以下方面：
//!
//! - **参数净化安全测试**：验证 Git 参数净化功能能够正确阻止各类注入攻击，
//!   包括命令注入、分页器/编辑器注入、配置注入、重定向注入等。
//!
//! - **读写权限检测测试**：验证工具能够正确识别哪些 Git 操作需要写权限，
//!   哪些是只读操作，确保权限控制逻辑正确。
//!
//! - **只读模式限制测试**：验证在只读（ReadOnly）自治级别下，写操作被正确阻止，
//!   而只读操作能够正常执行。
//!
//! - **输入验证测试**：验证缺失操作参数和未知操作被正确拒绝。
//!
//! - **边界条件测试**：验证多字节字符（如 emoji）的提交消息截断不会导致 panic。

use super::super::*;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use serde_json::json;
use tempfile::TempDir;

/// 创建用于测试的 GitOperationsTool 实例
///
/// 该辅助函数创建一个配置了监督（Supervised）自治级别的 GitOperationsTool 实例，
/// 用于大多数测试场景。
///
/// # 参数
///
/// - `dir`: Git 仓库的工作目录路径
///
/// # 返回值
///
/// 返回配置好安全策略的 `GitOperationsTool` 实例
///
/// # 示例
///
/// ```ignore
/// let tmp = TempDir::new().unwrap();
/// let tool = test_tool(tmp.path());
/// // 使用 tool 进行测试...
/// ```
fn test_tool(dir: &std::path::Path) -> GitOperationsTool {
    let security = Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        ..SecurityPolicy::default()
    });
    GitOperationsTool::new(security, dir.to_path_buf())
}

fn assert_anyhow_error_contains<T>(result: anyhow::Result<T>, expected: &str) {
    let error = match result {
        Ok(_) => panic!("expected an error containing {expected:?}"),
        Err(error) => error,
    };
    let message = error.to_string();
    assert!(message.contains(expected), "expected error containing {expected:?}, got {message:?}");
}

/// 测试参数净化功能能够阻止命令注入攻击
///
/// 该测试验证 `sanitize_git_args` 方法能够正确识别并拒绝以下危险模式：
/// - `--exec=` 前缀（可能用于执行任意命令）
/// - 命令替换语法 `$()` 和反引号
/// - 管道操作符 `|`
/// - 命令分隔符 `;`
#[test]
fn sanitize_git_blocks_injection() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // 验证危险参数被正确阻止
    assert!(tool.sanitize_git_args("--exec=rm -rf /").is_err());
    assert!(tool.sanitize_git_args("$(echo pwned)").is_err());
    assert!(tool.sanitize_git_args("`malicious`").is_err());
    assert!(tool.sanitize_git_args("arg | cat").is_err());
    assert!(tool.sanitize_git_args("arg; rm file").is_err());
}

/// 测试参数净化功能能够阻止分页器和编辑器注入
///
/// 分页器和编辑器配置可能被滥用执行任意程序，因此需要特别阻止。
/// 该测试验证 `--pager=` 和 `--editor=` 参数被正确拒绝。
#[test]
fn sanitize_git_blocks_pager_editor_injection() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // 验证分页器和编辑器注入被阻止
    assert!(tool.sanitize_git_args("--pager=less").is_err());
    assert!(tool.sanitize_git_args("--editor=vim").is_err());
}

/// 测试参数净化功能能够阻止 Git 配置注入
///
/// Git 的 `-c` 标志允许在命令行设置配置项，这可能被滥用覆盖关键安全设置
/// （如 SSH 命令、分页器等）。该测试验证此类注入被正确阻止。
///
/// 测试覆盖两种形式：
/// - `-c key=value`（空格分隔）
/// - `-c=key=value`（等号连接）
#[test]
fn sanitize_git_blocks_config_injection() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // 验证配置注入被阻止（精确匹配 `-c` 标志）
    assert!(tool.sanitize_git_args("-c core.sshCommand=evil").is_err());
    assert!(tool.sanitize_git_args("-c=core.pager=less").is_err());
}

/// 测试 `--no-verify` 标志被正确阻止
///
/// `--no-verify` 标志会跳过 Git 钩子，可能绕过重要的安全检查，
/// 因此需要被阻止以防止安全策略被绕过。
#[test]
fn sanitize_git_blocks_no_verify() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // 验证跳过钩子的标志被阻止
    assert!(tool.sanitize_git_args("--no-verify").is_err());
}

/// 测试参数中的重定向被正确阻止
///
/// Shell 重定向（`>`、`>>` 等）可能被用于写入任意文件或覆盖敏感数据，
/// 该测试验证包含重定向的参数被正确拒绝。
#[test]
fn sanitize_git_blocks_redirect_in_args() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // 验证重定向被阻止
    assert!(tool.sanitize_git_args("file.txt > /tmp/out").is_err());
}

/// 测试 `--cached` 标志不会被 `-c` 检查误拦截
///
/// 这是一个重要的边界条件测试。`--cached` 是合法的 Git 标志，
/// 用于查看暂存区内容，不应被 `-c` 配置注入检查误判为危险参数。
///
/// 该测试确保净化逻辑能够区分：
/// - 危险的 `-c`（配置注入）
/// - 安全的 `--cached`（暂存区查看）
#[test]
fn sanitize_git_cached_not_blocked() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // `--cached` 不应被 `-c` 检查阻止
    assert!(tool.sanitize_git_args("--cached").is_ok());
    // 其他以 `-c` 前缀开头的安全标志也应被允许
    assert!(tool.sanitize_git_args("-cached").is_ok());
}

/// 测试安全的 Git 参数能够正常通过净化检查
///
/// 该测试验证常见的、安全的 Git 参数格式能够通过 `sanitize_git_args` 检查，
/// 包括分支名、文件路径、目录指示符等。
#[test]
fn sanitize_git_allows_safe() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // 验证安全参数被允许通过
    assert!(tool.sanitize_git_args("main").is_ok());
    assert!(tool.sanitize_git_args("feature/test-branch").is_ok());
    assert!(tool.sanitize_git_args("--cached").is_ok());
    assert!(tool.sanitize_git_args("src/main.rs").is_ok());
    assert!(tool.sanitize_git_args(".").is_ok());
}

#[test]
fn sanitize_git_blocks_transport_and_tool_overrides_case_insensitively() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    for argument in [
        "--upload-pack=helper",
        "--receive-pack=helper",
        "--UPLOAD-PACK=helper",
        "--RECEIVE-PACK=helper",
        "--PAGER=helper",
        "--EDITOR=helper",
        "--NO-VERIFY",
    ] {
        assert_anyhow_error_contains(
            tool.sanitize_git_args(argument),
            "Blocked potentially dangerous git argument",
        );
    }
}

#[test]
fn sanitize_git_preserves_safe_tokens_and_accepts_empty_input() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert_eq!(
        tool.sanitize_git_args("  feature/topic   src/lib.rs  ").unwrap(),
        vec!["feature/topic", "src/lib.rs"]
    );
    assert!(tool.sanitize_git_args("   \n\t  ").unwrap().is_empty());
}

/// 测试写权限需求检测功能
///
/// 该测试验证 `requires_write_access` 方法能够正确区分需要写权限的操作
/// 和只读操作。写操作（如 commit、add、checkout）应返回 true，
/// 只读操作（如 status、diff、log）应返回 false。
#[test]
fn requires_write_detection() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // 写操作应需要写权限
    assert!(tool.requires_write_access("commit"));
    assert!(tool.requires_write_access("add"));
    assert!(tool.requires_write_access("checkout"));

    // 只读操作不需要写权限
    assert!(!tool.requires_write_access("status"));
    assert!(!tool.requires_write_access("diff"));
    assert!(!tool.requires_write_access("log"));
}

/// 测试分支列表操作不被错误地归类为写操作
///
/// `git branch`（不带参数）是只读操作，仅列出分支信息。
/// 该测试确保它不会被错误地要求写权限，从而在只读模式下被阻止。
///
/// 同时验证 `is_read_only` 方法正确识别该操作为只读。
#[test]
fn branch_is_not_write_gated() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // 分支列表是只读操作，不应要求写权限
    assert!(!tool.requires_write_access("branch"));
    assert!(tool.is_read_only("branch"));
}

/// 测试只读操作识别功能
///
/// 该测试验证 `is_read_only` 方法能够正确识别各类只读 Git 操作，
/// 确保只读检测逻辑的准确性。
#[test]
fn is_read_only_detection() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // 验证只读操作被正确识别
    assert!(tool.is_read_only("status"));
    assert!(tool.is_read_only("diff"));
    assert!(tool.is_read_only("log"));
    assert!(tool.is_read_only("branch"));

    // 验证写操作不被识别为只读
    assert!(!tool.is_read_only("commit"));
    assert!(!tool.is_read_only("add"));
}

#[test]
fn access_classification_covers_all_internal_operation_names() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    for operation in ["commit", "add", "checkout", "stash", "reset", "revert"] {
        assert!(tool.requires_write_access(operation), "{operation} should require write access");
        assert!(!tool.is_read_only(operation), "{operation} should not be read-only");
    }

    for operation in ["status", "diff", "log", "show", "branch", "rev-parse"] {
        assert!(tool.is_read_only(operation), "{operation} should be read-only");
        assert!(
            !tool.requires_write_access(operation),
            "{operation} should not require write access"
        );
    }

    assert!(!tool.requires_write_access("unknown"));
    assert!(!tool.is_read_only("unknown"));
}

#[test]
fn tool_metadata_and_schema_describe_the_supported_contract() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert_eq!(tool.name(), "git_operations");
    assert!(tool.description().contains("结构化 Git 操作"));

    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert_eq!(schema["required"], json!(["operation"]));
    assert_eq!(
        schema["properties"]["operation"]["enum"],
        json!(["status", "diff", "log", "branch", "commit", "add", "checkout", "stash"])
    );
    assert_eq!(schema["properties"]["action"]["enum"], json!(["push", "pop", "list", "drop"]));
}

/// 测试在只读模式下写操作被正确阻止
///
/// 当自治级别设置为 `ReadOnly` 时，任何写操作都应被阻止。
/// 该测试创建一个真实的 Git 仓库，并验证 commit 操作
/// 在只读模式下返回包含自治级别限制信息的错误。
///
/// 注意：错误消息应包含 "higher autonomy" 而非 "read-only"，
/// 因为 `can_act()` 在 ReadOnly 级别返回 false。
#[tokio::test]
async fn blocks_readonly_mode_for_write_ops() {
    let tmp = TempDir::new().unwrap();

    // 权限检查只需要仓库标记，不能执行到 Git 命令。
    std::fs::create_dir(tmp.path().join(".git")).unwrap();

    // 配置只读安全策略
    let security =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let tool = GitOperationsTool::new(security, tmp.path().to_path_buf());

    // 执行写操作并验证被阻止
    let result = tool.execute(json!({"operation": "commit", "message": "test"})).await.unwrap();
    assert!(!result.success);

    // can_act() 对 ReadOnly 返回 false，因此应得到"更高自治级别"的错误消息
    assert!(result.error.as_deref().unwrap_or("").contains("higher autonomy"));
}

/// 测试在只读模式下分支列表操作能够正常执行
///
/// 验证 `git branch`（分支列表）在只读自治级别下不会被错误阻止。
/// 这是一个关键的安全边界测试，确保只读模式不会过度限制合法的只读查询。
///
/// 测试步骤：
/// 1. 创建临时 Git 仓库
/// 2. 配置只读安全策略
/// 3. 执行 branch 操作
/// 4. 验证错误消息不包含只读限制相关信息
#[tokio::test]
async fn allows_branch_listing_in_readonly_mode() {
    let tmp = TempDir::new().unwrap();

    // 初始化 Git 仓库以便命令能够成功执行
    std::process::Command::new("git").args(["init"]).current_dir(tmp.path()).output().unwrap();

    // 配置只读安全策略
    let security =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let tool = GitOperationsTool::new(security, tmp.path().to_path_buf());

    // 执行分支列表操作
    let result = tool.execute(json!({"operation": "branch"})).await;

    // 验证分支列表不被只读模式阻止
    let error_msg =
        result.as_ref().ok().and_then(|tool_result| tool_result.error.as_deref()).unwrap_or("");
    assert!(
        !error_msg.contains("read-only") && !error_msg.contains("higher autonomy"),
        "branch listing should not be blocked in read-only mode, got: {error_msg}"
    );
}

/// 测试在只读模式下只读操作能够正常执行
///
/// 验证只读操作（如 `git status`）在只读自治级别下不会被自治限制阻止。
/// 即使操作因其他原因失败（如缺少 Git 仓库），错误也应来自 Git 本身，
/// 而非自治级别限制。
///
/// 测试要点：
/// - 不初始化 Git 仓库，操作会失败
/// - 失败原因应是 Git 相关错误，而非自治限制
/// - 错误消息不应包含 "read-only" 或 "autonomy" 字样
#[tokio::test]
async fn allows_readonly_ops_in_readonly_mode() {
    let tmp = TempDir::new().unwrap();

    // 配置只读安全策略
    let security =
        Arc::new(SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() });
    let tool = GitOperationsTool::new(security, tmp.path().to_path_buf());

    // 执行只读操作（因缺少 Git 仓库会失败，但不应被自治限制阻止）
    let result = tool.execute(json!({"operation": "status"})).await.unwrap();

    // 验证失败是由于缺少 Git 仓库，而非自治限制
    assert!(!result.success, "Expected failure due to missing git repo");
    let error_msg = result.error.as_deref().unwrap_or("");
    assert!(!error_msg.is_empty(), "Expected a git-related error message");

    // 错误应与 Git 相关，而非自治限制
    assert!(
        !error_msg.contains("read-only") && !error_msg.contains("autonomy"),
        "Error should be about git, not about autonomy restrictions: {error_msg}"
    );
}

/// 测试缺失操作参数时返回正确的错误
///
/// 当执行请求中未提供 `operation` 字段时，工具应返回明确的错误消息，
/// 指出缺少必要的操作参数。
#[tokio::test]
async fn rejects_missing_operation() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    // 执行空参数请求
    let result = tool.execute(json!({})).await.unwrap();

    // 验证返回缺失操作参数的错误
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Missing 'operation'"));
}

/// 测试未知操作被正确拒绝
///
/// 当请求执行不在允许列表中的操作（如 `push`）时，工具应返回
/// 明确的错误消息，指出操作未知。
///
/// 注意：`push` 操作被有意排除在允许列表之外，以防止意外推送代码。
#[tokio::test]
async fn rejects_unknown_operation() {
    let tmp = TempDir::new().unwrap();

    // 分发未知操作只需要仓库标记，不启动 Git 进程。
    std::fs::create_dir(tmp.path().join(".git")).unwrap();

    let tool = test_tool(tmp.path());

    // 尝试执行不允许的 push 操作
    let result = tool.execute(json!({"operation": "push"})).await.unwrap();

    // 验证返回未知操作的错误
    assert!(!result.success);
    assert!(result.error.as_deref().unwrap_or("").contains("Unknown operation"));
}

/// 测试多字节字符提交消息截断不会导致 panic
///
/// 提交消息长度限制是基于字符数而非字节数。该测试验证：
/// - 包含多字节字符（如 emoji 🦀）的长消息能够正确截断
/// - 截断后的消息恰好为 2000 个字符
/// - 截断过程不会因 UTF-8 边界问题导致 panic
///
/// 这是一个重要的安全性测试，确保在处理国际化内容时不会发生崩溃。
#[test]
fn truncates_multibyte_commit_message_without_panicking() {
    // 创建包含 2500 个 emoji 的字符串（每个 emoji 占 4 字节）
    let long = "🦀".repeat(2500);

    // 执行截断
    let truncated = GitOperationsTool::truncate_commit_message(&long);

    // 验证截断后恰好为 2000 个字符
    assert_eq!(truncated.chars().count(), 2000);
}

#[test]
fn truncate_commit_message_preserves_values_at_the_limit() {
    let message = "界".repeat(2000);

    assert_eq!(GitOperationsTool::truncate_commit_message(&message), message);
}

#[test]
fn truncate_commit_message_caps_ascii_and_appends_ellipsis() {
    let truncated = GitOperationsTool::truncate_commit_message(&"x".repeat(2001));

    assert_eq!(truncated, format!("{}...", "x".repeat(1997)));
    assert_eq!(truncated.chars().count(), 2000);
}

#[tokio::test]
async fn execute_rejects_non_repository_before_dispatch() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    let result = tool.execute(json!({"operation": "status"})).await.unwrap();

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("Not in a git repository"));
}

#[tokio::test]
async fn execute_finds_parent_repository_before_dispatch() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join(".git")).unwrap();
    let nested = tmp.path().join("nested");
    std::fs::create_dir(&nested).unwrap();
    let tool = test_tool(&nested);

    let result = tool.execute(json!({"operation": "unsupported"})).await.unwrap();

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("Unknown operation: unsupported"));
}

#[tokio::test]
async fn execute_rejects_exhausted_action_budget_before_dispatch() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join(".git")).unwrap();
    let security =
        Arc::new(SecurityPolicy { max_actions_per_hour: 0, ..SecurityPolicy::default() });
    let tool = GitOperationsTool::new(security, tmp.path().to_path_buf());

    let result = tool.execute(json!({"operation": "status"})).await.unwrap();

    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("Action blocked: rate limit exceeded"));
}

#[tokio::test]
async fn commit_rejects_missing_and_blank_messages_before_git() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert_anyhow_error_contains(tool.git_commit(json!({})).await, "Missing 'message' parameter");
    assert_anyhow_error_contains(
        tool.git_commit(json!({"message": "  \n\t\n  "})).await,
        "Commit message cannot be empty",
    );
}

#[tokio::test]
async fn add_rejects_missing_and_dangerous_paths_before_git() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert_anyhow_error_contains(tool.git_add(json!({})).await, "Missing 'paths' parameter");
    assert_anyhow_error_contains(
        tool.git_add(json!({"paths": "src | helper"})).await,
        "Blocked potentially dangerous git argument",
    );
}

#[tokio::test]
async fn diff_rejects_dangerous_files_before_git() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert_anyhow_error_contains(
        tool.git_diff(json!({"files": "src; helper"})).await,
        "Blocked potentially dangerous git argument",
    );
}

#[tokio::test]
async fn checkout_rejects_invalid_branch_specifications_before_git() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert_anyhow_error_contains(tool.git_checkout(json!({})).await, "Missing 'branch' parameter");
    for branch in ["", "main other"] {
        assert_anyhow_error_contains(
            tool.git_checkout(json!({"branch": branch})).await,
            "Invalid branch specification",
        );
    }
    for branch in ["feature@{1}", "main^", "main~1"] {
        assert_anyhow_error_contains(
            tool.git_checkout(json!({"branch": branch})).await,
            "Branch name contains invalid characters",
        );
    }
}

#[tokio::test]
async fn stash_rejects_unknown_action_and_oversized_index_before_git() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    assert_anyhow_error_contains(
        tool.git_stash(json!({"action": "clear"})).await,
        "Unknown stash action: clear",
    );
    assert_anyhow_error_contains(
        tool.git_stash(json!({"action": "drop", "index": u64::MAX})).await,
        "stash index too large",
    );
}

#[tokio::test]
async fn commit_sanitizes_multiline_message_before_reporting_git_failure() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    let result = tool.git_commit(json!({"message": "  subject  \n\n  body line  "})).await.unwrap();

    assert!(!result.success);
    assert!(result.output.is_empty());
    assert!(result.error.as_deref().unwrap_or("").contains("Commit failed: Git command failed"));
}

#[tokio::test]
async fn add_checkout_and_stash_report_non_repository_git_failures() {
    let tmp = TempDir::new().unwrap();
    let tool = test_tool(tmp.path());

    let add = tool.git_add(json!({"paths": "src/lib.rs"})).await.unwrap();
    assert!(!add.success);
    assert!(add.error.as_deref().unwrap_or("").contains("Add failed: Git command failed"));

    let checkout = tool.git_checkout(json!({"branch": "feature/tests"})).await.unwrap();
    assert!(!checkout.success);
    assert!(
        checkout.error.as_deref().unwrap_or("").contains("Checkout failed: Git command failed")
    );

    for action in ["push", "save", "pop", "list", "drop"] {
        let stash = tool.git_stash(json!({"action": action, "index": 0})).await.unwrap();
        assert!(!stash.success, "{action} should fail outside a repository");
        assert!(
            stash.error.as_deref().unwrap_or("").contains(&format!("Stash {action} failed")),
            "unexpected {action} error: {:?}",
            stash.error
        );
    }
}
