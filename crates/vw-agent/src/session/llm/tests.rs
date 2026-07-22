//! LLM 会话适配层的行为回归测试。
//!
//! 这些测试覆盖 ACP 配置解析、历史重放、会话命名和消息提取等边界。测试只构造本地数据，
//! 不启动真实 ACP 代理，便于快速验证请求前后的纯函数行为。

use super::acp::{
    AcpReplayStrategy, ParsedAcpOptions, acp_session_name, build_acp_command_line,
    build_replay_prompt, extract_delta_from_acp_message, extract_reasoning_delta_from_acp_message,
    extract_tool_call_from_acp_message, is_acp_session_changed_message, lookup_acp_command,
    missing_session_error, normalize_acp_agent_config, parse_acp_options, parse_replay_strategy,
};
use serde_json::json;
use vw_acp::{AcpSessionOptions, AuthPolicy, NonInteractivePermissionPolicy, PermissionMode};

use crate::app::agent::config;
use crate::app::agent::provider::provider;

fn test_model(api_id: &str) -> provider::Model {
    // 使用完整模型 JSON fixture，确保解析路径和真实配置结构保持一致。
    serde_json::from_value(json!({
        "id": api_id,
        "providerID": "test-provider",
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
fn discard_replay_strategy_is_the_default_and_can_be_selected() {
    assert_eq!(
        parse_replay_strategy(&json!({ "acp_history_strategy": "discard" })),
        AcpReplayStrategy::Discard
    );
    assert_eq!(parse_replay_strategy(&json!({})), AcpReplayStrategy::Discard);
}

#[test]
fn discard_replay_prompt_omits_old_history() {
    let prompt = build_replay_prompt(
        &json!([
            { "role": "system", "content": "Follow repo rules." },
            { "role": "user", "content": "first request" },
            { "role": "assistant", "content": "first answer" },
            { "role": "user", "content": "current request" }
        ]),
        AcpReplayStrategy::Discard,
        3,
    );

    assert!(prompt.contains("current request"));
    assert!(prompt.contains("<system>"));
    assert!(!prompt.contains("first request"));
    assert!(!prompt.contains("<recent_messages>"));
    assert!(!prompt.contains("<conversation_history>"));
    assert!(!prompt.contains("<conversation_summary>"));
}

#[test]
fn parse_acp_options_maps_cli_style_runtime_overrides() {
    let parsed = parse_acp_options(
        &json!({
            "acp_permission_mode": "approve-reads",
            "acp_non_interactive_permissions": "fail",
            "acp_auth_policy": "fail",
            "acp_session_mode": "plan",
            "acp_session_model": "gpt-5-codex",
            "acp_allowed_tools": ["read_file", "apply_patch"],
            "acp_max_turns": 12,
            "acp_session_config": {
                "sandbox_mode": "workspace-write"
            },
            "reasoning_effort": "high"
        }),
        "codex",
        &config::schema::AcpAgentConfig {
            command: "codex".to_string(),
            args: Vec::new(),
            env: Default::default(),
        },
    )
    .expect("acp options should parse");

    assert_eq!(
        parsed,
        ParsedAcpOptions {
            permission_mode: Some(PermissionMode::ApproveReads),
            non_interactive_permissions: Some(NonInteractivePermissionPolicy::Fail),
            auth_policy: Some(AuthPolicy::Fail),
            session_mode: Some("plan".to_string()),
            session_options: Some(AcpSessionOptions {
                model: Some("gpt-5-codex".to_string()),
                allowed_tools: Some(vec!["read_file".to_string(), "apply_patch".to_string()]),
                max_turns: Some(12),
            }),
            session_config_options: vec![
                ("sandbox_mode".to_string(), "workspace-write".to_string()),
                ("reasoning_effort".to_string(), "high".to_string()),
            ],
        }
    );
}

#[test]
fn parse_acp_options_normalizes_codex_thought_level() {
    let parsed = parse_acp_options(
        &json!({
            "acp_session_config": {
                "thought_level": "high"
            }
        }),
        "codex",
        &config::schema::AcpAgentConfig {
            command: "codex".to_string(),
            args: Vec::new(),
            env: Default::default(),
        },
    )
    .expect("acp options should parse");

    assert_eq!(
        parsed.session_config_options,
        vec![("reasoning_effort".to_string(), "high".to_string())]
    );
}

#[test]
fn parse_acp_options_rejects_invalid_permission_mode() {
    let error = parse_acp_options(
        &json!({
            "acp_permission_mode": "unsafe"
        }),
        "codex",
        &config::schema::AcpAgentConfig {
            command: "codex".to_string(),
            args: Vec::new(),
            env: Default::default(),
        },
    )
    .expect_err("invalid permission mode should fail");

    assert!(format!("{error}").contains("invalid acp_permission_mode"));
}

#[test]
fn acp_session_name_tracks_local_session_id() {
    assert_eq!(
        acp_session_name(" local-session-1 "),
        Some("vw-session:local-session-1".to_string())
    );
    assert_eq!(acp_session_name("   "), None);
}

#[test]
fn build_acp_command_line_preserves_configured_args() {
    let command_line = build_acp_command_line(&config::schema::AcpAgentConfig {
        command: "npx".to_string(),
        args: vec!["-y".to_string(), "@agentclientprotocol/claude-agent-acp@latest".to_string()],
        env: Default::default(),
    });

    assert_eq!(command_line, "npx -y @agentclientprotocol/claude-agent-acp@latest");
}

#[test]
fn lookup_acp_command_uses_built_in_agent_when_config_is_empty() {
    let cfg = config::schema::Config::default();
    let model = test_model("gpt-5.4");
    let (agent_name, agent_cfg) =
        lookup_acp_command(&cfg, &model, &json!({})).expect("built-in ACP agent should resolve");

    assert_eq!(agent_name, "codex");
    assert!(!agent_cfg.command.trim().is_empty());
}

#[test]
fn lookup_acp_command_resolves_built_in_agent_from_explicit_selection() {
    let cfg = config::schema::Config::default();
    let model = test_model("gpt-5.4");
    let (agent_name, agent_cfg) =
        lookup_acp_command(&cfg, &model, &json!({ "acp_agent": "opencode" }))
            .expect("explicit built-in ACP agent should resolve");

    assert_eq!(agent_name, "opencode");
    assert!(!agent_cfg.command.trim().is_empty());
    if agent_cfg.command == "npx" {
        assert_eq!(agent_cfg.args, vec!["opencode-ai@latest".to_string(), "acp".to_string()]);
    } else {
        assert_eq!(agent_cfg.args, vec!["acp".to_string()]);
    }
}

#[test]
fn missing_session_error_matches_cli_guidance() {
    let error = missing_session_error(
        "/Users/xiongjiaojiao/code/vibe-window",
        "claude",
        Some("vw-session:local-session-1"),
    );

    assert_eq!(
        format!("{error}"),
        "⚠ No vwacp session found (searched up to /Users/xiongjiaojiao/code/vibe-window).\nCreate one: vwacp claude sessions new --name vw-session:local-session-1"
    );
}

#[test]
fn normalize_acp_agent_config_rewrites_legacy_claude_code_package() {
    let normalized = normalize_acp_agent_config(
        "Claude Code",
        &config::schema::AcpAgentConfig {
            command: "npx".to_string(),
            args: vec!["@zed-industries/claude-code-acp@latest".to_string()],
            env: Default::default(),
        },
    );
    let expected =
        vw_acp::resolve_agent_spec("Claude Code").expect("built-in Claude Code spec should exist");

    assert_eq!(normalized.command, expected.command);
    assert_eq!(normalized.args, expected.args);
}

#[test]
fn acp_message_extractors_preserve_text_reasoning_and_tool_calls() {
    let text_message = serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "sessionId": "session-1",
            "update": {
                "sessionUpdate": "agent_message_chunk",
                "content": {
                    "type": "text",
                    "text": "hello "
                }
            }
        }
    }))
    .expect("message should deserialize");
    let reasoning_message = serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "sessionId": "session-1",
            "update": {
                "sessionUpdate": "agent_thought_chunk",
                "content": {
                    "type": "text",
                    "text": "thinking..."
                }
            }
        }
    }))
    .expect("message should deserialize");
    let tool_message = serde_json::from_value(json!({
        "jsonrpc": "2.0",
        "method": "session/update",
        "params": {
            "sessionId": "session-1",
            "update": {
                "sessionUpdate": "tool_call",
                "toolCallId": "tool-1",
                "title": "Read File",
                "rawInput": {
                    "path": "/tmp/demo.txt"
                }
            }
        }
    }))
    .expect("message should deserialize");

    assert_eq!(extract_delta_from_acp_message(&text_message), Some("hello ".to_string()));
    assert_eq!(
        extract_reasoning_delta_from_acp_message(&reasoning_message),
        Some("thinking...".to_string())
    );

    let tool_call =
        extract_tool_call_from_acp_message(&tool_message).expect("tool call should be extracted");
    assert_eq!(tool_call.id, "tool-1");
    assert_eq!(tool_call.name, "Read File");
    assert_eq!(tool_call.arguments, r#"{"path":"/tmp/demo.txt"}"#);
}

#[test]
fn acp_session_changed_matcher_accepts_nested_acp_prefixes() {
    assert!(is_acp_session_changed_message(
        "acp session changed: expected=session-a actual=session-b"
    ));
    assert!(is_acp_session_changed_message(
        "acp: acp session changed: expected=session-a actual=session-b"
    ));
    assert!(!is_acp_session_changed_message("acp prompt failed: timeout"));
}
