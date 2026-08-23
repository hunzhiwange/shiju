//! 验证脚本运行结果解析、错误脱敏及 JavaScript 与 Python 执行契约。

use super::{
    javascript_script, parse_last_json_line, python_script, run_code_node, run_script,
    sanitize_error_text, value_to_output_map,
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

#[test]
fn parse_last_json_line_skips_noise_and_uses_latest_envelope() {
    let envelope = parse_last_json_line(
        "diagnostic\n{not json}\n{\"ok\":true,\"result\":{\"answer\":42}}\n\n",
    )
    .expect("valid envelope");

    assert!(envelope.ok);
    assert_eq!(envelope.result, Some(json!({"answer": 42})));
    assert!(parse_last_json_line("noise only\n").is_none());
}

#[test]
fn value_to_output_map_wraps_scalars_and_preserves_objects() {
    assert_eq!(value_to_output_map(json!({"answer": 42}))["answer"], json!(42));
    assert_eq!(value_to_output_map(Value::Null)["result"], Value::Null);
}

#[test]
fn sanitize_error_text_redacts_nested_sensitive_values_only() {
    let inputs = BTreeMap::from([
        ("api_key".to_string(), json!({"values": ["secret-a", "secret-b"]})),
        ("message".to_string(), Value::String("public".into())),
    ]);

    let sanitized = sanitize_error_text("secret-a secret-b public".to_string(), &inputs);

    assert_eq!(sanitized, "[REDACTED] [REDACTED] public");
}

#[test]
fn generated_scripts_include_user_code_and_required_entrypoint_checks() {
    let python = python_script("def main(**inputs): return inputs");
    let javascript = javascript_script("module.exports.main = async inputs => inputs;");

    assert!(python.contains("def main(**inputs): return inputs"));
    assert!(python.contains("workflow HTTP bridge token is unavailable"));
    assert!(javascript.contains("module.exports.main = async inputs => inputs;"));
    assert!(javascript.contains("main function is required"));
}

#[tokio::test]
async fn run_code_node_rejects_unsupported_language_without_spawning() {
    let error = run_code_node(" ruby ", "puts 'hi'", &BTreeMap::new())
        .await
        .expect_err("unsupported language");

    assert_eq!(error, "不支持的 code_language: ruby");
}

#[tokio::test]
async fn run_code_node_executes_javascript_and_returns_object_outputs() {
    let inputs = BTreeMap::from([("left".to_string(), json!(2)), ("right".to_string(), json!(3))]);

    let outputs = run_code_node(
        " JavaScript ",
        "function main(inputs) { return { sum: inputs.left + inputs.right }; }",
        &inputs,
    )
    .await
    .expect("javascript code should execute");

    assert_eq!(outputs["sum"], json!(5));
}

#[tokio::test]
async fn run_code_node_redacts_sensitive_javascript_error_output() {
    let inputs =
        BTreeMap::from([("api_key".to_string(), json!({"nested": ["secret-token-value"]}))]);

    let error = run_code_node(
        "node",
        "function main(inputs) { throw new Error(inputs.api_key.nested[0]); }",
        &inputs,
    )
    .await
    .expect_err("javascript error should fail");

    assert!(error.contains("[REDACTED]"), "{error}");
    assert!(!error.contains("secret-token-value"), "{error}");
}

#[tokio::test]
async fn run_code_node_executes_python_and_wraps_scalar_output() {
    let inputs = BTreeMap::from([("value".to_string(), json!(21))]);

    let outputs =
        run_code_node("python", "def main(**inputs):\n    return inputs['value'] * 2", &inputs)
            .await
            .expect("python code should execute");

    assert_eq!(outputs["result"], json!(42));
}

#[tokio::test]
async fn run_script_reports_sanitized_output_when_no_json_envelope() {
    let inputs =
        BTreeMap::from([("api_key".to_string(), Value::String("secret-token-value".into()))]);

    let error = run_script("node", ".js", "console.log('secret-token-value')", &inputs, &[])
        .await
        .expect_err("missing JSON envelope should fail");

    assert!(error.contains("code 节点没有返回合法 JSON"), "{error}");
    assert!(error.contains("[REDACTED]"), "{error}");
    assert!(!error.contains("secret-token-value"), "{error}");
}

#[tokio::test]
async fn run_script_reports_spawn_failure_for_missing_program() {
    let error = run_script(
        "vw-missing-workflow-runtime-for-test",
        ".js",
        "console.log(JSON.stringify({ ok: true }))",
        &BTreeMap::new(),
        &[],
    )
    .await
    .expect_err("missing runtime should fail");

    assert!(error.contains("启动 vw-missing-workflow-runtime-for-test 失败"), "{error}");
}
