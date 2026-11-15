//! 定时任务自动投递配置注入模块
//!
//! 本模块负责在代理执行循环中自动为定时任务（cron）注入投递配置，
//! 使得代理创建的定时任务能够自动将结果投递回原始通道。
//!
//! # 主要功能
//!
//! - 识别 `cron_add` 工具调用并判断是否需要自动注入投递配置
//! - 仅对代理类型任务（agent jobs）进行自动配置
//! - 尊重用户显式指定的投递模式，仅在未设置或为 none 时自动填充
//!
//! # 支持的通道
//!
//! 目前支持自动投递的通道包括：telegram、discord、slack、mattermost

/// 支持自动定时任务投递的通道列表
///
/// 仅当定时任务通过这些通道创建时，系统才会自动注入投递配置。
/// 其他通道将不会触发自动注入逻辑。
///
/// # 支持的通道
///
/// - `telegram`：Telegram 消息通道
/// - `discord`：Discord 消息通道
/// - `slack`：Slack 消息通道
/// - `mattermost`：Mattermost 消息通道
pub const AUTO_CRON_DELIVERY_CHANNELS: &[&str] =
    &["telegram", "discord", "slack", "mattermost"];

#[cfg(test)]
#[path = "cron_tests.rs"]
mod cron_tests;

/// 尝试为 `cron_add` 工具调用自动注入投递配置
///
/// 当用户通过支持自动投递的通道创建代理类型的定时任务时，
/// 此函数会自动在工具参数中填充投递配置，使任务结果能够
/// 自动投递回原始通道。
///
/// # 参数
///
/// - `tool_name`：工具名称，仅当为 `"cron_add"` 时才会触发注入逻辑
/// - `tool_args`：工具参数的可变引用，函数会在此对象中注入投递配置
/// - `channel_name`：当前通道名称，必须是 [`AUTO_CRON_DELIVERY_CHANNELS`] 之一才会触发注入
/// - `reply_target`：回复目标地址（如频道 ID、用户 ID 等），用于指定投递目标
///
/// # 行为说明
///
/// ## 触发条件
///
/// 函数仅在以下条件全部满足时才会执行注入：
/// 1. 工具名称为 `"cron_add"`
/// 2. 当前通道在支持自动投递的通道列表中
/// 3. 提供了非空的 `reply_target`
/// 4. 任务类型为代理任务（agent job）
///
/// ## 任务类型判断
///
/// 代理任务的判断逻辑如下：
/// - 如果 `job_type` 参数为 `"agent"`，则判定为代理任务
/// - 如果未指定 `job_type` 但存在 `prompt` 参数，也判定为代理任务
/// - 其他情况不触发自动注入
///
/// ## 投递配置注入规则
///
/// - **mode**：仅在未设置或为 `"none"` 或空字符串时，自动设置为 `"announce"`
///   - 如果用户已显式设置为其他模式（如 `"silent"`），则保持不变
/// - **channel**：仅在未设置或为空时，自动填充为当前通道名称
/// - **to**：仅在未设置或为空时，自动填充为 `reply_target`
///
/// # 示例
///
/// ```ignore
/// use serde_json::json;
///
/// let mut args = json!({
///     "prompt": "每小时检查一次天气",
///     "cron": "0 * * * *"
/// });
///
/// maybe_inject_cron_add_delivery(
///     "cron_add",
///     &mut args,
///     "telegram",
///     Some("@weather_group")
/// );
///
/// // args 现在包含自动注入的 delivery 配置：
/// // {
/// //     "prompt": "每小时检查一次天气",
/// //     "cron": "0 * * * *",
/// //     "delivery": {
/// //         "mode": "announce",
/// //         "channel": "telegram",
/// //         "to": "@weather_group"
/// //     }
/// // }
/// ```
///
/// # 不触发注入的情况
///
/// - 工具名称不是 `"cron_add"`
/// - 通道不在支持列表中（如 `"web"` 通道）
/// - `reply_target` 为 `None` 或空字符串
/// - 任务类型不是代理任务（如 `job_type: "shell"`）
/// - 用户已显式设置了非 announce 的投递模式
pub fn maybe_inject_cron_add_delivery(
    tool_name: &str,
    tool_args: &mut serde_json::Value,
    channel_name: &str,
    reply_target: Option<&str>,
) {
    // 检查是否为 cron_add 工具，以及当前通道是否支持自动投递
    if tool_name != "cron_add"
        || !AUTO_CRON_DELIVERY_CHANNELS.iter().any(|supported| supported == &channel_name)
    {
        return;
    }

    // 检查 reply_target 是否存在且非空，无目标则无法投递
    let Some(reply_target) = reply_target.map(str::trim).filter(|v| !v.is_empty()) else {
        return;
    };

    // 尝试获取参数对象的可变引用
    let Some(args_obj) = tool_args.as_object_mut() else {
        return;
    };

    // 判断是否为代理类型任务
    // 优先检查 job_type 参数，如果未指定则通过是否存在 prompt 参数来判断
    let is_agent_job = match args_obj.get("job_type").and_then(serde_json::Value::as_str) {
        Some("agent") => true,                   // 显式指定为 agent 类型
        Some(_) => false,                        // 显式指定为其他类型（如 shell、http 等）
        None => args_obj.contains_key("prompt"), // 未指定类型，通过 prompt 存在与否判断
    };

    // 仅对代理任务进行自动注入
    if !is_agent_job {
        return;
    }

    // 获取或创建 delivery 对象
    let delivery = args_obj.entry("delivery".to_string()).or_insert_with(|| serde_json::json!({}));
    let Some(delivery_obj) = delivery.as_object_mut() else {
        return;
    };

    // 处理投递模式
    // 如果未设置或为 none/空，则自动设置为 announce
    // 如果用户已显式设置其他模式，则保持不变（尊重用户选择）
    let mode = delivery_obj.get("mode").and_then(serde_json::Value::as_str).unwrap_or("none");
    if mode.eq_ignore_ascii_case("none") || mode.trim().is_empty() {
        delivery_obj.insert("mode".to_string(), serde_json::Value::String("announce".to_string()));
    } else if !mode.eq_ignore_ascii_case("announce") {
        // 用户已显式选择非 announce 模式，保持不变
        return;
    }

    // 自动填充通道名称（如果未设置）
    let needs_channel = delivery_obj
        .get("channel")
        .and_then(serde_json::Value::as_str)
        .is_none_or(|value| value.trim().is_empty());
    if needs_channel {
        delivery_obj
            .insert("channel".to_string(), serde_json::Value::String(channel_name.to_string()));
    }

    // 自动填充投递目标（如果未设置）
    let needs_target = delivery_obj
        .get("to")
        .and_then(serde_json::Value::as_str)
        .is_none_or(|value| value.trim().is_empty());
    if needs_target {
        delivery_obj.insert("to".to_string(), serde_json::Value::String(reply_target.to_string()));
    }
}
