//! 验证 ACP 客户端构建器的默认状态、可选配置与回调存储。
//!
//! 最小配置和回调 fixture 用于检查构建器赋值，不启动代理进程。

use std::collections::HashMap;
use std::sync::Arc;

use agent_client_protocol::{McpServer, McpServerHttp, SessionNotification};
use tokio::time::Duration;

use crate::types::{
    AcpAgentConfig, AcpJsonRpcMessage, AcpMessageCallback, AcpMessageDirection, AcpSessionOptions,
    AuthPolicy, ClientOperation, ClientOperationCallback, NonInteractivePermissionPolicy,
    PermissionMode, PermissionStats, SessionUpdateCallback,
};

use super::{AcpClient, DEFAULT_ACTOR_IDLE_TIMEOUT};

fn config() -> AcpAgentConfig {
    let mut env = HashMap::new();
    env.insert("AGENT_TOKEN".to_string(), "token".to_string());

    AcpAgentConfig { command: "agent-bin".to_string(), args: vec!["--acp".to_string()], env }
}

// 下列行为用例覆盖默认值、替换与清空语义，以及权限统计快照。
#[test]
fn new_sets_default_builder_state() {
    let client = AcpClient::new("test-agent", config());

    assert_eq!(client.agent_name, "test-agent");
    assert_eq!(client.config.command, "agent-bin");
    assert_eq!(client.config.args, vec!["--acp"]);
    assert_eq!(client.config.env.get("AGENT_TOKEN"), Some(&"token".to_string()));
    assert_eq!(client.client_name, "vibewindow-acp-client");
    assert_eq!(client.client_version, env!("CARGO_PKG_VERSION"));
    assert!(client.mcp_servers.is_empty());
    assert_eq!(client.permission_mode, PermissionMode::ApproveAll);
    assert!(!client.full_access);
    assert_eq!(client.non_interactive_permissions, None);
    assert!(client.auth_credentials.is_empty());
    assert_eq!(client.auth_policy, AuthPolicy::Skip);
    assert_eq!(client.session_options, None);
    assert!(!client.verbose);
    assert!(client.on_acp_message.is_none());
    assert!(client.on_acp_output_message.is_none());
    assert!(client.on_session_update.is_none());
    assert!(client.on_client_operation.is_none());
    assert_eq!(client.permission_stats(), PermissionStats::default());
    assert!(client.active_prompt.lock().is_none());
    assert!(client.cancelling_session_ids.lock().is_empty());
    assert!(client.actor_state.lock().reusable_session_id.is_none());
    assert_eq!(client.actor_idle_timeout, DEFAULT_ACTOR_IDLE_TIMEOUT);
}

#[test]
fn builder_methods_replace_configurable_values() {
    let mcp_servers =
        vec![McpServer::Http(McpServerHttp::new("project", "https://example.test/mcp"))];
    let session_options = AcpSessionOptions {
        model: Some("fast-model".to_string()),
        allowed_tools: Some(vec!["shell".to_string()]),
        max_turns: Some(4),
    };
    let mut auth_credentials = HashMap::new();
    auth_credentials.insert("api_key".to_string(), "secret".to_string());

    let client = AcpClient::new("test-agent", config())
        .with_client_info("", "")
        .with_mcp_servers(mcp_servers.clone())
        .with_permission_mode(PermissionMode::DenyAll)
        .with_full_access(true)
        .with_non_interactive_permissions(Some(NonInteractivePermissionPolicy::Fail))
        .with_session_options(Some(session_options.clone()))
        .with_auth_credentials(auth_credentials.clone())
        .with_auth_policy(AuthPolicy::Fail)
        .with_verbose(true)
        .with_actor_idle_timeout(Duration::from_millis(5));

    assert_eq!(client.client_name, "");
    assert_eq!(client.client_version, "");
    assert_eq!(client.mcp_servers, mcp_servers);
    assert_eq!(client.permission_mode, PermissionMode::DenyAll);
    assert!(client.full_access);
    assert_eq!(client.non_interactive_permissions, Some(NonInteractivePermissionPolicy::Fail));
    assert_eq!(client.session_options, Some(session_options));
    assert_eq!(client.auth_credentials, auth_credentials);
    assert_eq!(client.auth_policy, AuthPolicy::Fail);
    assert!(client.verbose);
    assert_eq!(client.actor_idle_timeout, Duration::from_millis(5));
}

