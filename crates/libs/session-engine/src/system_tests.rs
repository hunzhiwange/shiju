//! Local environment formatting and model prompt contracts.

use super::*;
use serde_json::json;

fn test_model(api_id: &str, provider_id: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": api_id,
        "providerID": provider_id,
        "api": {
            "id": api_id,
            "url": "http://localhost",
            "adapter": "openai-compatible"
        },
        "name": api_id,
        "family": null,
        "capabilities": {
            "temperature": true,
            "reasoning": true,
            "attachment": false,
            "toolcall": true,
            "input": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "output": {
                "text": true,
                "audio": false,
                "image": false,
                "video": false,
                "pdf": false
            },
            "interleaved": false
        },
        "cost": {
            "input": 0.0,
            "output": 0.0,
            "cache": {
                "read": 0.0,
                "write": 0.0
            },
            "experimental_over_200k": null
        },
        "limit": {
            "context": 8192,
            "input": null,
            "output": 4096
        },
        "status": "active",
        "options": {},
        "headers": {},
        "release_date": "2026-01-01",
        "variants": {}
    }))
    .expect("test model should deserialize")
}

#[test]
fn yes_no_en_is_stable() {
    assert_eq!(yes_no_en(true), "Yes");
    assert_eq!(yes_no_en(false), "No");
}

#[test]
fn environment_text_contains_core_fields() {
    let text = environment_text("gpt-5", "openai", "/tmp/work", true, "no extra dirs");
    assert!(text.contains("gpt-5"));
    assert!(text.contains("openai/gpt-5"));
    assert!(text.contains("/tmp/work"));
    assert!(text.contains("Yes"));
    assert!(text.contains(platform()));
}

#[test]
fn provider_returns_unified_prompt_for_all_models() {
    let models = [
        ("gpt-5-mini", "openai"),
        ("gpt-4.1", "openai"),
        ("o3-mini", "openai"),
        ("gemini-2.5-pro", "google"),
        ("claude-sonnet-4", "anthropic"),
        ("Trinity-v1", "local"),
        ("qwen-plus", "qwen"),
        ("gpt-5-claude-gemini-Trinity", "mixed"),
        ("gpt-4-claude-gemini-Trinity", "mixed"),
        ("unknown-model", "custom"),
    ];

    for (api_id, provider_id) in models {
        let prompts = provider(&test_model(api_id, provider_id), "");
        assert_eq!(prompts.len(), 1);
        assert!(prompts[0].contains(UNIFIED_PROMPT_MARKER));
        assert!(prompts[0].contains(&format!("{provider_id}/{api_id}")));
        assert!(!prompts[0].contains("{{"));
    }
}

#[test]
fn instructions_render_unified_prompt_defaults() {
    let instructions = instructions("");

    assert!(instructions.contains(UNIFIED_PROMPT_MARKER));
    assert!(instructions.contains("`unknown`"));
    assert!(!instructions.contains("{{"));
    assert!(!instructions.starts_with('\n'));
    assert!(!instructions.ends_with('\n'));
}

#[test]
fn environment_text_preserves_structure_for_non_git_workspace() {
    let text = environment_text("model", "provider", "/workspace", false, "");
    let lines = text.lines().collect::<Vec<_>>();

    assert_eq!(lines.len(), 11);
    assert_eq!(lines[0], "You are using the model model. Exact model ID: provider/model");
    assert_eq!(lines[1], "Here is some information about the runtime environment:");
    assert_eq!(lines[2], "<env>");
    assert_eq!(lines[3], "  Working directory: /workspace");
    assert_eq!(lines[4], "  Is Git repository: No");
    assert_eq!(lines[5], format!("  Platform: {}", platform()));
    assert!(lines[6].starts_with("  Today's date: "));
    assert_eq!(lines[7], "</env>");
    assert_eq!(lines[8], "<directories>");
    assert_eq!(lines[9], "  ");
    assert_eq!(lines[10], "</directories>");
}

#[test]
fn today_date_string_uses_documented_four_component_format() {
    let date = today_date_string();
    let components = date.split_whitespace().collect::<Vec<_>>();

    assert_eq!(components.len(), 4);
    assert_eq!(components[0].len(), 3);
    assert_eq!(components[1].len(), 3);
    assert_eq!(components[3].len(), 4);
}
