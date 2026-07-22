//! Pushover 工具模块的测试套件
//!
//! 本模块包含对 `PushoverTool` 的全面测试用例，覆盖以下方面：
//!
//! - 工具基本属性：名称、描述、参数 schema
//! - 凭证解析：从 .env 文件中解析 token 和 user_key
//! - 安全策略：只读模式和速率限制的阻断行为
//! - 参数验证：优先级值范围校验
//!
//! ## 测试结构
//!
//! - 单元测试：验证工具的基本属性和 schema 定义
//! - 异步测试：验证凭证解析和执行行为
//!
//! ## 依赖项
//!
//! - `tempfile`：创建临时目录用于隔离测试环境
//! - `serde_json`：构建测试用的 JSON 参数

use super::super::*;
use crate::app::agent::security::AutonomyLevel;
use serde_json::json;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// 创建测试用的安全策略
///
/// 根据指定的自主级别和每小时最大操作数，构造一个 `SecurityPolicy` 实例。
/// 该策略用于控制工具在测试环境中的行为限制。
///
/// # 参数
///
/// - `level`: 自主级别，决定工具的执行权限（如 `ReadOnly` 或 `Full`）
/// - `max_actions_per_hour`: 每小时允许的最大操作数，用于速率限制
///
/// # 返回值
///
/// 返回一个包装在 `Arc` 中的 `SecurityPolicy` 实例，可跨线程共享。
///
/// # 示例
///
/// ```ignore
/// let policy = test_security(AutonomyLevel::Full, 100);
/// // 使用该策略创建工具实例...
/// ```
fn test_security(level: AutonomyLevel, max_actions_per_hour: u32) -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: level,
        max_actions_per_hour,
        workspace_dir: std::env::temp_dir(),
        ..SecurityPolicy::default()
    })
}

/// 测试 PushoverTool 的名称属性
///
/// 验证 `PushoverTool::name()` 方法返回正确的工具标识符 "pushover"。
/// 该标识符用于在工具注册表中查找和调用此工具。
#[test]
fn pushover_tool_name() {
    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), PathBuf::from("/tmp"));
    assert_eq!(tool.name(), "pushover");
}

/// 测试 PushoverTool 的描述属性
///
/// 验证 `PushoverTool::description()` 方法返回非空的描述字符串。
/// 该描述用于向用户或 AI 代理解释工具的功能和用途。
#[test]
fn pushover_tool_description() {
    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), PathBuf::from("/tmp"));
    assert!(!tool.description().is_empty());
}

/// 测试 PushoverTool 的参数 schema 定义
///
/// 验证工具的参数 schema 是一个有效的 JSON 对象类型，
/// 并且包含必需的 "message" 属性，用于指定推送消息内容。
#[test]
fn pushover_tool_has_parameters_schema() {
    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), PathBuf::from("/tmp"));
    let schema = tool.parameters_schema();
    assert_eq!(schema["type"], "object");
    assert!(schema["properties"].get("message").is_some());
}

/// 测试 PushoverTool 的必填参数约束
///
/// 验证 "message" 参数被正确标记为必填字段。
/// 调用者必须提供 message 参数，否则工具执行将失败。
#[test]
fn pushover_tool_requires_message() {
    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), PathBuf::from("/tmp"));
    let schema = tool.parameters_schema();
    let required = schema["required"].as_array().unwrap();
    assert!(required.contains(&serde_json::Value::String("message".to_string())));
}