#[test]
fn builder_methods_can_clear_optional_values() {
    let client = AcpClient::new("test-agent", config())
        .with_non_interactive_permissions(Some(NonInteractivePermissionPolicy::Deny))
        .with_session_options(Some(AcpSessionOptions::default()))
        .with_non_interactive_permissions(None)
        .with_session_options(None);

    assert_eq!(client.non_interactive_permissions, None);
    assert_eq!(client.session_options, None);
}

#[test]
fn callback_builders_store_callbacks() {
    let acp_callback: AcpMessageCallback =
        Arc::new(|_direction: AcpMessageDirection, _message: AcpJsonRpcMessage| {});
    let acp_output_callback = acp_callback.clone();
    let session_update_callback: SessionUpdateCallback =
        Arc::new(|_notification: SessionNotification| {});
    let client_operation_callback: ClientOperationCallback =
        Arc::new(|_operation: ClientOperation| {});

    let client = AcpClient::new("test-agent", config())
        .with_acp_message_callback(Some(acp_callback.clone()))
        .with_acp_output_message_callback(Some(acp_output_callback.clone()))
        .with_session_update_callback(Some(session_update_callback.clone()))
        .with_client_operation_callback(Some(client_operation_callback.clone()));

    assert!(
        client.on_acp_message.as_ref().is_some_and(|callback| Arc::ptr_eq(callback, &acp_callback))
    );
    assert!(
        client
            .on_acp_output_message
            .as_ref()
            .is_some_and(|callback| Arc::ptr_eq(callback, &acp_output_callback))
    );
    assert!(
        client
            .on_session_update
            .as_ref()
            .is_some_and(|callback| Arc::ptr_eq(callback, &session_update_callback))
    );
    assert!(
        client
            .on_client_operation
            .as_ref()
            .is_some_and(|callback| Arc::ptr_eq(callback, &client_operation_callback))
    );
}

#[test]
fn callback_builders_can_clear_callbacks() {
    let acp_callback: AcpMessageCallback =
        Arc::new(|_direction: AcpMessageDirection, _message: AcpJsonRpcMessage| {});
    let session_update_callback: SessionUpdateCallback =
        Arc::new(|_notification: SessionNotification| {});
    let client_operation_callback: ClientOperationCallback =
        Arc::new(|_operation: ClientOperation| {});

    let client = AcpClient::new("test-agent", config())
        .with_acp_message_callback(Some(acp_callback.clone()))
        .with_acp_output_message_callback(Some(acp_callback))
        .with_session_update_callback(Some(session_update_callback))
        .with_client_operation_callback(Some(client_operation_callback))
        .with_acp_message_callback(None)
        .with_acp_output_message_callback(None)
        .with_session_update_callback(None)
        .with_client_operation_callback(None);

    assert!(client.on_acp_message.is_none());
    assert!(client.on_acp_output_message.is_none());
    assert!(client.on_session_update.is_none());
    assert!(client.on_client_operation.is_none());
}

#[test]
fn permission_stats_returns_snapshot() {
    let client = AcpClient::new("test-agent", config());
    *client.permission_stats.lock() =
        PermissionStats { requested: 3, approved: 1, denied: 1, cancelled: 1 };

    let snapshot = client.permission_stats();
    *client.permission_stats.lock() = PermissionStats::default();

    assert_eq!(snapshot, PermissionStats { requested: 3, approved: 1, denied: 1, cancelled: 1 });
    assert_eq!(client.permission_stats(), PermissionStats::default());
}
