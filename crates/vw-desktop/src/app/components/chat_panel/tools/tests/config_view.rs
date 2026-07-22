//! config_view.rs 测试模块。
//!
//! 这些测试固定相邻解析器、视图辅助函数或状态计算的行为，防止后续 UI 重排时破坏边界契约。

use serde_json::json;

/// 重新导出 use super::{parse_config_input, parse_config_result_from_output, summary_from_input, summary_from_result}，让上层模块通过稳定路径访问。
use super::{
    parse_config_input, parse_config_result, parse_config_result_from_output, success_body,
    summary_from_input, summary_from_result, tool_config_view, value_label,
};
use crate::app::App;

/// 解析 config result from output reads get shape 的输入文本，返回后续视图可以直接消费的结构化结果。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn parse_config_result_from_output_reads_get_shape() {
    let result = parse_config_result_from_output(
        r#"{"success":true,"operation":"get","setting":"browser.enabled","value":true}"#,
    )
    .expect("config get payload should parse");

    assert_eq!(result.setting.as_deref(), Some("browser.enabled"));
    assert_eq!(result.value, Some(json!(true)));
    assert_eq!(summary_from_result(&result), "browser.enabled = true");
}

#[test]
fn parse_config_result_from_output_rejects_non_json_text() {
    assert!(parse_config_result_from_output("plain text").is_none());
    assert!(parse_config_result_from_output("{not-json").is_none());
}

/// 解析 config result from output reads set shape 的输入文本，返回后续视图可以直接消费的结构化结果。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn parse_config_result_from_output_reads_set_shape() {
    let result = parse_config_result_from_output(
        r#"{"success":true,"operation":"set","setting":"appUi.terminalTheme","previousValue":"system","newValue":"monokai"}"#,
    )
    .expect("config set payload should parse");

    assert_eq!(result.previous_value, Some(json!("system")));
    assert_eq!(result.new_value, Some(json!("monokai")));
    assert_eq!(summary_from_result(&result), "appUi.terminalTheme -> monokai");
}

