//! 定时任务投递注入测试模块
//!
//! 本模块测试 `maybe_inject_cron_add_delivery` 函数的行为，
//! 该函数负责在创建定时任务时自动注入投递配置。
//!
//! # 测试场景
//!
//! - 从通道上下文自动填充投递配置
//! - 不覆盖显式指定的投递目标
//! - 跳过 shell 类型任务的投递注入

use crate::cron::maybe_inject_cron_add_delivery;

/// 测试从通道上下文自动填充投递配置
///
/// 验证当创建代理类型（agent）的定时任务时，
/// 如果未显式指定投递配置，系统应自动从当前通道上下文
/// （通道类型和目标地址）注入投递信息。
///
/// # 测试步骤
///
/// 1. 创建一个不包含 delivery 字段的 cron_add 参数
/// 2. 调用注入函数，传入 telegram 通道和目标地址
/// 3. 验证 delivery 字段被正确填充
#[test]
fn maybe_inject_cron_add_delivery_populates_agent_delivery_from_channel_context() {
    // 构造不带投递配置的定时任务参数
    let mut args = serde_json::json!({
        "job_type": "agent",
        "prompt": "remind me later"
    });

    // 从通道上下文注入投递配置
    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", Some("-10012345"));

    // 验证投递配置被正确注入
    assert_eq!(args["delivery"]["mode"], "announce");
    assert_eq!(args["delivery"]["channel"], "telegram");
    assert_eq!(args["delivery"]["to"], "-10012345");
}

/// 测试不覆盖显式指定的投递目标
///
/// 验证当用户已经在定时任务参数中显式指定了投递配置时，
/// 系统不应覆盖这些配置，即用户显式配置具有更高优先级。
///
/// # 测试步骤
///
/// 1. 创建一个已包含 delivery 字段的 cron_add 参数（指定 discord 通道）
/// 2. 调用注入函数，传入不同的通道上下文（telegram）
/// 3. 验证原有的投递配置保持不变
#[test]
fn maybe_inject_cron_add_delivery_does_not_override_explicit_target() {
    // 构造已包含显式投递配置的定时任务参数
    let mut args = serde_json::json!({
        "job_type": "agent",
        "prompt": "remind me later",
        "delivery": {
            "mode": "announce",
            "channel": "discord",
            "to": "C123"
        }
    });

    // 尝试从不同的通道上下文注入投递配置
    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", Some("-10012345"));

    // 验证显式指定的投递配置未被覆盖
    assert_eq!(args["delivery"]["channel"], "discord");
    assert_eq!(args["delivery"]["to"], "C123");
}

/// 测试跳过 shell 类型任务的投递注入
///
/// 验证对于 shell 类型的定时任务（执行 shell 命令的任务），
/// 系统不应注入投递配置，因为 shell 任务不需要消息投递通道。
///
/// # 测试步骤
///
/// 1. 创建一个 shell 类型的定时任务参数
/// 2. 调用注入函数，传入通道上下文
/// 3. 验证 delivery 字段未被添加
#[test]
fn maybe_inject_cron_add_delivery_skips_shell_jobs() {
    // 构造 shell 类型的定时任务参数
    let mut args = serde_json::json!({
        "job_type": "shell",
        "command": "echo hello"
    });

    // 尝试注入投递配置
    maybe_inject_cron_add_delivery("cron_add", &mut args, "telegram", Some("-10012345"));

    // 验证 shell 任务未添加投递配置
    assert!(args.get("delivery").is_none());
}
