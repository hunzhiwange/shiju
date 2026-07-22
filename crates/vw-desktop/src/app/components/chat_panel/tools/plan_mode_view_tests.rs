use super::ToolPermissionState;
use super::plan_mode_view::{
    body_lines, bool_field, derived_summary, is_plan_mode_tool, metadata_text,
    plan_mode_base_title, plan_mode_title, string_field, string_list_field, tool_plan_mode_view,
    u64_field,
};
use crate::app::{App, Message};
use serde_json::json;

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: iced::Element<'_, Message>) {
    std::hint::black_box(element);
}

#[test]
fn plan_mode_tool_names_are_explicit() {
    assert!(is_plan_mode_tool("enter_plan_mode"));
    assert!(is_plan_mode_tool("verify_plan_execution"));
    assert!(!is_plan_mode_tool("bash"));
}

#[test]
fn field_helpers_ignore_wrong_types_and_empty_strings() {
    let value = json!({"goal":" ship ","ok":true,"count":3,"items":["a", "", "b"]});
    let data = value.as_object();

    assert_eq!(string_field(data, "goal"), Some("ship"));
    assert!(bool_field(data, "ok"));
    assert_eq!(u64_field(data, "count"), 3);
    assert_eq!(string_list_field(data, "items"), vec!["a".to_string(), "b".to_string()]);
}

#[test]
fn derived_summary_and_metadata_are_deterministic() {
    let value = json!({"ready":true,"pending_count":4,"todo_count":5,"in_progress_count":1,"goal":"finish"});
    let data = value.as_object();

    assert_eq!(derived_summary("verify_plan_execution", data), "Ready to execute 4 todo(s)");
    assert!(metadata_text("verify_plan_execution", data).contains("Pending: 4"));
}

#[test]
fn derived_summary_covers_plan_mode_state_transitions_and_blockers() {
    let enter_active = json!({"active":true});
    assert_eq!(derived_summary("enter_plan_mode", enter_active.as_object()), "Plan mode enabled");

    let enter_already = json!({"already_active":true});
    assert_eq!(
        derived_summary("enter_plan_mode", enter_already.as_object()),
        "Plan mode remains active"
    );

    let exit_done = json!({"exited":true});
    assert_eq!(derived_summary("exit_plan_mode", exit_done.as_object()), "Plan mode disabled");

    let blocked = json!({"ready":false,"blockers":["missing todos","dirty plan"]});
    assert_eq!(
        derived_summary("verify_plan_execution", blocked.as_object()),
        "Blocked by 2 issue(s)"
    );
}

#[test]
fn metadata_text_truncates_goal_and_note_without_verify_counts() {
    let value = json!({"goal":"ship feature","note":"review first"});
    let metadata = metadata_text("enter_plan_mode", value.as_object());

    assert!(metadata.contains("Goal: ship feature"));
    assert!(metadata.contains("Note: review first"));
    assert!(!metadata.contains("Todo:"));
}

#[test]
fn tool_plan_mode_view_rejects_invalid_inputs_and_empty_completed_data() {
    let app = test_app();

    assert!(tool_plan_mode_view(&app, 0, 0, "tool read\n{}").is_none());
    assert!(tool_plan_mode_view(&app, 0, 0, "tool enter_plan_mode\nnot json").is_none());
    assert!(
        tool_plan_mode_view(
            &app,
            0,
            0,
            r#"tool enter_plan_mode
{"status":"completed","result":{"data":{}}}"#
        )
        .is_none()
    );
}

