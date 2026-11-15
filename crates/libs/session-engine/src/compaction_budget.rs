//! Pure input and summary budgets shared by session compaction.
use model_types::ModelLimit;
pub const AUTO_COMPACTION_THRESHOLD_PERCENT: u64 = 90;

/// 计算可用于当前请求输入的窗口。
///
/// 显式输入限制优先；没有输入限制时，从总窗口中预留最大输出空间。
pub fn effective_input_window(limit: &ModelLimit) -> u64 {
    model_state::effective_input_window(limit)
}

/// Unknown windows use the existing runtime fallback, not a claimed provider capability.
pub fn compaction_context_window(limit: &ModelLimit) -> u64 {
    if limit.context > 0 {
        limit.context
    } else {
        limit
            .input
            .filter(|input| *input > 0)
            .unwrap_or(model_state::DEFAULT_MODEL_CONTEXT_LIMIT)
    }
}

/// Reserve the summary's request allocation, never the model's entire output capacity.
pub fn summary_output_budget(limit: &ModelLimit) -> u64 {
    let output = if limit.output > 0 {
        limit.output
    } else {
        model_state::DEFAULT_MODEL_OUTPUT_LIMIT
    };
    output.min(20_000).min((compaction_context_window(limit) / 4).max(1))
}

pub fn compaction_input_window(limit: &ModelLimit) -> u64 {
    let context = compaction_context_window(limit);
    let input = limit.input.filter(|input| *input > 0).unwrap_or(context);
    // An input-only limit does not describe a combined input/output window.
    if limit.context == 0 && limit.input.is_some_and(|input| input > 0) {
        input
    } else {
        input.min(context.saturating_sub(summary_output_budget(limit)))
    }
}

/// 判断当前输入是否达到自动压缩边界。
pub fn should_compact(input_tokens: u64, limit: &ModelLimit) -> bool {
    let window = compaction_input_window(limit);
    if window == 0 {
        return input_tokens > 0;
    }

    let total = compaction_context_window(limit);
    let threshold = ((u128::from(total) * u128::from(AUTO_COMPACTION_THRESHOLD_PERCENT))
        .div_ceil(100) as u64)
        .min(window);
    input_tokens >= threshold
}