/// 测试从 .env 文件正确解析凭证
///
/// 验证当 .env 文件中同时存在 `PUSHOVER_TOKEN` 和 `PUSHOVER_USER_KEY` 时，
/// 工具能够正确解析并返回这两个值。
///
/// # 测试步骤
///
/// 1. 创建临时目录和 .env 文件
/// 2. 写入有效的 token 和 user_key
/// 3. 验证 `get_credentials()` 返回正确的值
#[tokio::test]
async fn credentials_parsed_from_env_file() {
    let tmp = TempDir::new().unwrap();
    let env_path = tmp.path().join(".env");
    fs::write(&env_path, "PUSHOVER_TOKEN=testtoken123\nPUSHOVER_USER_KEY=userkey456\n").unwrap();

    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), tmp.path().to_path_buf());
    let result = tool.get_credentials().await;

    assert!(result.is_ok());
    let (token, user_key) = result.unwrap();
    assert_eq!(token, "testtoken123");
    assert_eq!(user_key, "userkey456");
}

/// 测试缺失 .env 文件时的错误处理
///
/// 验证当工作目录中不存在 .env 文件时，
/// `get_credentials()` 方法返回错误而不是 panic。
#[tokio::test]
async fn credentials_fail_without_env_file() {
    let tmp = TempDir::new().unwrap();
    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), tmp.path().to_path_buf());
    let result = tool.get_credentials().await;

    assert!(result.is_err());
}

/// 测试缺失 PUSHOVER_TOKEN 时的错误处理
///
/// 验证当 .env 文件中只有 `PUSHOVER_USER_KEY` 而没有 `PUSHOVER_TOKEN` 时，
/// `get_credentials()` 方法返回错误。
#[tokio::test]
async fn credentials_fail_without_token() {
    let tmp = TempDir::new().unwrap();
    let env_path = tmp.path().join(".env");
    fs::write(&env_path, "PUSHOVER_USER_KEY=userkey456\n").unwrap();

    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), tmp.path().to_path_buf());
    let result = tool.get_credentials().await;

    assert!(result.is_err());
}

/// 测试缺失 PUSHOVER_USER_KEY 时的错误处理
///
/// 验证当 .env 文件中只有 `PUSHOVER_TOKEN` 而没有 `PUSHOVER_USER_KEY` 时，
/// `get_credentials()` 方法返回错误。
#[tokio::test]
async fn credentials_fail_without_user_key() {
    let tmp = TempDir::new().unwrap();
    let env_path = tmp.path().join(".env");
    fs::write(&env_path, "PUSHOVER_TOKEN=testtoken123\n").unwrap();

    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), tmp.path().to_path_buf());
    let result = tool.get_credentials().await;

    assert!(result.is_err());
}

/// 测试 .env 文件中的注释被正确忽略
///
/// 验证 .env 解析器能够正确处理以 `#` 开头的注释行，
/// 不会将注释内容误解析为凭证值。
///
/// # 测试场景
///
/// - 文件包含多行注释
/// - 凭证值夹杂在注释之间
#[tokio::test]
async fn credentials_ignore_comments() {
    let tmp = TempDir::new().unwrap();
    let env_path = tmp.path().join(".env");
    fs::write(&env_path, "# This is a comment\nPUSHOVER_TOKEN=realtoken\n# Another comment\nPUSHOVER_USER_KEY=realuser\n").unwrap();

    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), tmp.path().to_path_buf());
    let result = tool.get_credentials().await;

    assert!(result.is_ok());
    let (token, user_key) = result.unwrap();
    assert_eq!(token, "realtoken");
    assert_eq!(user_key, "realuser");
}

/// 测试 PushoverTool 支持 priority 参数
///
/// 验证工具的参数 schema 中包含可选的 "priority" 属性，
/// 允许调用者设置消息的推送优先级。
#[test]
fn pushover_tool_supports_priority() {
    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), PathBuf::from("/tmp"));
    let schema = tool.parameters_schema();
    assert!(schema["properties"].get("priority").is_some());
}

/// 测试 PushoverTool 支持 sound 参数
///
/// 验证工具的参数 schema 中包含可选的 "sound" 属性，
/// 允许调用者自定义推送通知的声音。
#[test]
fn pushover_tool_supports_sound() {
    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), PathBuf::from("/tmp"));
    let schema = tool.parameters_schema();
    assert!(schema["properties"].get("sound").is_some());
}

