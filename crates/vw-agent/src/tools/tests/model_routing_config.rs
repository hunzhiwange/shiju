//! 模型路由配置工具的单元测试模块
//!
//! 本模块提供对 `ModelRoutingConfigTool` 的全面测试覆盖，验证模型路由配置管理的各项功能。
//! 测试范围包括：
//! - 默认模型配置的设置和更新
//! - 场景路由规则的创建、查询和删除
//! - 代理（Agent）配置的管理
//! - 安全策略（如只读模式）的执行
//!
//! # 测试架构
//!
//! 每个测试用例遵循以下模式：
//! 1. 创建临时工作目录
//! 2. 初始化测试配置和安全策略
//! 3. 执行工具操作
//! 4. 验证操作结果

use super::super::*;
use crate::Config;
use crate::app::agent::config::schema::ConfigExt;
use crate::app::agent::security::{AutonomyLevel, SecurityPolicy};
use serde_json::{Value, json};
use tempfile::TempDir;

/// 创建测试用的安全策略（监管模式）
///
/// 返回一个配置为监管模式（`Supervised`）的安全策略，
/// 适用于需要写入权限的测试场景。
///
/// # 返回值
///
/// 返回包装在 `Arc` 中的 `SecurityPolicy` 实例，具有以下特性：
/// - 自主级别：`Supervised`（监管模式，允许有条件的写入操作）
/// - 工作目录：系统临时目录
fn test_security() -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::Supervised,
        workspace_dir: std::env::temp_dir(),
        ..SecurityPolicy::default()
    })
}

/// 创建只读模式的安全策略
///
/// 返回一个配置为只读模式（`ReadOnly`）的安全策略，
/// 用于验证工具在受限权限下的行为。
///
/// # 返回值
///
/// 返回包装在 `Arc` 中的 `SecurityPolicy` 实例，具有以下特性：
/// - 自主级别：`ReadOnly`（只读模式，禁止所有写入操作）
/// - 工作目录：系统临时目录
fn readonly_security() -> Arc<SecurityPolicy> {
    Arc::new(SecurityPolicy {
        autonomy: AutonomyLevel::ReadOnly,
        workspace_dir: std::env::temp_dir(),
        ..SecurityPolicy::default()
    })
}

/// 创建测试用的配置对象
///
/// 在指定的临时目录中初始化一个完整的 `Config` 实例，
/// 包括工作空间目录和配置文件路径的设置。
///
/// # 参数
///
/// * `tmp` - 临时目录引用，用于存放测试相关的所有文件
///
/// # 返回值
///
/// 返回包装在 `Arc` 中的 `Config` 实例，配置已持久化到磁盘。
///
/// # Panics
///
/// 如果配置保存失败，测试将 panic。
async fn test_config(tmp: &TempDir) -> Arc<Config> {
    std::fs::create_dir_all(tmp.path().join("workspace")).unwrap();
    let config: Config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    config.save().await.unwrap();
    Arc::new(config)
}

/// 测试设置默认模型配置的功能
///
/// 验证 `set_default` 操作能够正确更新：
/// - Provider（服务提供商）
/// - Model（模型名称）
/// - Temperature（温度参数）
///
/// # 测试流程
///
/// 1. 创建临时目录和工具实例
/// 2. 执行 `set_default` 操作，设置 kimi 提供商的 moonshot-v1-8k 模型
/// 3. 验证操作成功且配置正确更新
#[tokio::test]
async fn set_default_updates_provider_model_and_temperature() {
    let tmp = TempDir::new().unwrap();
    let tool = ModelRoutingConfigTool::new(test_config(&tmp).await, test_security());

    // 执行设置默认配置操作
    let result = tool
        .execute(json!({
            "action": "set_default",
            "provider": "kimi",
            "model": "moonshot-v1-8k",
            "temperature": 0.2
        }))
        .await
        .unwrap();

    // 验证操作成功
    assert!(result.success, "{:?}", result.error);
    // 解析输出并验证配置值
    let output: Value = serde_json::from_str(&result.output).unwrap();
    assert_eq!(output["config"]["default"]["provider"].as_str(), Some("kimi"));
    assert_eq!(output["config"]["default"]["model"].as_str(), Some("moonshot-v1-8k"));
    assert_eq!(output["config"]["default"]["temperature"].as_f64(), Some(0.2));
}