#[test]
fn tool_plan_mode_view_builds_enter_exit_verify_running_and_error_cards() {
    let app = test_app();

    let enter = tool_plan_mode_view(&app, 1, 1, r#"tool enter_plan_mode
{"status":"completed","result":{"data":{"active":true,"message":"Plan mode enabled","instructions":["inspect","plan"],"goal":"cover tests"}}}"#)
    .expect("enter view");
    keep_element(enter);

    let exit = tool_plan_mode_view(&app, 1, 2, r#"tool exit_plan_mode
{"status":"completed","result":{"data":{"exited":false,"active":true,"blockers":["finish plan"]}}}"#)
    .expect("exit view");
    keep_element(exit);

    let verify = tool_plan_mode_view(&app, 1, 3, r#"tool verify_plan_execution
{"status":"completed","result":{"data":{"ready":true,"todo_count":3,"pending_count":2,"in_progress_count":1}}}"#)
    .expect("verify view");
    keep_element(verify);

    let running = tool_plan_mode_view(
        &app,
        1,
        4,
        r#"tool enter_plan_mode
{"status":"running"}"#,
    )
    .expect("running view");
    keep_element(running);

    let error = tool_plan_mode_view(
        &app,
        1,
        5,
        r#"tool verify_plan_execution
{"status":"error","error":"blocked"}"#,
    )
    .expect("error view");
    keep_element(error);
}

#[test]
fn field_helpers_handle_missing_and_wrong_types() {
    let data_value = json!({"str":"hello","flag":true,"num":42,"list":["a"]});
    let data = data_value.as_object();

    assert_eq!(string_field(data, "missing"), None);
    assert_eq!(string_field(data, "flag"), None);
    assert_eq!(bool_field(data, "missing"), false);
    assert_eq!(bool_field(data, "str"), false);
    assert_eq!(u64_field(data, "missing"), 0);
    assert_eq!(u64_field(data, "str"), 0);
    assert!(string_list_field(data, "missing").is_empty());
    assert!(string_list_field(data, "str").is_empty());

    assert_eq!(string_field(None, "anything"), None);
    assert_eq!(bool_field(None, "anything"), false);
    assert_eq!(u64_field(None, "anything"), 0);
    assert!(string_list_field(None, "anything").is_empty());

    let empty_value = json!({"empty_str":""});
    let empty_data = empty_value.as_object();
    assert_eq!(string_field(empty_data, "empty_str"), None);
}

#[test]
fn derived_summary_handles_empty_data_and_fallthrough_paths() {
    assert_eq!(derived_summary("enter_plan_mode", None), "");
    assert_eq!(derived_summary("enter_plan_mode", Some(&json!({}).as_object().unwrap())), "");

    let exit_active = json!({"active":true});
    assert_eq!(
        derived_summary("exit_plan_mode", exit_active.as_object()),
        "Plan mode remains active"
    );
    assert_eq!(derived_summary("exit_plan_mode", None), "");
    assert_eq!(derived_summary("exit_plan_mode", Some(&json!({}).as_object().unwrap())), "");

    assert_eq!(derived_summary("verify_plan_execution", Some(&json!({}).as_object().unwrap())), "");
    assert_eq!(derived_summary("verify_plan_execution", None), "");

    assert_eq!(derived_summary("bash", None), "");
}

#[test]
fn metadata_text_handles_empty_and_partial_data() {
    assert_eq!(metadata_text("enter_plan_mode", None), "");
    assert_eq!(metadata_text("enter_plan_mode", Some(&json!({}).as_object().unwrap())), "");

    let meta_goal =
        metadata_text("enter_plan_mode", Some(&json!({"goal":"test"}).as_object().unwrap()));
    assert!(meta_goal.contains("Goal: test"));
    assert!(!meta_goal.contains("Note:"));

    let meta_note =
        metadata_text("enter_plan_mode", Some(&json!({"note":"reminder"}).as_object().unwrap()));
    assert!(!meta_note.contains("Goal:"));
    assert!(meta_note.contains("Note: reminder"));

    let meta_verify_zero = metadata_text(
        "verify_plan_execution",
        Some(&json!({"todo_count":0,"pending_count":0,"in_progress_count":0}).as_object().unwrap()),
    );
    assert!(meta_verify_zero.contains("Todo: 0"));
    assert!(meta_verify_zero.contains("Pending: 0"));
    assert!(meta_verify_zero.contains("In progress: 0"));
}

#[test]
fn plan_mode_title_helpers_cover_known_tools_and_state_precedence() {
    assert_eq!(plan_mode_base_title("enter_plan_mode"), Some("进入规划模式"));
    assert_eq!(plan_mode_base_title("exit_plan_mode"), Some("退出规划模式"));
    assert_eq!(plan_mode_base_title("verify_plan_execution"), Some("校验计划执行"));
    assert_eq!(plan_mode_base_title("bash"), None);

    assert_eq!(
        plan_mode_title("进入规划模式", Some(ToolPermissionState::ApprovalRequired), true, true),
        "进入规划模式待批准"
    );
    assert_eq!(
        plan_mode_title("进入规划模式", Some(ToolPermissionState::Rejected), false, true),
        "进入规划模式已拒绝"
    );
    assert_eq!(plan_mode_title("进入规划模式", None, true, true), "进入规划模式 运行中");
    assert_eq!(plan_mode_title("进入规划模式", None, false, true), "进入规划模式 失败");
    assert_eq!(plan_mode_title("进入规划模式", None, false, false), "进入规划模式");
}

#[test]
fn body_lines_builds_plan_mode_details_and_fallback_output() {
    let enter =
        json!({"result":{"data":{"message":" Read first ","instructions":["inspect"," ","plan"]}}});
    assert_eq!(
        body_lines("enter_plan_mode", &enter, false),
        vec![
            "Read first".to_string(),
            "In plan mode, you should:".to_string(),
            "1. inspect".to_string(),
            "2. plan".to_string(),
            "Remember: DO NOT write or edit any files yet. This is a read-only exploration and planning phase."
                .to_string(),
        ]
    );

    let enter_message = json!({"result":{"data":{"message":"Only message"}}});
    assert_eq!(body_lines("enter_plan_mode", &enter_message, false), vec!["Only message"]);

    let exited = json!({"result":{"data":{"exited":true}}});
    assert_eq!(
        body_lines("exit_plan_mode", &exited, false),
        vec!["Plan mode exited. Execution can continue."]
    );

    let exit_blocked = json!({"result":{"data":{"blockers":["finish todos","review"]}}});
    assert_eq!(
        body_lines("exit_plan_mode", &exit_blocked, false),
        vec![
            "Plan mode exit is currently blocked:".to_string(),
            "- finish todos".to_string(),
            "- review".to_string(),
        ]
    );

    let ready = json!({"result":{"data":{"ready":true,"todo_count":3,"pending_count":2,"in_progress_count":1}}});
    assert_eq!(
        body_lines("verify_plan_execution", &ready, false),
        vec!["Ready to execute.\nTodo: 3 · Pending: 2 · In progress: 1"]
    );

    let verify_blocked = json!({"result":{"data":{"blockers":["missing approval"]}}});
    assert_eq!(
        body_lines("verify_plan_execution", &verify_blocked, false),
        vec!["Plan execution is currently blocked:".to_string(), "- missing approval".to_string(),]
    );

    let fallback = json!({"output":" first line\n\n second line "});
    assert_eq!(body_lines("enter_plan_mode", &fallback, false), vec!["first line", "second line"]);
}

#[test]
fn body_lines_prefers_error_text_when_error_state_is_set() {
    let value = json!({"error":" denied \n try again ","output":"ignored"});

    assert_eq!(
        body_lines("enter_plan_mode", &value, true),
        vec!["denied".to_string(), "try again".to_string()]
    );
}

#[test]
fn tool_plan_mode_view_covers_exit_exited_blockers_and_fallback_paths() {
    let app = test_app();

    let exited = tool_plan_mode_view(
        &app,
        0,
        0,
        r#"tool exit_plan_mode
{"status":"completed","result":{"data":{"exited":true}}}"#,
    )
    .expect("exit completed");
    keep_element(exited);

    let blocked = tool_plan_mode_view(
        &app,
        0,
        1,
        r#"tool verify_plan_execution
{"status":"completed","result":{"data":{"ready":false,"blockers":["blocker1"]}}}"#,
    )
    .expect("blocked verify");
    keep_element(blocked);

    let msg_only = tool_plan_mode_view(
        &app,
        0,
        2,
        r#"tool enter_plan_mode
{"status":"completed","result":{"data":{"active":true,"message":"Hello"}}}"#,
    )
    .expect("message-only enter");
    keep_element(msg_only);

    let fallback = tool_plan_mode_view(
        &app,
        0,
        3,
        r#"tool enter_plan_mode
{"status":"completed","output":"fallback output"}"#,
    )
    .expect("fallback output");
    keep_element(fallback);

    let verify_fb = tool_plan_mode_view(
        &app,
        0,
        4,
        r#"tool verify_plan_execution
{"status":"completed","result":{"data":{}},"output":"verify fallback"}"#,
    )
    .expect("verify fallback");
    keep_element(verify_fb);

    assert!(
        tool_plan_mode_view(
            &app,
            0,
            5,
            r#"tool enter_plan_mode
{"status":"completed","result":{"data":{}}}"#
        )
        .is_none()
    );

    assert!(
        tool_plan_mode_view(
            &app,
            0,
            6,
            r#"tool exit_plan_mode
{"status":"completed","result":{"data":{}}}"#
        )
        .is_none()
    );

    let already = tool_plan_mode_view(
        &app,
        0,
        7,
        r#"tool enter_plan_mode
{"status":"completed","result":{"data":{"already_active":true}}}"#,
    )
    .expect("already active");
    keep_element(already);

    let exit_active = tool_plan_mode_view(
        &app,
        0,
        8,
        r#"tool exit_plan_mode
{"status":"completed","result":{"data":{"exited":false,"active":true}}}"#,
    )
    .expect("exit active");
    keep_element(exit_active);
}

#[test]
fn tool_plan_mode_view_handles_denied_status() {
    let app = test_app();
    let denied_view = tool_plan_mode_view(
        &app,
        0,
        0,
        r#"tool enter_plan_mode
{"status":"denied","error":"permission denied"}"#,
    )
    .expect("denied view");
    keep_element(denied_view);
}

#[test]
fn tool_plan_mode_view_builds_hovered_detail_action() {
    let mut app = test_app();
    app.chat_tool_hovered_idx = Some((7_u64 << 32) | 9);

    let view = tool_plan_mode_view(
        &app,
        7,
        9,
        r#"tool enter_plan_mode
{"status":"completed","result":{"data":{"active":true}}}"#,
    )
    .expect("hovered plan-mode view");

    keep_element(view);
}
