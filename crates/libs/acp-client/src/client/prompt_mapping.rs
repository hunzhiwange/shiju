//! ACP prompt 响应字段映射。

use super::*;

/// 将 ACP 结束原因映射为 VibeWindow 内部提示词结束原因。
pub(super) fn acp_finish_reason(reason: acp::StopReason) -> String {
    match reason {
        acp::StopReason::MaxTokens => "length".to_string(),
        acp::StopReason::MaxTurnRequests => "max_turn_requests".to_string(),
        acp::StopReason::Refusal => "refusal".to_string(),
        acp::StopReason::Cancelled => "cancelled".to_string(),
        _ => "stop".to_string(),
    }
}

/// 将 ACP 用量结构映射为内部用量结构。
pub(super) fn map_usage(usage: &acp::Usage) -> PromptUsage {
    PromptUsage {
        input_tokens: usage.input_tokens as i64,
        output_tokens: usage.output_tokens as i64,
        cached_tokens: usage.cached_read_tokens.unwrap_or_default() as i64,
        reasoning_tokens: usage.thought_tokens.unwrap_or_default() as i64,
    }
}