/// 测试 .env 文件支持 export 前缀和引号包裹的值
///
/// 验证解析器能够正确处理以下格式变体：
///
/// - `export VAR=value` 格式（带 export 前缀）
/// - 双引号包裹的值：`VAR="value"`
/// - 单引号包裹的值：`VAR='value'`
///
/// 这些格式在 shell 脚本中很常见，工具应能兼容。
#[tokio::test]
async fn credentials_support_export_and_quoted_values() {
    let tmp = TempDir::new().unwrap();
    let env_path = tmp.path().join(".env");
    fs::write(&env_path, "export PUSHOVER_TOKEN=\"quotedtoken\"\nPUSHOVER_USER_KEY='quoteduser'\n")
        .unwrap();

    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), tmp.path().to_path_buf());
    let result = tool.get_credentials().await;

    assert!(result.is_ok());
    let (token, user_key) = result.unwrap();
    assert_eq!(token, "quotedtoken");
    assert_eq!(user_key, "quoteduser");
}

/// 测试只读模式下工具执行被阻断
///
/// 验证当安全策略的自主级别为 `ReadOnly` 时，
/// 工具的 `execute` 方法拒绝执行并返回包含 "read-only" 的错误信息。
///
/// # 安全考虑
///
/// 只读模式用于限制代理的行为，防止其执行具有副作用的操作。
#[tokio::test]
async fn execute_blocks_readonly_mode() {
    let tool =
        PushoverTool::new(test_security(AutonomyLevel::ReadOnly, 100), PathBuf::from("/tmp"));

    let result = tool.execute(json!({"message": "hello"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("read-only"));
}

/// 测试速率限制生效时的执行阻断
///
/// 验证当 `max_actions_per_hour` 设置为 0 时，
/// 工具的 `execute` 方法拒绝执行并返回包含 "rate limit" 的错误信息。
///
/// # 安全考虑
///
/// 速率限制用于防止工具被滥用或过度调用外部服务。
#[tokio::test]
async fn execute_blocks_rate_limit() {
    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 0), PathBuf::from("/tmp"));

    let result = tool.execute(json!({"message": "hello"})).await.unwrap();
    assert!(!result.success);
    assert!(result.error.unwrap().contains("rate limit"));
}

/// 测试优先级值超出范围时被拒绝
///
/// 验证当 priority 参数值超出有效范围 [-2, 2] 时，
/// 工具的 `execute` 方法拒绝执行并返回包含范围信息的错误。
///
/// # Pushover 优先级规范
///
/// - `-2`: 无通知提醒
/// - `-1`: 静默通知
/// - `0`: 默认优先级
/// - `1`: 高优先级（忽略静音时段）
/// - `2`: 紧急优先级（需确认收据）
#[tokio::test]
async fn execute_rejects_priority_out_of_range() {
    let tool = PushoverTool::new(test_security(AutonomyLevel::Full, 100), PathBuf::from("/tmp"));

    let result = tool.execute(json!({"message": "hello", "priority": 5})).await.unwrap();

    assert!(!result.success);
    assert!(result.error.unwrap().contains("-2..=2"));
}

#[test]
fn parse_env_value_strips_inline_comments_and_preserves_hashes_without_separator() {
    assert_eq!(PushoverTool::parse_env_value(" token # comment"), "token");
    assert_eq!(PushoverTool::parse_env_value("token#suffix"), "token#suffix");
}

#[tokio::test]
async fn execute_rejects_blank_message_before_loading_credentials() {
    let tool = PushoverTool::new(
        test_security(AutonomyLevel::Full, 100),
        PathBuf::from("/workspace-without-pushover-env"),
    );

    let error = tool
        .execute(json!({"message": "   "}))
        .await
        .expect_err("blank messages must be rejected as invalid input");

    assert!(error.to_string().contains("Missing 'message' parameter"));
}
