//! MCP desktop form behavior through the production message dispatcher.
use super::message::mcp_form::{FormScope, McpFormMessage};
use super::{App, Message};
use serde_json::json;
use vw_gateway_client::vw_api_types::mcp_form::McpFormRequest;

fn scope() -> FormScope {
    FormScope {
        run_id: 25,
        session: "chat-a".into(),
        host_id: "host-a".into(),
        client: std::sync::Arc::new(crate::app::config::gateway_client().unwrap()),
    }
}

fn app() -> App {
    let (mut app, _) = App::new();
    app.active_session_id = Some("chat-a".into());
    let runtime = app.get_session_runtime_mut("chat-a");
    runtime.is_requesting = true;
    runtime.active_agent_request = Some(std::sync::Arc::new(crate::app::AgentRequest {
        id: 25,
        session: "chat-a".into(),
        query: String::new(),
        root: None,
        model: None,
        reasoning_effort: None,
        acp_test: false,
        acp_agent: None,
        acp_allowed_tools: None,
        agent: None,
        allowed_tools: None,
        require_agent_tool: false,
        acp_force_new_session: false,
        acp_history_mode: crate::app::state::AcpHistoryReplayMode::Discard,
        acp_recent_count: 0,
        full_access_enabled: false,
        resume_history_only: false,
        workflow_mode_enabled: false,
        goal_mode_enabled: false,
        history: vec![],
    }));
    app
}

fn request() -> McpFormRequest {
    McpFormRequest {
        id: "form-a".into(),
        session_id: "chat-a".into(),
        server_name: "本地服务".into(),
        connection_id: "connection-a".into(),
        server_request_id: json!(4),
        message: "请填写中文表单".into(),
        requested_schema: json!({"type":"object","properties":{"name":{"type":"string","title":"姓名"}},"required":["name"]}),
        meta: None,
    }
}
fn deliver(app: &mut App, requests: Vec<McpFormRequest>) {
    let _ = app.update(Message::McpForm(McpFormMessage::Pending { scope: scope(), requests }));
}
#[test]
fn codex_plugin_compat_t25_renders_and_cancels_server_form() {
    let mut app = app();
    deliver(&mut app, vec![request()]);
    assert_eq!(app.mcp_forms.requests.len(), 1, "a server form must enter the desktop modal queue");
    assert_eq!(app.mcp_forms.requests[0].server_name, "本地服务");
    deliver(&mut app, vec![]);
    assert!(app.mcp_forms.requests.is_empty(), "finished form must disappear");
}
#[test]
fn codex_plugin_compat_t25_rejects_invalid_input() {
    let mut app = app();
    let mut invalid = request();
    invalid.requested_schema = json!({"type":"object","properties":{"nested":{"type":"object"}}});
    deliver(&mut app, vec![invalid]);
    assert!(app.mcp_forms.requests.is_empty());
    assert!(app.mcp_forms.error.is_some(), "unsupported controls must fail visibly");
}
#[test]
fn codex_plugin_compat_t25_preserves_isolation() {
    let mut app = app();
    deliver(&mut app, vec![request()]);
    let mut foreign = request();
    foreign.session_id = "chat-b".into();
    deliver(&mut app, vec![foreign]);
    assert_eq!(
        app.mcp_forms.requests.len(),
        1,
        "foreign payload cannot clear an owned pending form"
    );
    assert_eq!(app.mcp_forms.requests[0].session_id, "chat-a");
}

