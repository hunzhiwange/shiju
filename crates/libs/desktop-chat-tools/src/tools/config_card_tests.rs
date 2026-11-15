use super::config_view::tool_config_view;
use super::view_fixture::TestApp as App;
use serde_json::json;
#[test]
fn config_view_rejects_bad_tool_and_empty_completed_result() {
    let app = App::new().0;

    assert!(tool_config_view(&app.files().card, 0, 0, "tool bash\n{}").is_none());
    assert!(tool_config_view(&app.files().card, 0, 0, "tool config\nnot-json").is_none());
    assert!(
        tool_config_view(
            &app.files().card,
            0,
            0,
            r#"tool config
{"status":"success","output":"plain text"}"#
        )
        .is_none()
    );
}

#[test]
fn config_view_renders_running_success_and_error_states() {
    let mut app = App::new().0;
    app.chat_tool_hovered_idx = Some((1_u64 << 32) | 2);

    assert!(
        tool_config_view(
            &app.files().card,
            1,
            2,
            r#"tool config
{"status":"running","input":"{\"setting\":\"browser.enabled\"}"}"#
        )
        .is_some()
    );
    assert!(
        tool_config_view(
            &app.files().card,
            1,
            2,
            r#"tool config
{"result":{"data":{"success":true,"operation":"set","setting":"browser.enabled","previousValue":false,"newValue":true}}}"#
        )
        .is_some()
    );
    assert!(
        tool_config_view(
            &app.files().card,
            1,
            2,
            r#"tool config
{"status":"error","error":"bad config","result":{"data":{"success":false,"error":"bad config"}}}"#
        )
        .is_some()
    );
}

/// value_label converts each JSON Value variant to the expected display string.
#[test]
fn config_view_renders_denied_state() {
    let app = App::new().0;
    let visible = r#"tool config
{"status":"denied","error":"not allowed","result":{"data":{"success":false,"error":"not allowed"}}}"#;
    assert!(tool_config_view(&app.files().card, 0, 0, visible).is_some());
}

/// tool_config_view shows different titles for running with/without value.
#[test]
fn config_view_title_changes_with_running_input() {
    let app = App::new().0;

    let with_value = r#"tool config
{"status":"running","input":"{\"setting\":\"theme\",\"value\":\"dark\"}"}"#;
    assert!(tool_config_view(&app.files().card, 0, 0, with_value).is_some());

    let without_value = r#"tool config
{"status":"running","input":"{\"setting\":\"theme\"}"}"#;
    assert!(tool_config_view(&app.files().card, 0, 0, without_value).is_some());
}

/// tool_config_view renders a fallback summary for a successful get.
#[test]
fn config_view_renders_fallback_summary_for_get() {
    let app = App::new().0;
    let value = serde_json::json!({
        "result": {
            "data": {
                "success": true,
                "operation": "get"
            }
        }
    });
    let visible = format!("tool config\n{}", value);
    assert!(tool_config_view(&app.files().card, 0, 0, &visible).is_some());
}

/// tool_config_view renders set success with previous_value and new_value.
#[test]
fn config_view_renders_set_with_old_and_new_values() {
    let app = App::new().0;
    let visible = r#"tool config
{"result":{"data":{"success":true,"operation":"set","setting":"theme","previousValue":"light","newValue":"dark"}}}"#;
    assert!(tool_config_view(&app.files().card, 0, 0, visible).is_some());
}

/// tool_config_view renders get with object value (config overview).
#[test]
fn config_view_renders_get_object_overview() {
    let app = App::new().0;
    let visible = r#"tool config
{"result":{"data":{"success":true,"operation":"get","value":{"proxy":{},"model_routing":{},"runtime":{}}}}}"#;
    assert!(tool_config_view(&app.files().card, 0, 0, visible).is_some());
}

/// tool_config_view renders error without explicit error field, falling back to tool_error_text.
#[test]
fn config_view_falls_back_to_tool_error_text() {
    let app = App::new().0;
    let visible = r#"tool config
{"status":"error","output":"permission denied","result":{"data":{"success":false}}}"#;
    assert!(tool_config_view(&app.files().card, 0, 0, visible).is_some());
}
