use std::collections::HashMap;

use serde_json::{Value, json};
use vw_acp::{AcpSessionOptions, AuthPolicy, NonInteractivePermissionPolicy, PermissionMode};

use super::*;
use crate::app::agent::config;
use crate::app::agent::provider::provider;

fn acp_config(command: &str, args: &[&str]) -> config::schema::AcpAgentConfig {
    config::schema::AcpAgentConfig {
        command: command.to_string(),
        args: args.iter().map(|arg| (*arg).to_string()).collect(),
        env: HashMap::new(),
    }
}

fn test_model(api_id: &str) -> provider::Model {
    serde_json::from_value(json!({
        "id": api_id,
        "providerID": "test-provider",
        "api": {
            "id": api_id,
            "url": "http://localhost",
            "adapter": "acp"
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

fn custom_cfg_with_acp(name: &str, command: &str, args: &[&str]) -> config::schema::Config {
    let mut cfg = config::schema::Config::default();
    cfg.acp.insert(name.to_string(), acp_config(command, args));
    cfg
}

fn error_text(err: Error) -> String {
    err.to_string()
}

#[test]
fn normalize_acp_agent_config_leaves_non_legacy_commands_unchanged() {
    let mut cfg = acp_config(" npx ", &["@zed-industries/claude-code-acp@latest"]);
    cfg.env.insert("TOKEN".to_string(), "secret".to_string());

    let normalized = normalize_acp_agent_config("codex", &cfg);

    assert_eq!(normalized.command, cfg.command);
    assert_eq!(normalized.args, cfg.args);
    assert_eq!(normalized.env, cfg.env);
}

#[test]
fn normalize_acp_agent_config_rewrites_legacy_claude_package_and_preserves_env() {
    let mut cfg = acp_config("npx", &["@zed-industries/claude-code-acp@latest"]);
    cfg.env.insert("ANTHROPIC_AUTH_TOKEN".to_string(), "token".to_string());

    let normalized = normalize_acp_agent_config(" Claude Code ", &cfg);

    assert_eq!(normalized.env.get("ANTHROPIC_AUTH_TOKEN").map(String::as_str), Some("token"));
    assert_ne!(
        normalized.args.first().map(String::as_str),
        Some("@zed-industries/claude-code-acp@latest")
    );
    assert!(!build_acp_command_line(&normalized).contains("@zed-industries/claude-code-acp"));
}

#[test]
fn build_acp_command_line_trims_command_and_skips_blank_args() {
    let line = build_acp_command_line(&acp_config("  npx  ", &["  -y ", " ", "\t", "pkg"]));

    assert_eq!(line, "npx -y pkg");
}

#[test]
fn lookup_acp_command_prefers_explicit_agent_override() {
    let mut cfg = custom_cfg_with_acp("custom", "custom-bin", &["serve"]);
    cfg.acp.insert("model-agent".to_string(), acp_config("model-bin", &[]));

    let (name, resolved) =
        lookup_acp_command(&cfg, &test_model("model-agent"), &json!({ "acp_agent": " custom " }))
            .expect("explicit custom agent should resolve");

    assert_eq!(name, "custom");
    assert_eq!(resolved.command, "custom-bin");
    assert_eq!(resolved.args, vec!["serve".to_string()]);
}

#[test]
fn lookup_acp_command_uses_model_preference_then_default() {
    let cfg = custom_cfg_with_acp("model-agent", "model-bin", &["--stdio"]);
    let (name, resolved) = lookup_acp_command(&cfg, &test_model("model-agent"), &json!({}))
        .expect("model preferred custom agent should resolve");

    assert_eq!(name, "model-agent");
    assert_eq!(resolved.command, "model-bin");

    let (fallback_name, fallback) =
        lookup_acp_command(&config::schema::Config::default(), &test_model("unknown"), &json!({}))
            .expect("default built-in agent should resolve");
    assert_eq!(fallback_name, vw_acp::DEFAULT_AGENT_NAME);
    assert!(!fallback.command.trim().is_empty());
}

#[test]
fn lookup_acp_command_returns_none_for_unknown_explicit_agent() {
    let resolved = lookup_acp_command(
        &config::schema::Config::default(),
        &test_model("codex"),
        &json!({ "acp_agent": "missing-agent" }),
    );

    assert!(resolved.is_none());
}

#[test]
fn lookup_acp_command_ignores_blank_explicit_agent() {
    let cfg = custom_cfg_with_acp("model-agent", "model-bin", &[]);
    let (name, resolved) =
        lookup_acp_command(&cfg, &test_model("model-agent"), &json!({ "acp_agent": "   " }))
            .expect("blank explicit agent should fall back to model preference");

    assert_eq!(name, "model-agent");
    assert_eq!(resolved.command, "model-bin");
}

#[test]
fn parse_acp_options_returns_default_when_no_options_are_set() {
    let parsed = parse_acp_options(&json!({}), "codex", &acp_config("codex", &[])).expect("parse");

    assert_eq!(parsed, ParsedAcpOptions::default());
}

#[test]
fn parse_acp_options_maps_all_explicit_options() {
    let parsed = parse_acp_options(
        &json!({
            "acp_permission_mode": "deny-all",
            "acp_non_interactive_permissions": "deny",
            "acp_auth_policy": "skip",
            "acp_session_mode": "plan",
            "acp_session_model": "gpt-5-codex",
            "acp_allowed_tools": [" read_file ", "", " apply_patch "],
            "acp_max_turns": 9
        }),
        "codex",
        &acp_config("codex", &[]),
    )
    .expect("parse");

    assert_eq!(parsed.permission_mode, Some(PermissionMode::DenyAll));
    assert_eq!(parsed.non_interactive_permissions, Some(NonInteractivePermissionPolicy::Deny));
    assert_eq!(parsed.auth_policy, Some(AuthPolicy::Skip));
    assert_eq!(parsed.session_mode.as_deref(), Some("plan"));
    assert_eq!(
        parsed.session_options,
        Some(AcpSessionOptions {
            model: Some("gpt-5-codex".to_string()),
            allowed_tools: Some(vec!["read_file".to_string(), "apply_patch".to_string()]),
            max_turns: Some(9),
        })
    );
}

#[test]
fn parse_acp_options_omits_blank_string_and_empty_array_options() {
    let parsed = parse_acp_options(
        &json!({
            "acp_session_mode": "   ",
            "acp_session_model": "\t",
            "acp_allowed_tools": [" ", ""]
        }),
        "codex",
        &acp_config("codex", &[]),
    )
    .expect("parse");

    assert_eq!(parsed.session_mode, None);
    assert_eq!(parsed.session_options, None);
}

#[test]
fn parse_acp_options_accepts_signed_and_unsigned_max_turns() {
    let signed =
        parse_acp_options(&json!({ "acp_max_turns": -1 }), "codex", &acp_config("codex", &[]))
            .expect("signed integer should parse");
    assert_eq!(signed.session_options.expect("session options").max_turns, Some(-1));

    let unsigned =
        parse_acp_options(&json!({ "acp_max_turns": 3_u64 }), "codex", &acp_config("codex", &[]))
            .expect("unsigned integer should parse");
    assert_eq!(unsigned.session_options.expect("session options").max_turns, Some(3));
}

#[test]
fn parse_acp_options_rejects_bad_scalar_option_types() {
    for (payload, expected) in [
        (json!({ "acp_permission_mode": 42 }), "acp_permission_mode must be a string"),
        (json!({ "acp_permission_mode": "unsafe" }), "invalid acp_permission_mode: unsafe"),
        (json!({ "acp_session_mode": true }), "acp_session_mode must be a string"),
        (json!({ "acp_allowed_tools": "read" }), "acp_allowed_tools must be an array of strings"),
        (
            json!({ "acp_allowed_tools": ["read", 1] }),
            "acp_allowed_tools must be an array of strings",
        ),
        (json!({ "acp_max_turns": 1.5 }), "acp_max_turns must be an integer"),
    ] {
        let err = parse_acp_options(&payload, "codex", &acp_config("codex", &[]))
            .expect_err("invalid option should fail");
        assert!(error_text(err).contains(expected), "expected {expected:?} in error");
    }
}

#[test]
fn parse_acp_options_rejects_u64_values_that_do_not_fit_i64() {
    let err = parse_acp_options(
        &json!({ "acp_max_turns": u64::MAX }),
        "codex",
        &acp_config("codex", &[]),
    )
    .expect_err("oversized max turns should fail");

    assert!(error_text(err).contains("acp_max_turns is too large"));
}

#[test]
fn parse_acp_options_converts_session_config_scalar_values() {
    let parsed = parse_acp_options(
        &json!({
            "acp_session_config": {
                "blank": "   ",
                "enabled": true,
                "float": 1.25,
                "nothing": null,
                "signed": -7,
                "text": "  value  ",
                "unsigned": 8_u64
            }
        }),
        "custom",
        &acp_config("custom-agent", &[]),
    )
    .expect("scalar config values should parse");

    let config = parsed.session_config_options.into_iter().collect::<HashMap<_, _>>();
    assert_eq!(config.get("enabled").map(String::as_str), Some("true"));
    assert_eq!(config.get("float").map(String::as_str), Some("1.25"));
    assert_eq!(config.get("signed").map(String::as_str), Some("-7"));
    assert_eq!(config.get("text").map(String::as_str), Some("value"));
    assert_eq!(config.get("unsigned").map(String::as_str), Some("8"));
    assert!(!config.contains_key("blank"));
    assert!(!config.contains_key("nothing"));
}

#[test]
fn parse_acp_options_rejects_structured_session_config_values() {
    for (payload, expected) in [
        (json!({ "acp_session_config": [] }), "acp_session_config must be an object"),
        (
            json!({ "acp_session_config": { "nested": {} } }),
            "nested must be a string, number, bool, or null",
        ),
        (
            json!({ "acp_session_config": { "list": [] } }),
            "list must be a string, number, bool, or null",
        ),
    ] {
        let err = parse_acp_options(&payload, "codex", &acp_config("codex", &[]))
            .expect_err("invalid session config should fail");
        assert!(error_text(err).contains(expected));
    }
}

#[test]
fn parse_acp_options_rejects_structured_implicit_thought_level() {
    let err = parse_acp_options(
        &json!({ "thought_level": { "level": "high" } }),
        "codex",
        &acp_config("codex", &[]),
    )
    .expect_err("structured implicit config should fail");

    assert!(error_text(err).contains("thought_level must be a string, number, bool, or null"));
}

#[test]
fn parse_acp_options_normalizes_compatible_config_ids_and_deduplicates_implicit_keys() {
    let parsed = parse_acp_options(
        &json!({
            "acp_session_config": {
                "thought_level": "high"
            },
            "thought_level": "low",
            "reasoning_effort": "medium"
        }),
        "codex",
        &acp_config("codex-acp", &[]),
    )
    .expect("parse");

    assert_eq!(
        parsed.session_config_options,
        vec![("reasoning_effort".to_string(), "high".to_string())]
    );
}

#[test]
fn parse_acp_options_adds_implicit_reasoning_keys_when_explicit_config_is_absent() {
    let parsed = parse_acp_options(
        &json!({
            "reasoning_effort": "medium",
            "thought_level": "high"
        }),
        "other",
        &acp_config("other-agent", &[]),
    )
    .expect("parse");

    assert_eq!(
        parsed.session_config_options,
        vec![
            ("reasoning_effort".to_string(), "medium".to_string()),
            ("thought_level".to_string(), "high".to_string()),
        ]
    );
}

#[test]
fn parse_acp_options_converts_shared_reasoning_effort_at_acp_boundary() {
    for expected in ["none", "low", "medium", "high", "xhigh", "max"] {
        let parsed = parse_acp_options(
            &json!({ "reasoning_effort": expected }),
            "codex",
            &acp_config("codex", &[]),
        )
        .expect("shared reasoning effort should parse");

        assert_eq!(
            parsed.session_config_options,
            vec![("reasoning_effort".to_string(), expected.to_string())]
        );
    }

    let error = parse_acp_options(
        &json!({ "reasoning_effort": "extreme" }),
        "codex",
        &acp_config("codex", &[]),
    )
    .expect_err("unsupported reasoning effort must fail");
    assert!(error.to_string().contains("reasoning_effort must be a supported string"));
}

#[test]
fn parse_acp_options_allows_non_object_root_values() {
    for value in [Value::Null, json!("text"), json!([1, 2, 3])] {
        let parsed = parse_acp_options(&value, "codex", &acp_config("codex", &[]))
            .expect("missing object keys should behave like no options");
        assert_eq!(parsed, ParsedAcpOptions::default());
    }
}
