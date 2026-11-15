//! 使用率可视化模块
//!
//! 本模块提供 token 使用率的可视化组件，包括：
//! - 圆环进度条，显示当前上下文使用百分比
//! - 使用率计算函数，从应用状态中提取并计算使用统计
//!
//! 主要用于输入面板中显示当前会话的 token 消耗情况。

use crate::app::models::{ChatSessionStep, TokenUsage};
use crate::app::App;

/// 新步骤的默认零值表示尚未收到用量；沿用本会话最近的有效统计。
/// 按时间顺序查找，允许压缩后的实际用量下降。
pub(crate) fn latest_reported_usage(steps: &[ChatSessionStep]) -> Option<&TokenUsage> {
    steps.iter().rev().find(|step| step.usage.input_tokens > 0).map(|step| &step.usage)
}

/// 计算当前上下文使用率百分比
///
/// 根据活跃会话的最近有效步骤的输入 token 数与模型输入窗口限制，
/// 计算当前上下文使用百分比。
///
/// # 参数
///
/// - `app`: 应用状态引用，包含会话信息和模型使用配置
///
/// # 返回
///
/// 返回使用率百分比（0.0 - 100.0），如果无法计算则返回 0.0
///
/// # 计算逻辑
///
/// 1. 从活跃会话的最近有效步骤获取输入 token 数
/// 2. 获取模型的有效输入窗口
/// 3. 计算 (input_tokens / input_limit) * 100
pub fn get_usage_rate_percent(app: &App) -> f32 {
    // 尝试从活跃会话获取最近有效步骤的输入 token 数
    if let Some(info) = &app.usage_model_info {
        let last_step_input_tokens = latest_reported_usage(&app.active_session_view_state.steps)
            .map(|usage| usage.input_tokens)
            .unwrap_or(0);
        // 防止除以零
        if info.input_limit == 0 {
            return 0.0;
        }
        // 计算使用率百分比
        ((last_step_input_tokens as f64) * 100.0 / (info.input_limit as f64)) as f32
    } else {
        0.0
    }
}

/// 获取详细的使用统计信息
///
/// 从应用状态中提取完整的使用数据，包括 token 数量和估算费用。
///
/// # 参数
///
/// - `app`: 应用状态引用，包含会话和模型使用信息
///
/// # 返回
///
/// 返回元组 (最后步骤输入tokens, 上下文限制, 估算费用, 总token数):
/// - `last_step_input_tokens`: 活跃会话最近有效步骤的输入 token 数
/// - `context_limit`: 模型的上下文窗口限制
/// - `estimated_cost`: 基于 token 使用量的估算费用（美元）
/// - `total_tokens`: 所有类型 token 的总和（输入+输出+缓存+推理）
///
/// # 费用计算
///
/// 估算费用使用输入和输出 token 价格的平均值计算：
/// `estimated_cost = (total_tokens / 1,000,000) * ((input_price + output_price) / 2)`
pub fn get_usage_details(app: &App) -> (i64, i64, f64, i64) {
    // 获取活跃会话最近有效步骤的输入 token 数
    let last_step_input_tokens = latest_reported_usage(&app.active_session_view_state.steps)
        .map(|usage| usage.input_tokens)
        .unwrap_or(0);

    // 获取模型的上下文限制
    let context_limit = app.usage_model_info.as_ref().map(|i| i.context_limit as i64).unwrap_or(0);

    // 获取模型的每百万 token 价格（输入和输出）
    let (cost_input, cost_output) = app
        .usage_model_info
        .as_ref()
        .map(|info| (info.cost_input_per_million, info.cost_output_per_million))
        .unwrap_or((0.0, 0.0));

    // 计算所有类型 token 的总和
    let total_tokens = app.usage.input_tokens
        + app.usage.output_tokens
        + app.usage.cached_tokens
        + app.usage.reasoning_tokens;

    // 估算费用：使用平均价格
    let estimated_cost = (total_tokens as f64 / 1_000_000.0) * ((cost_input + cost_output) / 2.0);

    (last_step_input_tokens, context_limit, estimated_cost, total_tokens)
}

/// 使用率圆环组件
///
/// 一个基于 Canvas 的圆环进度条，用于可视化显示使用率百分比。
/// 根据使用率的不同区间显示不同的颜色：
/// - 0% - 70%: 绿色 (#2EC27E) - 正常
/// - 70% - 90%: 橙色 (#F5A623) - 警告
/// - 90% - 100%: 红色 (#E53E3E) - 危险
///
/// # 示例
///
/// ```ignore
/// let ring = UsageRing { percent: 75.0 };
/// // 在 Canvas 中使用
/// ```
pub use desktop_chat_view::header_usage::UsageRing;
