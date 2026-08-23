//! 验证工具决策合并、权限检查和拒绝输入快照行为。

use super::*;
use crate::agent::loop_::approval::NonCliApprovalContext;
use crate::app::agent::approval::{ApprovalManager, ApprovalResponse};
use crate::app::agent::config::AutonomyConfig;
use crate::app::agent::security::AutonomyLevel;
use serde_json::json;
use std::sync::Arc;

struct MockTool {
    spec: ToolSpec,
    permission_error: Option<&'static str>,
}

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl Tool for MockTool {
    fn name(&self) -> &str {
        &self.spec.id
    }

    fn description(&self) -> &str {
        &self.spec.description
    }

    fn parameters_schema(&self) -> Value {
        self.spec.input_schema.clone()
    }

    fn spec(&self) -> ToolSpec {
        self.spec.clone()
    }

    async fn execute(&self, _args: Value) -> anyhow::Result<super::super::ToolResult> {
        Ok(super::super::ToolResult { success: true, output: "ok".to_string(), error: None })
    }

    async fn check_permissions(&self, _input: &Value) -> anyhow::Result<()> {
        if let Some(message) = self.permission_error {
            anyhow::bail!(message);
        }
        Ok(())
    }
}

fn tool(id: &str, read_only: bool, permission_error: Option<&'static str>) -> MockTool {
    MockTool {
        spec: ToolSpec::new(id, "mock", Value::Null).with_read_only(read_only),
        permission_error,
    }
}

fn context_with_security(security: SecurityPolicy) -> ToolUseContext {
    ToolUseContext::new("decision-test", Some(".".to_string())).with_security(Arc::new(security))
}

#[test]
fn allow_and_ask_expose_updated_input_and_reason() {
    let allow = PermissionDecision::allow(json!({"ok": true}));
    assert!(allow.reason().is_none());
    assert_eq!(allow.updated_input(), Some(&json!({"ok": true})));

    let ask = PermissionDecision::ask("needs approval", json!({"cmd": "git status"}));
    assert_eq!(ask.reason(), Some("needs approval"));
    assert_eq!(ask.updated_input(), Some(&json!({"cmd": "git status"})));
}

#[tokio::test]
async fn finalize_allows_without_security_and_classifies_tool_permission_errors() {
    let context = ToolUseContext::new("decision-test", None);
    let allowed = finalize_tool_input(&tool("read", true, None), json!({"path": "ok"}), &context)
        .await
        .expect("no security allows input");
    assert_eq!(allowed, json!({"path": "ok"}));

    let error = finalize_tool_input(
        &tool("read", true, Some("blocked by tool")),
        json!({"path": "ok"}),
        &context,
    )
    .await
    .expect_err("tool permission errors are classified");

    assert!(matches!(error, ToolCallError::Denied { .. }));
}

#[tokio::test]
async fn finalize_permission_decision_maps_allow_ask_and_denials() {
    let mock = tool("write", false, None);
    let allowed =
        finalize_permission_decision(&mock, PermissionDecision::allow(json!({"path": "ok"})))
            .await
            .expect("allow returns input");
    assert_eq!(allowed, json!({"path": "ok"}));

    let ask = finalize_permission_decision(
        &mock,
        PermissionDecision::ask("approval required", json!({"path": "ok"}))
            .with_warning(Some("review".to_string())),
    )
    .await
    .expect_err("ask becomes denied permission request");
    assert!(matches!(ask, ToolCallError::Denied { permission_request: Some(_), .. }));

    let permission_denial = finalize_permission_decision(
        &mock,
        PermissionDecision::deny("Denied by user.").with_updated_input(json!({"path": "ok"})),
    )
    .await
    .expect_err("permission denials keep request context");
    assert!(matches!(permission_denial, ToolCallError::Denied { permission_request: Some(_), .. }));

    let plain_denial =
        finalize_permission_decision(&mock, PermissionDecision::deny("path blocked"))
            .await
            .expect_err("plain denial has no permission request");
    assert!(matches!(plain_denial, ToolCallError::Denied { permission_request: None, .. }));
}