#[test]
fn summary_from_result_handles_failures_and_overview_gets() {
    let failed = parse_config_result_from_output(
        r#"{"success":false,"operation":"set","error":"permission denied"}"#,
    )
    .expect("config failure payload should parse");
    assert_eq!(summary_from_result(&failed), "permission denied");

    let overview =
        parse_config_result_from_output(r#"{"success":true,"operation":"get","value":{"proxy":{},"model_routing":{},"runtime":{}}}"#)
            .expect("config overview should parse");
    assert_eq!(summary_from_result(&overview), "当前配置概览");
}

/// 解析 config input reads pending write request 的输入文本，返回后续视图可以直接消费的结构化结果。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn parse_config_input_reads_pending_write_request() {
    let input = parse_config_input(r#"{"setting":"appUi.terminalTheme","value":"monokai"}"#)
        .expect("config input should parse");

    assert_eq!(input.setting.as_deref(), Some("appUi.terminalTheme"));
    assert_eq!(input.value, Some(json!("monokai")));
    assert_eq!(summary_from_input(&input), "设置 appUi.terminalTheme = monokai");
}

#[test]
fn summary_from_input_handles_reads_and_sections() {
    let read = parse_config_input(r#"{"setting":"browser.enabled"}"#)
        .expect("config read input should parse");
    assert_eq!(summary_from_input(&read), "读取 browser.enabled");

    let proxy =
        parse_config_input(r#"{"section":"proxy"}"#).expect("config section input should parse");
    assert_eq!(summary_from_input(&proxy), "读取 proxy 高级配置");

    let overview = parse_config_input(r#"{}"#).expect("empty config input should parse");
    assert_eq!(summary_from_input(&overview), "读取配置概览");
    assert!(parse_config_input("not-json").is_none());
}

#[test]
fn config_view_rejects_bad_tool_and_empty_completed_result() {
    let app = App::new().0;

    assert!(tool_config_view(&app, 0, 0, "tool bash\n{}").is_none());
    assert!(tool_config_view(&app, 0, 0, "tool config\nnot-json").is_none());
    assert!(
        tool_config_view(
            &app,
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
            &app,
            1,
            2,
            r#"tool config
{"status":"running","input":"{\"setting\":\"browser.enabled\"}"}"#
        )
        .is_some()
    );
    assert!(
        tool_config_view(
            &app,
            1,
            2,
            r#"tool config
{"result":{"data":{"success":true,"operation":"set","setting":"browser.enabled","previousValue":false,"newValue":true}}}"#
        )
        .is_some()
    );
    assert!(
        tool_config_view(
            &app,
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
fn value_label_converts_all_value_types() {
    assert_eq!(value_label(&serde_json::Value::Null), "default");
    assert_eq!(value_label(&serde_json::json!("hello")), "hello");
    assert_eq!(value_label(&serde_json::json!(true)), "true");
    assert_eq!(value_label(&serde_json::json!(false)), "false");
    assert_eq!(value_label(&serde_json::json!(42)), "42");
    assert_eq!(value_label(&serde_json::json!(3.14)), "3.14");

    let arr = serde_json::json!([1, 2, 3]);
    let label = value_label(&arr);
    assert_eq!(label, "[1,2,3]");

    let obj = serde_json::json!({"key": "val"});
    let label = value_label(&obj);
    assert_eq!(label, r#"{"key":"val"}"#);
}

/// parse_config_result reads from tool_result_data path then falls back to output text.
#[test]
fn parse_config_result_reads_from_result_data_path() {
    let value = serde_json::json!({
        "result": {
            "data": {
                "success": true,
                "operation": "get",
                "setting": "theme",
                "value": "dark"
            }
        }
    });
    let result = parse_config_result(&value).expect("should parse from result.data");
    assert_eq!(result.setting.as_deref(), Some("theme"));
    assert_eq!(result.value, Some(serde_json::json!("dark")));
}

#[test]
fn parse_config_result_falls_back_to_output_text() {
    let value = serde_json::json!({
        "output": r#"{"success":false,"operation":"set","error":"timeout"}"#
    });
    let result = parse_config_result(&value).expect("should fall back to output");
    assert!(!result.success);
    assert_eq!(result.error.as_deref(), Some("timeout"));
}

#[test]
fn parse_config_result_returns_none_when_no_parseable_data() {
    let value = serde_json::json!({"status": "completed", "output": "plain text"});
    assert!(parse_config_result(&value).is_none());

    let empty = serde_json::json!({});
    assert!(parse_config_result(&empty).is_none());
}

#[test]
fn success_body_handles_get_set_and_unknown_operation_display_boundaries() {
    let get_missing_value =
        parse_config_result_from_output(r#"{"success":true,"operation":"get","setting":"theme"}"#)
            .expect("get without value");
    assert!(success_body(&get_missing_value).is_none());

    let set_without_values =
        parse_config_result_from_output(r#"{"success":true,"operation":"set","setting":"theme"}"#)
            .expect("set without values");
    assert!(success_body(&set_without_values).is_some());

    let unknown_operation =
        parse_config_result_from_output(r#"{"success":true,"operation":"delete"}"#)
            .expect("unknown operation");
    assert!(success_body(&unknown_operation).is_none());
}

/// summary_from_result edge cases: fallback operation labels.
#[test]
fn summary_from_result_fallback_labels() {
    let set_no_details = parse_config_result_from_output(r#"{"success":true,"operation":"set"}"#)
        .expect("set without details");
    assert_eq!(summary_from_result(&set_no_details), "配置已更新");

    let unknown_op = parse_config_result_from_output(r#"{"success":true,"operation":"delete"}"#)
        .expect("delete operation");
    assert_eq!(summary_from_result(&unknown_op), "配置");

    let no_op = parse_config_result_from_output(r#"{"success":true}"#).expect("no operation");
    assert_eq!(summary_from_result(&no_op), "配置");
}

/// summary_from_input edge cases: empty setting, model_routing section.
#[test]
fn summary_from_input_edge_cases() {
    let empty_setting = parse_config_input(r#"{"setting":"  "}"#).expect("whitespace-only setting");
    assert_eq!(summary_from_input(&empty_setting), "读取配置概览");

    let model_routing =
        parse_config_input(r#"{"section":"model_routing"}"#).expect("model_routing section");
    assert_eq!(summary_from_input(&model_routing), "读取 model_routing 高级配置");
}

/// tool_config_view renders denied status correctly.
#[test]
fn config_view_renders_denied_state() {
    let app = App::new().0;
    let visible = r#"tool config
{"status":"denied","error":"not allowed","result":{"data":{"success":false,"error":"not allowed"}}}"#;
    assert!(tool_config_view(&app, 0, 0, visible).is_some());
}

/// tool_config_view shows different titles for running with/without value.
#[test]
fn config_view_title_changes_with_running_input() {
    let app = App::new().0;

    let with_value = r#"tool config
{"status":"running","input":"{\"setting\":\"theme\",\"value\":\"dark\"}"}"#;
    assert!(tool_config_view(&app, 0, 0, with_value).is_some());

    let without_value = r#"tool config
{"status":"running","input":"{\"setting\":\"theme\"}"}"#;
    assert!(tool_config_view(&app, 0, 0, without_value).is_some());
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
    assert!(tool_config_view(&app, 0, 0, &visible).is_some());
}

/// tool_config_view renders set success with previous_value and new_value.
#[test]
fn config_view_renders_set_with_old_and_new_values() {
    let app = App::new().0;
    let visible = r#"tool config
{"result":{"data":{"success":true,"operation":"set","setting":"theme","previousValue":"light","newValue":"dark"}}}"#;
    assert!(tool_config_view(&app, 0, 0, visible).is_some());
}

/// tool_config_view renders get with object value (config overview).
#[test]
fn config_view_renders_get_object_overview() {
    let app = App::new().0;
    let visible = r#"tool config
{"result":{"data":{"success":true,"operation":"get","value":{"proxy":{},"model_routing":{},"runtime":{}}}}}"#;
    assert!(tool_config_view(&app, 0, 0, visible).is_some());
}

/// tool_config_view renders error without explicit error field, falling back to tool_error_text.
#[test]
fn config_view_falls_back_to_tool_error_text() {
    let app = App::new().0;
    let visible = r#"tool config
{"status":"error","output":"permission denied","result":{"data":{"success":false}}}"#;
    assert!(tool_config_view(&app, 0, 0, visible).is_some());
}
