//! # Cron 合并任务测试模块
//!
//! 本模块提供对 cron 合并任务创建功能的全面测试覆盖。
//!
//! ## 功能概述
//!
//! - 测试合并任务的创建逻辑是否正确
//! - 验证默认调度表达式的应用
//! - 确保任务提示词包含关键指令
//! - 测试自定义调度配置（包括时区支持）
//!
//! ## 测试范围
//!
//! | 测试函数 | 验证内容 |
//! |---------|---------|
//! | `create_consolidation_job_produces_valid_job` | 任务基本属性正确性 |
//! | `create_consolidation_job_uses_correct_schedule` | 默认调度表达式 |
//! | `create_consolidation_job_prompt_contains_key_instructions` | 提示词关键内容 |
//! | `create_consolidation_job_with_custom_schedule_applies_tz` | 自定义时区 |

use agent_config::runtime::Config;
use crate::consolidation::{
    CONSOLIDATION_JOB_NAME, DEFAULT_SCHEDULE_EXPR, create_consolidation_job,
    create_consolidation_job_with_schedule,
};
use crate::{JobType, Schedule, SessionTarget};
use tempfile::TempDir;

/// 创建用于测试的配置实例
///
/// 构建一个临时的测试环境，包含独立的工作空间目录和配置文件路径。
/// 使用临时目录确保测试之间相互隔离，避免状态污染。
///
/// # 参数
///
/// - `tmp`: 临时目录引用，用于创建测试所需的工作空间
///
/// # 返回值
///
/// 返回配置好的 `Config` 实例，包含：
/// - `workspace_dir`: 临时工作空间目录
/// - `config_path`: 临时配置文件路径
/// - 其他字段使用默认值
///
/// # 副作用
///
/// 函数会创建 `workspace_dir` 目录结构
fn test_config(tmp: &TempDir) -> Config {
    // 构建测试配置，使用临时目录作为工作空间
    let config = Config {
        workspace_dir: tmp.path().join("workspace"),
        config_path: tmp.path().join("vibewindow.json"),
        ..Config::default()
    };
    // 确保工作空间目录存在
    std::fs::create_dir_all(&config.workspace_dir).unwrap();
    config
}

/// 测试：创建合并任务应产生有效的任务对象
///
/// 验证 `create_consolidation_job` 函数创建的任务具备正确的属性：
/// - 任务名称应与常量 `CONSOLIDATION_JOB_NAME` 匹配
/// - 任务类型应为 `JobType::Agent`
/// - 会话目标应为 `SessionTarget::Isolated`（隔离模式）
/// - `delete_after_run` 应为 false（任务持久化）
/// - `enabled` 应为 true（任务启用）
#[test]
fn create_consolidation_job_produces_valid_job() {
    // 创建临时测试环境
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    // 使用默认配置创建合并任务
    let job = create_consolidation_job(&config).unwrap();

    // 验证任务名称正确
    assert_eq!(job.name.as_deref(), Some(CONSOLIDATION_JOB_NAME));
    // 验证任务类型为 Agent 类型
    assert_eq!(job.job_type, JobType::Agent);
    // 验证会话采用隔离模式
    assert_eq!(job.session_target, SessionTarget::Isolated);
    // 验证任务不会在运行后自动删除
    assert!(!job.delete_after_run);
    // 验证任务默认启用
    assert!(job.enabled);
}

/// 测试：创建合并任务应使用正确的默认调度表达式
///
/// 验证未指定自定义调度时，任务使用默认的 cron 表达式：
/// - 调度表达式应与 `DEFAULT_SCHEDULE_EXPR` 匹配
/// - 时区应为 `None`（使用系统默认时区）
#[test]
fn create_consolidation_job_uses_correct_schedule() {
    // 创建临时测试环境
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    // 创建合并任务
    let job = create_consolidation_job(&config).unwrap();

    // 验证调度配置
    match &job.schedule {
        Schedule::Cron { expr, tz } => {
            // 调度表达式应使用默认值
            assert_eq!(expr, DEFAULT_SCHEDULE_EXPR);
            // 时区应未设置（使用系统默认）
            assert!(tz.is_none());
        }
        other => panic!("Expected Cron schedule, got {other:?}"),
    }
}

/// 测试：合并任务提示词应包含关键指令
///
/// 验证任务的 prompt 字段包含执行合并操作所需的所有关键指令：
/// - `memory_recall`: 内存召回工具调用指令
/// - `memory_store`: 内存存储工具调用指令
/// - `cron_runs`: 定时任务运行记录访问指令
/// - `consolidation_YYYY-MM-DD`: 合并结果的键格式规范
/// - `core`: 核心类别标记
/// - `MEMORY.md`: 内存文件引用
///
/// 这些指令确保 Agent 能够正确执行历史运行记录的合并与摘要操作。
#[test]
fn create_consolidation_job_prompt_contains_key_instructions() {
    // 创建临时测试环境
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    // 创建合并任务
    let job = create_consolidation_job(&config).unwrap();
    // 获取任务提示词（合并任务必须有提示词）
    let prompt = job.prompt.expect("consolidation job must have a prompt");

    // 验证提示词包含内存召回工具指令
    assert!(prompt.contains("memory_recall"), "prompt should instruct use of memory_recall");
    // 验证提示词包含内存存储工具指令
    assert!(prompt.contains("memory_store"), "prompt should instruct use of memory_store");
    // 验证提示词包含定时任务记录访问指令
    assert!(prompt.contains("cron_runs"), "prompt should instruct use of cron_runs");
    // 验证提示词指定了合并结果的键格式
    assert!(prompt.contains("consolidation_YYYY-MM-DD"), "prompt should specify key format");
    // 验证提示词指定了核心类别
    assert!(prompt.contains("core"), "prompt should specify core category");
    // 验证提示词提及了内存文件
    assert!(prompt.contains("MEMORY.md"), "prompt should mention MEMORY.md");
}

/// 测试：使用自定义调度创建合并任务应正确应用时区
///
/// 验证 `create_consolidation_job_with_schedule` 函数能够：
/// - 接受自定义的 cron 表达式
/// - 正确应用指定的时区配置
///
/// # 测试场景
///
/// - 自定义表达式: `"0 4 * * *"`（每天凌晨 4 点）
/// - 自定义时区: `"America/New_York"`（纽约时区）
#[test]
fn create_consolidation_job_with_custom_schedule_applies_tz() {
    // 创建临时测试环境
    let tmp = TempDir::new().unwrap();
    let config = test_config(&tmp);

    // 使用自定义调度表达式和时区创建合并任务
    let job = create_consolidation_job_with_schedule(
        &config,
        "0 4 * * *",
        Some("America/New_York".into()),
    )
    .unwrap();

    // 验证调度配置正确应用了自定义值
    match &job.schedule {
        Schedule::Cron { expr, tz } => {
            // 调度表达式应为自定义值
            assert_eq!(expr, "0 4 * * *");
            // 时区应为纽约时区
            assert_eq!(tz.as_deref(), Some("America/New_York"));
        }
        other => panic!("Expected Cron schedule, got {other:?}"),
    }
}