#[test]
fn deny_only_exposes_input_after_explicit_snapshot() {
    let deny = PermissionDecision::deny("blocked").with_warning(Some("careful".into()));
    assert_eq!(deny.reason(), Some("blocked"));
    assert_eq!(deny.warning(), Some("careful"));
    assert!(deny.updated_input().is_none());

    let deny = deny.with_updated_input(json!({"path": "secret"}));
    assert_eq!(deny.updated_input(), Some(&json!({"path": "secret"})));
}

#[test]
fn updated_input_snapshot_only_changes_denials() {
    let allow = PermissionDecision::allow(json!({"original": true}))
        .with_updated_input(json!({"replacement": true}));

    assert_eq!(allow.updated_input(), Some(&json!({"original": true})));
}

#[test]
fn security_decision_handles_readonly_rate_limit_and_blocked_paths() {
    let write_spec = ToolSpec::new("write", "write", Value::Null).with_read_only(false);
    let read_spec = ToolSpec::new("read", "read", Value::Null).with_read_only(true);

    assert!(matches!(
        security_decision(
            &read_spec,
            json!({"path": "../secret"}),
            &ToolUseContext::new("decision-test", None),
            false,
        ),
        PermissionDecision::Allow { .. }
    ));

    let readonly =
        SecurityPolicy { autonomy: AutonomyLevel::ReadOnly, ..SecurityPolicy::default() };
    let decision = security_decision(
        &write_spec,
        json!({"path": "safe.txt"}),
        &context_with_security(readonly),
        false,
    );
    assert!(matches!(decision, PermissionDecision::Deny { .. }));
    assert!(decision.reason().unwrap_or_default().contains("read-only"));

    let rate_limited = SecurityPolicy { max_actions_per_hour: 0, ..SecurityPolicy::default() };
    let decision = security_decision(
        &write_spec,
        json!({"path": "safe.txt"}),
        &context_with_security(rate_limited),
        false,
    );
    assert!(decision.reason().unwrap_or_default().contains("Rate limit"));

    let decision = security_decision(
        &write_spec,
        json!({"path": "../outside"}),
        &context_with_security(SecurityPolicy::default()),
        false,
    );
    assert!(decision.reason().unwrap_or_default().contains("Path not allowed"));
}

#[test]
fn blocked_path_checks_string_and_array_fields() {
    let security = SecurityPolicy::default();

    assert_eq!(blocked_path(&json!("not an object"), &security), None);
    assert_eq!(blocked_path(&json!({"dir": "src"}), &security), None);
    assert_eq!(
        blocked_path(&json!({"path": "../secret"}), &security).as_deref(),
        Some("../secret")
    );
    assert_eq!(
        blocked_path(&json!({"newPath": ["src/lib.rs", "../secret"]}), &security).as_deref(),
        Some("../secret")
    );
}

#[test]
fn shell_security_decision_marks_approved_inputs_and_preserves_denials() {
    let security = SecurityPolicy::default();
    let allowed =
        shell_security_decision("shell", json!({"command": "date"}), &security, "date", true);
    assert_eq!(allowed.updated_input().unwrap()["approved"], json!(true));

    let denied = shell_security_decision(
        "shell",
        json!({"command": "rm -rf /"}),
        &security,
        "rm -rf /",
        false,
    );
    assert!(matches!(denied, PermissionDecision::Deny { .. } | PermissionDecision::Ask { .. }));
    assert!(denied.updated_input().is_some());
}

#[tokio::test]
async fn request_interactive_approval_denies_when_manager_is_missing_or_user_declines() {
    let pending = PermissionDecision::ask("needs approval", json!({"command": "python script.py"}))
        .with_warning(Some("review".to_string()));
    let no_manager =
        request_interactive_approval("shell", pending.clone(), &ToolUseContext::new("s", None))
            .await;

    assert!(matches!(no_manager, PermissionDecision::Deny { .. }));
    assert!(no_manager.reason().unwrap_or_default().contains("unavailable"));
    assert_eq!(no_manager.warning(), Some("review"));
    assert!(no_manager.updated_input().is_some());

    let approval = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let context = ToolUseContext::new("s", None).with_approval(approval).with_channel("slack");
    let declined = request_interactive_approval("shell", pending, &context).await;

    assert_eq!(declined.reason(), Some("Denied by user."));
    assert!(declined.updated_input().is_some());
}

