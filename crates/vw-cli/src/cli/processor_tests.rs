//! 验证 CLI processor 的请求桥接、终态规整与 delta 转发行为。
//! 测试只使用纯转换、内存 channel 与未发起请求的 runtime。

use crate::app::agent::session::processor as legacy_processor;
use crate::cli::tui::runtime::gateway::GatewaySessionSeed;
use crate::session::ui_types as models;
use serde_json::json;
use std::path::PathBuf;
use vw_gateway_client::{GatewayClient, GatewayEndpoint};

use super::*;

fn sample_request() -> legacy_processor::Request {
    legacy_processor::Request {
        stream: 7,
        session: "session_bridge".to_string(),
        query: "continue the migration".to_string(),
        root: Some("/tmp/bridge-workspace".to_string()),
        model: Some("provider/model".to_string()),
        reliability: crate::app::agent::config::ReliabilityConfig::default(),
        options: json!({
            "chat_system_prompt": "be precise",
            "temperature": 0.1,
        }),
        approval: None,
        channel_name: None,
        non_cli_approval_context: None,
        assistant_message_id: None,
        history: vec![
            models::ChatMessage {
                role: models::ChatRole::System,
                content: "system prompt".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
            models::ChatMessage {
                role: models::ChatRole::Assistant,
                content: "previous answer".to_string(),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            },
        ],
        persist_app_session_artifacts: true,
        app_session_scope: None,
        abort_token: None,
        session_tools: vw_agent::session::runtime::SessionToolset::default(),
    }
}

fn sample_runtime(directory: &str) -> GatewayUiRuntime {
    let client = GatewayClient::new(GatewayEndpoint::new("127.0.0.1", 42617))
        .expect("gateway client should construct without connecting");
    GatewayUiRuntime::new(client, GatewaySessionSeed::new(PathBuf::from(directory)))
}

fn gateway_usage() -> GatewayChatUsage {
    GatewayChatUsage { input_tokens: 11, output_tokens: 23, cached_tokens: 5, reasoning_tokens: 7 }
}

#[test]
fn gateway_stream_request_from_processor_request_preserves_history_and_query() {
    let request = sample_request();

    let gateway = gateway_stream_request_from_processor_request(&request);

    assert_eq!(gateway.session_id.as_ref().map(AsRef::as_ref), Some("session_bridge"));
    assert_eq!(gateway.model.as_deref(), Some("provider/model"));
    assert_eq!(gateway.options, Some(request.options.clone()));
    assert_eq!(gateway.messages.len(), 3);
    assert_eq!(gateway.messages[0], json!({ "role": "system", "content": "system prompt" }));
    assert_eq!(gateway.messages[1], json!({ "role": "assistant", "content": "previous answer" }));
    assert_eq!(gateway.messages[2], json!({ "role": "user", "content": "continue the migration" }));
}

#[test]
fn gateway_stream_request_from_processor_request_drops_blank_session_and_null_options() {
    let mut request = sample_request();
    request.session = "   ".to_string();
    request.options = serde_json::Value::Null;

    let gateway = gateway_stream_request_from_processor_request(&request);

    assert!(gateway.session_id.is_none());
    assert!(gateway.options.is_none());
}

#[test]
fn gateway_stream_request_from_processor_request_maps_all_history_roles() {
    let mut request = sample_request();
    request.history = vec![
        models::ChatMessage {
            role: models::ChatRole::User,
            content: "earlier question".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        models::ChatMessage {
            role: models::ChatRole::Assistant,
            content: "earlier answer".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        models::ChatMessage {
            role: models::ChatRole::System,
            content: "system rule".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
        models::ChatMessage {
            role: models::ChatRole::Tool,
            content: "tool result".to_string(),
            model: None,
            think_timing: Vec::new(),
            processing_timing: None,
        },
    ];

    let gateway = gateway_stream_request_from_processor_request(&request);

    assert_eq!(
        gateway.messages,
        vec![
            json!({ "role": "user", "content": "earlier question" }),
            json!({ "role": "assistant", "content": "earlier answer" }),
            json!({ "role": "system", "content": "system rule" }),
            json!({ "role": "tool", "content": "tool result" }),
            json!({ "role": "user", "content": "continue the migration" }),
        ]
    );
}

#[test]
fn gateway_stream_request_from_processor_request_keeps_empty_query_after_empty_history() {
    let mut request = sample_request();
    request.history.clear();
    request.query.clear();

    let gateway = gateway_stream_request_from_processor_request(&request);

    assert_eq!(gateway.messages, vec![json!({ "role": "user", "content": "" })]);
    assert!(gateway.system.is_none());
    assert!(gateway.reasoning_effort.is_none());
    assert!(gateway.agent.is_none());
    assert!(gateway.allowed_tools.is_none());
    assert!(gateway.acp_agent.is_none());
    assert!(gateway.acp_allowed_tools.is_none());
}

#[test]
fn gateway_stream_request_from_processor_request_trims_session_and_keeps_non_null_options() {
    let mut request = sample_request();
    request.session = "  session_trimmed  ".to_string();
    request.options = json!(false);

    let gateway = gateway_stream_request_from_processor_request(&request);

    assert_eq!(gateway.session_id.as_ref().map(AsRef::as_ref), Some("session_trimmed"));
    assert_eq!(gateway.options, Some(json!(false)));
}

#[test]
fn gateway_request_options_distinguishes_null_from_empty_containers() {
    assert_eq!(gateway_request_options(&serde_json::Value::Null), None);
    assert_eq!(gateway_request_options(&json!({})), Some(json!({})));
    assert_eq!(gateway_request_options(&json!([])), Some(json!([])));
}

#[test]
fn gateway_message_value_preserves_literal_role_and_content() {
    let message = gateway_message_value("custom-role", "line one\n\"quoted\"");

    assert_eq!(
        message,
        json!({
            "role": "custom-role",
            "content": "line one\n\"quoted\"",
        })
    );
}

#[test]
fn comparable_result_into_cli_result_keeps_done_payload() {
    let result = SessionProcessorComparableResult {
        output: "assistant output".to_string(),
        usage: models::TokenUsage {
            input_tokens: 11,
            output_tokens: 23,
            cached_tokens: 5,
            reasoning_tokens: 7,
        },
        step_finishes: 2,
        terminal: SessionProcessorComparableTerminal::Done {
            finish_reason: Some("stop".to_string()),
            message_id: Some("msg_a".to_string()),
            parent_message_id: Some("msg_u".to_string()),
        },
    };

    let cli = result.into_cli_result().expect("done terminal should stay successful");

    assert_eq!(cli.output, "assistant output");
    assert_eq!(cli.usage.input_tokens, 11);
    assert_eq!(cli.usage.output_tokens, 23);
    assert_eq!(cli.usage.cached_tokens, 5);
    assert_eq!(cli.usage.reasoning_tokens, 7);
    assert_eq!(cli.step_finishes, 2);
}

#[test]
fn comparable_result_into_cli_result_turns_timeout_into_error() {
    let result = SessionProcessorComparableResult {
        output: "partial output".to_string(),
        usage: models::TokenUsage::default(),
        step_finishes: 1,
        terminal: SessionProcessorComparableTerminal::TimedOut {
            message: "request timed out after 90s".to_string(),
            message_id: None,
            parent_message_id: None,
        },
    };

    let err = result.into_cli_result().expect_err("timeout terminal should become error");

    assert_eq!(err.to_string(), "request timed out after 90s");
}

#[test]
fn comparable_result_into_cli_result_uses_cancel_reason() {
    let result = SessionProcessorComparableResult {
        output: "partial output".to_string(),
        usage: models::TokenUsage::default(),
        step_finishes: 1,
        terminal: SessionProcessorComparableTerminal::Cancelled {
            reason: Some("cancelled by user".to_string()),
            message_id: Some("msg_a".to_string()),
            parent_message_id: Some("msg_u".to_string()),
        },
    };

    let err = result.into_cli_result().expect_err("cancelled terminal should become error");

    assert_eq!(err.to_string(), "cancelled by user");
}

#[test]
fn comparable_result_into_cli_result_supplies_default_cancel_reason() {
    let result = SessionProcessorComparableResult {
        output: String::new(),
        usage: models::TokenUsage::default(),
        step_finishes: 0,
        terminal: SessionProcessorComparableTerminal::Cancelled {
            reason: None,
            message_id: None,
            parent_message_id: None,
        },
    };

    let err = result.into_cli_result().expect_err("cancelled terminal should become error");

    assert_eq!(err.to_string(), "session processor cancelled");
}

#[test]
fn comparable_result_into_cli_result_preserves_error_message() {
    let result = SessionProcessorComparableResult {
        output: String::new(),
        usage: models::TokenUsage::default(),
        step_finishes: 0,
        terminal: SessionProcessorComparableTerminal::Error("processor failed".to_string()),
    };

    let err = result.into_cli_result().expect_err("error terminal should stay an error");

    assert_eq!(err.to_string(), "processor failed");
}

#[test]
fn runtime_directory_match_accepts_absent_blank_and_equal_roots() {
    let runtime = sample_runtime("/tmp/bridge-workspace");
    let mut request = sample_request();

    request.root = None;
    assert!(ensure_runtime_directory_matches_request(&runtime, &request).is_ok());

    request.root = Some("   ".to_string());
    assert!(ensure_runtime_directory_matches_request(&runtime, &request).is_ok());

    request.root = Some("  /tmp/bridge-workspace  ".to_string());
    assert!(ensure_runtime_directory_matches_request(&runtime, &request).is_ok());
}

#[tokio::test]
async fn gateway_runtime_rejects_directory_mismatch_before_streaming() {
    let runtime = sample_runtime("/tmp/runtime-workspace");
    let request = sample_request();

    let err = run_gateway_runtime_for_cli(&runtime, request, None)
        .await
        .expect_err("mismatched roots must be rejected before a gateway request");

    assert_eq!(
        err.to_string(),
        "gateway runtime directory does not match legacy processor request root: \
runtime=/tmp/runtime-workspace, request=/tmp/bridge-workspace"
    );
}

#[test]
fn gateway_delta_forwarder_without_receiver_does_not_spawn_task() {
    let (bridge_tx, task) = start_gateway_delta_forwarder(None);

    assert!(bridge_tx.is_none());
    assert!(task.is_none());
}

#[tokio::test]
async fn gateway_delta_forwarder_preserves_delta_order() {
    let (delta_tx, mut delta_rx) = mpsc::channel(2);
    let (bridge_tx, task) = start_gateway_delta_forwarder(Some(delta_tx));
    let bridge_tx = bridge_tx.expect("bridge sender should be created");
    let task = task.expect("forwarder task should be created");

    bridge_tx.send("first".to_string()).expect("forward first delta");
    bridge_tx.send("second".to_string()).expect("forward second delta");
    drop(bridge_tx);

    assert_eq!(delta_rx.recv().await.as_deref(), Some("first"));
    assert_eq!(delta_rx.recv().await.as_deref(), Some("second"));
    assert!(delta_rx.recv().await.is_none());
    task.await.expect("delta forwarder should finish cleanly");
}

#[tokio::test]
async fn gateway_delta_forwarder_stops_when_delta_receiver_is_closed() {
    let (delta_tx, delta_rx) = mpsc::channel(1);
    drop(delta_rx);
    let (bridge_tx, task) = start_gateway_delta_forwarder(Some(delta_tx));
    let bridge_tx = bridge_tx.expect("bridge sender should be created");
    let task = task.expect("forwarder task should be created");

    bridge_tx.send("ignored".to_string()).expect("queue delta for forwarding");
    drop(bridge_tx);

    task.await.expect("closed receiver should stop the forwarder cleanly");
}

#[test]
fn token_usage_from_terminal_maps_usage_and_defaults_errors() {
    let done = UiRuntimeTerminalEvent::Done {
        finish_reason: Some("stop".to_string()),
        usage: Some(gateway_usage()),
        message_id: None,
        parent_message_id: None,
    };
    let cancelled = UiRuntimeTerminalEvent::Cancelled {
        reason: None,
        usage: Some(gateway_usage()),
        message_id: None,
        parent_message_id: None,
    };
    let timed_out = UiRuntimeTerminalEvent::TimedOut {
        message: "timed out".to_string(),
        usage: Some(gateway_usage()),
        message_id: None,
        parent_message_id: None,
    };
    let expected = models::TokenUsage {
        input_tokens: 11,
        output_tokens: 23,
        cached_tokens: 5,
        reasoning_tokens: 7,
    };

    assert_eq!(token_usage_from_terminal(&done), expected);
    assert_eq!(token_usage_from_terminal(&cancelled), expected);
    assert_eq!(token_usage_from_terminal(&timed_out), expected);
    assert_eq!(
        token_usage_from_terminal(&UiRuntimeTerminalEvent::Cancelled {
            reason: None,
            usage: None,
            message_id: None,
            parent_message_id: None,
        }),
        models::TokenUsage::default()
    );
    assert_eq!(
        token_usage_from_terminal(&UiRuntimeTerminalEvent::Error("failed".to_string())),
        models::TokenUsage::default()
    );
}

#[test]
fn token_usage_from_terminal_defaults_done_without_usage() {
    let terminal = UiRuntimeTerminalEvent::Done {
        finish_reason: None,
        usage: None,
        message_id: Some("msg_done".to_string()),
        parent_message_id: Some("msg_parent".to_string()),
    };

    assert_eq!(token_usage_from_terminal(&terminal), models::TokenUsage::default());
}

#[test]
fn comparable_terminal_conversion_preserves_each_runtime_terminal() {
    let done = comparable_terminal_from_runtime_terminal(UiRuntimeTerminalEvent::Done {
        finish_reason: Some("stop".to_string()),
        usage: Some(gateway_usage()),
        message_id: Some("msg_done".to_string()),
        parent_message_id: Some("msg_parent".to_string()),
    });
    assert_eq!(
        done,
        SessionProcessorComparableTerminal::Done {
            finish_reason: Some("stop".to_string()),
            message_id: Some("msg_done".to_string()),
            parent_message_id: Some("msg_parent".to_string()),
        }
    );

    let cancelled = comparable_terminal_from_runtime_terminal(UiRuntimeTerminalEvent::Cancelled {
        reason: Some("user cancelled".to_string()),
        usage: None,
        message_id: Some("msg_cancelled".to_string()),
        parent_message_id: None,
    });
    assert_eq!(
        cancelled,
        SessionProcessorComparableTerminal::Cancelled {
            reason: Some("user cancelled".to_string()),
            message_id: Some("msg_cancelled".to_string()),
            parent_message_id: None,
        }
    );

    let timed_out = comparable_terminal_from_runtime_terminal(UiRuntimeTerminalEvent::TimedOut {
        message: "deadline exceeded".to_string(),
        usage: None,
        message_id: None,
        parent_message_id: Some("msg_parent".to_string()),
    });
    assert_eq!(
        timed_out,
        SessionProcessorComparableTerminal::TimedOut {
            message: "deadline exceeded".to_string(),
            message_id: None,
            parent_message_id: Some("msg_parent".to_string()),
        }
    );

    assert_eq!(
        comparable_terminal_from_runtime_terminal(UiRuntimeTerminalEvent::Error(
            "gateway failed".to_string()
        )),
        SessionProcessorComparableTerminal::Error("gateway failed".to_string())
    );
}
