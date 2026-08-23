//! 验证协议停止原因与用量统计到运行时类型的稳定映射。

use agent_client_protocol as acp;

use crate::types::PromptUsage;

use super::prompt_mapping::{acp_finish_reason, map_usage};

#[test]
fn acp_finish_reason_maps_protocol_reasons() {
    assert_eq!(acp_finish_reason(acp::StopReason::MaxTokens), "length");
    assert_eq!(acp_finish_reason(acp::StopReason::MaxTurnRequests), "max_turn_requests");
    assert_eq!(acp_finish_reason(acp::StopReason::Refusal), "refusal");
    assert_eq!(acp_finish_reason(acp::StopReason::Cancelled), "cancelled");
}

#[test]
fn acp_finish_reason_defaults_end_turn_to_stop() {
    assert_eq!(acp_finish_reason(acp::StopReason::EndTurn), "stop");
}

#[test]
fn map_usage_preserves_token_counts() {
    let mut usage = acp::Usage::new(15, 10, 5);
    usage.cached_read_tokens = Some(3);
    usage.thought_tokens = Some(2);

    assert_eq!(
        map_usage(&usage),
        PromptUsage { input_tokens: 10, output_tokens: 5, cached_tokens: 3, reasoning_tokens: 2 }
    );
}

#[test]
fn map_usage_defaults_missing_optional_counts_to_zero() {
    let usage = acp::Usage::new(15, 10, 5);

    assert_eq!(
        map_usage(&usage),
        PromptUsage { input_tokens: 10, output_tokens: 5, cached_tokens: 0, reasoning_tokens: 0 }
    );
}