#[tokio::test]
async fn non_cli_bypass_records_approval_and_allows_pending_decision() {
    let approval = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let context = ToolUseContext::new("s", None)
        .with_approval(approval.clone())
        .with_channel("slack")
        .with_bypass_non_cli_approval_for_turn(true);

    let decision = request_interactive_approval(
        "write",
        PermissionDecision::allow(json!({"path": "safe.txt"})),
        &context,
    )
    .await;

    assert!(matches!(decision, PermissionDecision::Allow { .. }));
}

#[tokio::test]
async fn non_cli_approval_context_sends_prompt_and_waits_for_resolution() {
    let approval = Arc::new(ApprovalManager::from_config(&AutonomyConfig::default()));
    let (prompt_tx, mut prompt_rx) = tokio::sync::mpsc::unbounded_channel();
    let context = ToolUseContext::new("s", None)
        .with_approval(approval.clone())
        .with_channel("telegram")
        .with_message_id("msg-1")
        .with_tool_call_id("call-1")
        .with_non_cli_approval_context(NonCliApprovalContext {
            sender: "alice".to_string(),
            reply_target: "chat-1".to_string(),
            prompt_tx,
        });
    let approver = approval.clone();
    let approval_task = tokio::spawn(async move {
        let prompt = prompt_rx.recv().await.expect("prompt");
        approver.record_non_cli_pending_resolution(&prompt.request_id, ApprovalResponse::Yes);
    });

    let decision = request_interactive_approval(
        "shell",
        PermissionDecision::ask("needs approval", json!({"command": "date"})),
        &context,
    )
    .await;

    approval_task.await.expect("approval task");
    assert!(matches!(decision, PermissionDecision::Allow { .. }));
    assert_eq!(decision.updated_input().unwrap()["approved"], json!(true));
}

#[test]
fn command_input_only_accepts_shell_tools_and_string_commands() {
    assert_eq!(command_input("shell", &json!({"command": "echo ok"})).as_deref(), Some("echo ok"));
    assert!(command_input("read", &json!({"command": "echo ok"})).is_none());
    assert!(command_input("bash", &json!({"command": 42})).is_none());
}

#[test]
fn approval_marker_only_changes_shell_object_inputs() {
    assert_eq!(
        mark_input_as_approved("process", json!({"command": "echo ok"})),
        json!({"command": "echo ok", "approved": true})
    );
    assert_eq!(mark_input_as_approved("read", json!({"path": "a"})), json!({"path": "a"}));
    assert_eq!(mark_input_as_approved("shell", json!(["echo ok"])), json!(["echo ok"]));
}

#[test]
fn permission_reason_helpers_preserve_structured_context() {
    assert_eq!(
        render_reason("approval required".into(), Some("review command")),
        "approval required. review command"
    );
    assert_eq!(render_reason("blocked".into(), Some("  ")), "blocked");

    let request = permission_request_from_decision(
        "Denied by user.",
        Some("review command"),
        Some(&json!({"command": "echo ok"})),
    )
    .expect("permission-related denial should retain request context");
    assert_eq!(request.reason, "Denied by user.");
    assert_eq!(request.warning.as_deref(), Some("review command"));
    assert_eq!(request.updated_input, Some(json!({"command": "echo ok"})));
    assert!(permission_request_from_decision("path blocked", None, None).is_none());
}

#[test]
fn classify_message_separates_denials_from_execution_failures() {
    assert!(matches!(
        classify_message(anyhow::anyhow!("operation forbidden")),
        ToolCallError::Denied { .. }
    ));
    assert!(matches!(
        classify_message(anyhow::anyhow!("transport failed")),
        ToolCallError::Failed(_)
    ));
}
