//! 压缩溢出预算及 token 用量选择。

use crate::compaction::is_overflow;
use crate::message;
use model_types as provider;
use std::collections::HashMap;

fn capability_io(text: bool) -> provider::CapabilityIO {
    provider::CapabilityIO { text, audio: false, image: false, video: false, pdf: false }
}

fn model_with_limits(context: u64, input: Option<u64>, output: u64) -> provider::Model {
    provider::Model {
        id: "model-fixture".to_string(),
        provider_id: "provider-fixture".to_string(),
        api: provider::ApiInfo {
            protocols: None,
            id: "api-fixture".to_string(),
            url: "https://example.test/v1".to_string(),
            adapter: provider::default_adapter(),
        },
        name: "Fixture Model".to_string(),
        family: None,
        capabilities: provider::Capabilities {
            native_tool_search: false,
            catalog_execution: None,
            temperature: true,
            reasoning: false,
            reasoning_options: None,
            attachment: false,
            toolcall: true,
            input: capability_io(true),
            output: capability_io(true),
            interleaved: provider::InterleavedCapability::Bool(false),
        },
        cost: provider::ModelCost {
            input: 0.0,
            output: 0.0,
            cache: provider::ModelCostCache { read: 0.0, write: 0.0 },
            experimental_over_200k: None,
        },
        limit: provider::ModelLimit { context, input, output },
        status: "stable".to_string(),
        options: HashMap::new(),
        headers: HashMap::new(),
        release_date: "2026-01-01".to_string(),
        variants: HashMap::new(),
    }
}

fn token_info(
    total: Option<i64>,
    input: i64,
    output: i64,
    read: i64,
    write: i64,
) -> message::TokenInfo {
    message::TokenInfo {
        total,
        input,
        output,
        reasoning: 0,
        cache: message::TokenCacheInfo { read, write },
    }
}

#[tokio::test]
async fn overflow_uses_fallback_for_unknown_and_respects_explicit_input() {
    let tokens = token_info(Some(1_000_000), 0, 0, 0, 0);

    assert!(is_overflow(&tokens, &model_with_limits(0, None, 0)).await);
    assert!(is_overflow(&tokens, &model_with_limits(10_000, Some(10), 50_000)).await);
}

#[tokio::test]
async fn overflow_uses_total_when_present_and_rounds_threshold() {
    let model = model_with_limits(100_000, None, 10_000);

    assert!(!is_overflow(&token_info(Some(89_999), 99_999, 0, 0, 0), &model).await);
    assert!(is_overflow(&token_info(Some(90_000), 1, 1, 1, 1), &model).await);
}

#[tokio::test]
async fn overflow_falls_back_to_component_sum_and_input_limit() {
    let model = model_with_limits(200_000, Some(100_000), 20_000);

    assert!(!is_overflow(&token_info(None, 90_000, 9_000, 900, 99), &model).await);
    assert!(is_overflow(&token_info(None, 90_000, 9_000, 900, 100), &model).await);
}

#[tokio::test]
async fn overflow_treats_negative_counts_as_zero() {
    let model = model_with_limits(100_000, None, 10_000);

    assert!(!is_overflow(&token_info(Some(-1), 100_000, 100_000, 0, 0), &model).await);
    assert!(!is_overflow(&token_info(None, -10, -20, -30, -40), &model).await);
}