#[test]
fn codex_plugin_compat_t25_requires_explicit_values_and_preserves_raw_enum_values() {
    use super::message::mcp_form::{current, response};
    use vw_gateway_client::vw_api_types::mcp_form::McpFormAction;
    let mut app = app();
    let mut form = request();
    form.requested_schema = json!({"type":"object","properties":{
        "confirm":{"type":"boolean","default":true},
        "choice":{"type":"string","enum":["raw-a","raw-b"],"enumNames":["甲","乙"]},
        "count":{"type":"integer"}},"required":["confirm","choice","count"]});
    deliver(&mut app, vec![form]);
    assert!(response(current(&app).unwrap().1, McpFormAction::Accept).is_err());
    for (field, value) in [("confirm", "false"), ("choice", "raw-b"), ("count", "21")] {
        let _ = app.update(Message::McpForm(McpFormMessage::Changed {
            id: "form-a".into(),
            field: field.into(),
            value: value.into(),
        }));
    }
    let answer = response(current(&app).unwrap().1, McpFormAction::Accept).unwrap();
    assert_eq!(answer.content, Some(json!({"confirm":false,"choice":"raw-b","count":21})));
    for action in [McpFormAction::Decline, McpFormAction::Cancel] {
        assert!(response(current(&app).unwrap().1, action).unwrap().content.is_none());
    }
}
#[test]
fn codex_plugin_compat_t25_stale_run_and_closed_host_cannot_mutate_current_form() {
    let mut app = app();
    deliver(&mut app, vec![request()]);
    let mut stale = scope();
    stale.run_id = 24;
    let _ = app.update(Message::McpForm(McpFormMessage::Pending {
        scope: stale.clone(),
        requests: vec![],
    }));
    let _ = app.update(Message::McpForm(McpFormMessage::Closed(stale)));
    assert_eq!(app.mcp_forms.requests.len(), 1);
    let _ = app.update(Message::McpForm(McpFormMessage::Closed(scope())));
    assert!(app.mcp_forms.drafts.is_empty());
    assert!(app.mcp_forms.requests.is_empty());
}
#[test]
fn codex_plugin_compat_t25_invalid_submit_keeps_input_and_reports_toast() {
    use vw_gateway_client::vw_api_types::mcp_form::McpFormAction;
    let mut app = app();
    deliver(&mut app, vec![request()]);
    let _ = app.update(Message::McpForm(McpFormMessage::Submit {
        id: "form-a".into(),
        action: McpFormAction::Accept,
    }));
    assert!(!app.mcp_forms.drafts["form-a"].busy);
    assert!(app.mcp_forms.drafts["form-a"].error.is_some());
    assert!(app.active_toast.is_some());
    assert_eq!(app.mcp_forms.requests.len(), 1);
}
#[test]
fn codex_plugin_compat_t25_server_failure_keeps_input_without_echoing_remote_payload() {
    let mut app = app();
    deliver(&mut app, vec![request()]);
    let _ = app.update(Message::McpForm(McpFormMessage::Changed {
        id: "form-a".into(),
        field: "name".into(),
        value: "仅用户可见".into(),
    }));
    let _ = app.update(Message::McpForm(McpFormMessage::Replied {
        scope: scope(),
        id: "form-a".into(),
        result: Err("remote-private-payload".into()),
    }));
    assert_eq!(app.mcp_forms.drafts["form-a"].values["name"], "仅用户可见");
    assert!(
        !app.mcp_forms.drafts["form-a"].error.as_ref().unwrap().contains("remote-private-payload")
    );
}
#[test]
fn codex_plugin_compat_t25_run_end_discards_all_answers_and_late_events() {
    let mut app = app();
    deliver(&mut app, vec![request()]);
    let _ = app.update(Message::McpForm(McpFormMessage::Changed {
        id: "form-a".into(),
        field: "name".into(),
        value: "private-value".into(),
    }));
    app.get_session_runtime_mut("chat-a").is_requesting = false;
    let _ = app.update(Message::None);
    assert!(app.mcp_forms.drafts.is_empty());
    deliver(&mut app, vec![request()]);
    assert!(app.mcp_forms.requests.is_empty());
}
#[test]
fn codex_plugin_compat_t25_debug_does_not_include_user_values_or_host_capability() {
    let message = McpFormMessage::Changed {
        id: "form-a".into(),
        field: "name".into(),
        value: "private-value".into(),
    };
    assert!(!format!("{message:?}").contains("private-value"));
    assert!(!format!("{:?}", scope()).contains("host-a"));
}
