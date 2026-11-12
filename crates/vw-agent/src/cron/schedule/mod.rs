//! 调度工具模块
//!
//! 本模块提供与调度相关的辅助函数，用于计算、验证和处理调度表达式。
//!
//! ## 主要功能
//!
//! - 计算调度的下一次运行时间（支持 cron、一次性、周期性三种模式）
//! - 验证调度配置的有效性
//! - 规范化 cron 表达式格式
//! - 提取 cron 表达式字符串
//!
//! ## 支持的调度类型
//!
//! - **Cron 表达式**: 标准的 cron 表达式，支持时区配置
//! - **一次性执行**: 指定具体的执行时间点
//! - **周期性执行**: 按固定的毫秒间隔循环执行

mod biweekly;

use super::Schedule;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use cron::Schedule as CronExprSchedule;
use std::str::FromStr;

/// 计算给定调度的下一次运行时间
///
/// 根据不同的调度类型（Cron、At、Every），计算从指定时间点开始的下一次执行时间。
///
/// # 参数
///
/// - `schedule`: 调度配置对象，可以是 Cron 表达式、一次性时间或周期性间隔
/// - `from`: 起始时间点，从这个时间开始计算下一次运行时间
///
/// # 返回值
///
/// 返回 `Result<DateTime<Utc>>`，表示下一次运行时间（UTC 时区）
///
/// # 错误
///
/// 在以下情况会返回错误：
/// - Cron 表达式格式无效
/// - 指定的时区名称无效（IANA 时区）
/// - Cron 表达式没有未来的执行时间点
/// - every_ms 值为 0 或溢出
///
/// # 示例
///
/// ```rust,ignore
/// use chrono::Utc;
/// let schedule = Schedule::Every { every_ms: 60000 };
/// let next = next_run_for_schedule(&schedule, Utc::now())?;
/// ```
pub fn next_run_for_schedule(schedule: &Schedule, from: DateTime<Utc>) -> Result<DateTime<Utc>> {
    match schedule {
        Schedule::Cron { expr, tz } => {
            // 规范化 cron 表达式，确保字段数量正确
            let normalized = normalize_expression(expr)?;

            // 解析 cron 表达式
            let cron = CronExprSchedule::from_str(&normalized)
                .with_context(|| format!("Invalid cron expression: {expr}"))?;

            // 如果指定了时区，需要在该时区下计算下一次执行时间
            if let Some(tz_name) = tz {
                // 解析 IANA 时区名称
                let timezone = chrono_tz::Tz::from_str(tz_name)
                    .with_context(|| format!("Invalid IANA timezone: {tz_name}"))?;

                // 将 UTC 时间转换为指定时区的本地时间
                let localized_from = from.with_timezone(&timezone);

                // 在指定时区下计算下一次执行时间
                let next_local = cron.after(&localized_from).next().ok_or_else(|| {
                    anyhow::anyhow!("No future occurrence for expression: {expr}")
                })?;

                // 将本地时间转换回 UTC 时区返回
                Ok(next_local.with_timezone(&Utc))
            } else {
                // 未指定时区时，直接使用 UTC 时间计算
                cron.after(&from)
                    .next()
                    .ok_or_else(|| anyhow::anyhow!("No future occurrence for expression: {expr}"))
            }
        }
        Schedule::Biweekly { anchor_date, weekdays, hour, minute, timezone } => {
            biweekly::next_run(*anchor_date, weekdays, *hour, *minute, timezone, from)
        }
        Schedule::At { at } => {
            // 一次性执行：直接返回指定的时间点
            Ok(*at)
        }
        Schedule::Every { every_ms } => {
            // 周期性执行：验证间隔值的有效性
            if *every_ms == 0 {
                anyhow::bail!("Invalid schedule: every_ms must be > 0");
            }

            // 将 u64 毫秒值转换为 i64（用于 chrono 计算）
            let ms = i64::try_from(*every_ms).context("every_ms is too large")?;
            let delta = ChronoDuration::milliseconds(ms);

            // 计算下一次执行时间，防止溢出
            from.checked_add_signed(delta)
                .ok_or_else(|| anyhow::anyhow!("every_ms overflowed DateTime"))
        }
    }
}