/// 测试创建场景路由规则的功能
///
/// 验证 `upsert_scenario` 操作能够正确创建场景配置，
/// 同时自动创建对应的分类规则。
///
/// # 测试流程
///
/// 1. 创建 "coding" 场景，配置 OpenAI GPT-5.3-Codex 模型
/// 2. 启用查询分类功能
/// 3. 设置关键词和模式匹配规则
/// 4. 验证场景和分类规则都正确创建
#[tokio::test]
async fn upsert_scenario_creates_route_and_rule() {
    let tmp = TempDir::new().unwrap();
    let tool = ModelRoutingConfigTool::new(test_config(&tmp).await, test_security());

    // 创建编程场景的路由配置
    let result = tool
        .execute(json!({
            "action": "upsert_scenario",
            "hint": "coding",
            "provider": "openai",
            "model": "gpt-5.3-codex",
            "classification_enabled": true,
            "keywords": ["code", "bug", "refactor"],
            "patterns": ["```"],
            "priority": 50
        }))
        .await
        .unwrap();

    assert!(result.success, "{:?}", result.error);

    // 查询当前配置以验证场景已创建
    let get_result = tool.execute(json!({"action": "get"})).await.unwrap();
    assert!(get_result.success);
    let output: Value = serde_json::from_str(&get_result.output).unwrap();

    // 验证查询分类已启用
    assert_eq!(output["query_classification"]["enabled"], json!(true));

    // 验证场景列表中包含新创建的 coding 场景
    let scenarios = output["scenarios"].as_array().unwrap();
    assert!(scenarios.iter().any(|item| {
        item["hint"] == json!("coding")
            && item["provider"] == json!("openai")
            && item["model"] == json!("gpt-5.3-codex")
    }));
}

/// 测试删除场景时同时删除关联规则的功能
///
/// 验证 `remove_scenario` 操作能够：
/// - 删除指定的场景配置
/// - 同时删除关联的分类规则
/// - 当没有场景时自动禁用查询分类
#[tokio::test]
async fn remove_scenario_also_removes_rule() {
    let tmp = TempDir::new().unwrap();
    let tool = ModelRoutingConfigTool::new(test_config(&tmp).await, test_security());

    // 首先创建一个场景
    let _ = tool
        .execute(json!({
            "action": "upsert_scenario",
            "hint": "coding",
            "provider": "openai",
            "model": "gpt-5.3-codex",
            "classification_enabled": true,
            "keywords": ["code"]
        }))
        .await
        .unwrap();

    // 删除刚创建的场景
    let removed = tool
        .execute(json!({
            "action": "remove_scenario",
            "hint": "coding"
        }))
        .await
        .unwrap();
    assert!(removed.success, "{:?}", removed.error);

    // 验证场景已被删除，查询分类已禁用
    let get_result = tool.execute(json!({"action": "get"})).await.unwrap();
    let output: Value = serde_json::from_str(&get_result.output).unwrap();
    assert_eq!(output["query_classification"]["enabled"], json!(false));
    // 场景列表应为空
    assert!(output["scenarios"].as_array().unwrap().is_empty());
}

/// 测试代理配置的添加和删除功能
///
/// 验证 `upsert_agent` 和 `remove_agent` 操作能够正确管理代理配置，
/// 包括代理的基本属性和权限设置。
///
/// # 测试流程
///
/// 1. 创建名为 "coder" 的代理，配置模型和工具权限
/// 2. 验证代理配置正确保存
/// 3. 删除代理
/// 4. 验证代理已从配置中移除
#[tokio::test]
async fn upsert_and_remove_delegate_agent() {
    let tmp = TempDir::new().unwrap();
    let tool = ModelRoutingConfigTool::new(test_config(&tmp).await, test_security());

    // 创建 coder 代理配置
    let upsert = tool
        .execute(json!({
            "action": "upsert_agent",
            "name": "coder",
            "provider": "openai",
            "model": "gpt-5.3-codex",
            "agentic": true,
            "allowed_tools": ["file_read", "file_write", "shell"],
            "max_iterations": 6
        }))
        .await
        .unwrap();
    assert!(upsert.success, "{:?}", upsert.error);

    // 验证代理配置已正确保存
    let get_result = tool.execute(json!({"action": "get"})).await.unwrap();
    let output: Value = serde_json::from_str(&get_result.output).unwrap();
    assert_eq!(output["agents"]["coder"]["provider"], json!("openai"));
    assert_eq!(output["agents"]["coder"]["model"], json!("gpt-5.3-codex"));
    assert_eq!(output["agents"]["coder"]["agentic"], json!(true));

    // 删除代理
    let remove = tool
        .execute(json!({
            "action": "remove_agent",
            "name": "coder"
        }))
        .await
        .unwrap();
    assert!(remove.success, "{:?}", remove.error);

    // 验证代理已被删除
    let get_result = tool.execute(json!({"action": "get"})).await.unwrap();
    let output: Value = serde_json::from_str(&get_result.output).unwrap();
    assert!(output["agents"]["coder"].is_null());
}

/// 测试只读模式阻止修改操作的安全策略
///
/// 验证在只读安全策略下，所有修改配置的操作都会被拒绝，
/// 确保安全策略的执行有效性。
///
/// # 测试流程
///
/// 1. 使用只读安全策略创建工具实例
/// 2. 尝试执行修改操作（set_default）
/// 3. 验证操作被拒绝，错误信息包含 "read-only" 提示
#[tokio::test]
async fn read_only_mode_blocks_mutating_actions() {
    let tmp = TempDir::new().unwrap();
    let tool = ModelRoutingConfigTool::new(test_config(&tmp).await, readonly_security());

    // 在只读模式下尝试修改配置
    let result = tool
        .execute(json!({
            "action": "set_default",
            "provider": "openai"
        }))
        .await
        .unwrap();

    // 验证操作被拒绝
    assert!(!result.success);
    // 验证错误信息提示只读限制
    assert!(result.error.unwrap_or_default().contains("read-only"));
}
