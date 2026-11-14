use vw_agent_apps::usage::model_names::{
    is_placeholder_pricing_model, model_pricing_candidates, should_try_pricing_prefix_match,
};

#[test]
fn usage17_model_names_exact_model_precedes_normalized_fallback() {
    let original = " OpenAI/GPT-5-2025-08-07-high ";
    let candidates = model_pricing_candidates(original);
    assert_eq!(candidates[0], "openai/gpt-5-2025-08-07-high");
    assert_eq!(candidates[1], "gpt-5-2025-08-07-high");
    let dated = candidates.iter().position(|s| s == "gpt-5-2025-08-07").unwrap();
    let base = candidates.iter().position(|s| s == "gpt-5").unwrap();
    assert!(dated < base);
    assert_eq!(original, " OpenAI/GPT-5-2025-08-07-high ");
}

#[test]
fn usage17_model_names_bedrock_versions_and_claude_dots_fall_back() {
    let candidates = model_pricing_candidates("us.anthropic.claude-sonnet-4.5-20250929-v1:0");
    assert_eq!(candidates[0], "us.anthropic.claude-sonnet-4.5-20250929-v1:0");
    assert!(candidates.iter().any(|s| s == "claude-sonnet-4-5"));
    assert!(candidates.iter().any(|s| s == "claude-sonnet-4.5"));
}

#[test]
fn usage17_model_names_upstream_aliases_and_effort_suffixes_are_supported() {
    for (input, expected) in [
        ("google.gemini-2.5-pro", "gemini-2.5-pro"),
        ("claude-deepseek-v3", "deepseek-v3"),
        ("gemini-pro@20250514", "gemini-pro"),
        ("claude-sonnet-4-5[1m]", "claude-sonnet-4-5"),
        ("doubao-seed-2-1-pro-260628", "doubao-seed-2-1-pro"),
        ("模型-2026-05-14", "模型"),
    ] {
        assert!(model_pricing_candidates(input).iter().any(|s| s == expected), "{input}");
    }
    for effort in ["minimal", "low", "medium", "high", "xhigh"] {
        assert_eq!(model_pricing_candidates(&format!("gpt-5-{effort}")),
                   vec![format!("gpt-5-{effort}"), "gpt-5".to_owned()]);
    }
    assert_eq!(model_pricing_candidates("widget-209900"), vec!["widget-209900"]);
    assert_eq!(model_pricing_candidates("abc🚀12345678"), vec!["abc🚀12345678"]);
}

#[test]
fn usage17_model_names_placeholders_never_produce_pricing_candidates() {
    for input in ["", " ", "UNKNOWN", " null ", "None", "provider/unknown", "unknown:0"] {
        assert!(model_pricing_candidates(input).is_empty(), "{input}");
    }
    assert!(is_placeholder_pricing_model(" Unknown "));
    assert!(!is_placeholder_pricing_model("gpt-5"));
}

#[test]
fn usage17_model_names_prefix_fallback_requires_specific_family() {
    for model in ["claude-sonnet-4-5", "gpt-5-mini", "gemini-2.5-pro", "o3-mini"] {
        assert!(should_try_pricing_prefix_match(model));
    }
    for model in ["claude", "claude-sonnet", "gpt-5", "unknown", "custom-model-1"] {
        assert!(!should_try_pricing_prefix_match(model));
    }
}