/// 验证调度配置的有效性
///
/// 检查调度配置是否合法，包括时间表达式的语法正确性和逻辑合理性。
///
/// # 参数
///
/// - `schedule`: 需要验证的调度配置对象
/// - `now`: 当前时间，用于验证一次性执行时间是否在未来
///
/// # 返回值
///
/// 返回 `Result<()>`，验证通过返回 `Ok(())`
///
/// # 错误
///
/// 在以下情况会返回错误：
/// - Cron 表达式格式无效或无法计算出下一次执行时间
/// - 一次性执行时间不在未来（小于等于当前时间）
/// - 周期性执行间隔为 0
///
/// # 示例
///
/// ```rust,ignore
/// use chrono::Utc;
/// let schedule = Schedule::Every { every_ms: 1000 };
/// validate_schedule(&schedule, Utc::now())?; // 验证通过
/// ```
pub fn validate_schedule(schedule: &Schedule, now: DateTime<Utc>) -> Result<()> {
    match schedule {
        Schedule::Cron { expr, .. } => {
            // 验证 cron 表达式格式
            let _ = normalize_expression(expr)?;
            // 验证能否计算出下一次执行时间
            let _ = next_run_for_schedule(schedule, now)?;
            Ok(())
        }
        Schedule::Biweekly { .. } => {
            next_run_for_schedule(schedule, now)?;
            Ok(())
        }
        Schedule::At { at } => {
            // 一次性执行时间必须在未来
            if *at <= now {
                anyhow::bail!("Invalid schedule: 'at' must be in the future");
            }
            Ok(())
        }
        Schedule::Every { every_ms } => {
            // 周期性执行间隔必须大于 0
            if *every_ms == 0 {
                anyhow::bail!("Invalid schedule: every_ms must be > 0");
            }
            Ok(())
        }
    }
}

/// 提取调度配置中的 cron 表达式
///
/// 如果调度类型是 Cron 表达式，则返回该表达式字符串；否则返回 None。
///
/// # 参数
///
/// - `schedule`: 调度配置对象
///
/// # 返回值
///
/// 返回 `Option<String>`：
/// - `Some(String)`: 如果调度类型是 Cron 表达式
/// - `None`: 如果调度类型是 At 或 Every
///
/// # 示例
///
/// ```rust,ignore
/// let schedule = Schedule::Cron { expr: "0 0 * * *".to_string(), tz: None };
/// assert_eq!(schedule_cron_expression(&schedule), Some("0 0 * * *".to_string()));
///
/// let schedule = Schedule::Every { every_ms: 60000 };
/// assert_eq!(schedule_cron_expression(&schedule), None);
/// ```
pub fn schedule_cron_expression(schedule: &Schedule) -> Option<String> {
    match schedule {
        Schedule::Cron { expr, .. } => Some(expr.clone()),
        _ => None,
    }
}

/// 规范化 cron 表达式格式
///
/// 将不同格式的 cron 表达式转换为统一的 6 字段或 7 字段格式（包含秒）。
///
/// # 参数
///
/// - `expression`: 原始的 cron 表达式字符串
///
/// # 返回值
///
/// 返回 `Result<String>`，包含规范化后的 cron 表达式
///
/// # 错误
///
/// 如果表达式字段数量不是 5、6 或 7，则返回错误
///
/// # 转换规则
///
/// - **5 字段**: 标准的 crontab 语法（分 时 日 月 周），自动在前面添加 `0`（秒字段）
/// - **6 字段**: crate 原生语法（秒 分 时 日 月 周），直接返回
/// - **7 字段**: 完整语法（秒 分 时 日 月 周 年），直接返回
///
/// # 示例
///
/// ```rust,ignore
/// // 5 字段转换为 6 字段
/// assert_eq!(normalize_expression("0 * * * *")?, "0 0 * * * *");
///
/// // 6 字段保持不变
/// assert_eq!(normalize_expression("0 0 * * * *")?, "0 0 * * * *");
/// ```
pub fn normalize_expression(expression: &str) -> Result<String> {
    // 去除表达式首尾的空白字符
    let expression = expression.trim();

    // 统计表达式中的字段数量
    let field_count = expression.split_whitespace().count();

    match field_count {
        // 标准的 crontab 语法（5 字段）：分 时 日 月 周
        // 在前面添加秒字段 0，转换为 6 字段格式
        5 => Ok(format!("0 {expression}")),

        // crate 原生语法（6 字段或 7 字段）：包含秒（+ 可选的年）
        // 直接返回，无需转换
        6 | 7 => Ok(expression.to_string()),

        // 字段数量不合法，返回错误
        _ => anyhow::bail!(
            "Invalid cron expression: {expression} (expected 5, 6, or 7 fields, got {field_count})"
        ),
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
mod biweekly_tests;
