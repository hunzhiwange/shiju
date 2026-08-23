//! 验证严格 JSON 输出策略对错误和结果载荷的发射条件。
//!
//! 命令解析、内存输出与失败 fixture 共同固定公开 CLI 的可观察契约。

use serde::Serialize;
use serde_json::Value;

use crate::{OutputErrorCode, OutputErrorParams, OutputFormat, OutputPolicy};

use super::*;

#[derive(Serialize)]
struct Payload<'a> {
    value: &'a str,
}

fn policy(json_strict: bool) -> OutputPolicy {
    OutputPolicy {
        format: OutputFormat::Json,
        json_strict,
        suppress_reads: false,
        suppress_non_json_stderr: json_strict,
        queue_error_already_emitted: true,
        suppress_sdk_console_errors: json_strict,
    }
}

// 下列行为用例同时检查格式分支、参数边界和错误传播。
#[test]
fn write_output_error_emits_only_for_json_strict_policy() {
    let error = OutputErrorParams {
        code: OutputErrorCode::Runtime,
        detail_code: None,
        origin: None,
        message: "failed".to_string(),
        retryable: None,
        acp: None,
        timestamp: None,
    };
    let mut output = Vec::new();

    write_output_error(&mut output, &error, &policy(false)).expect("non strict write");
    assert!(output.is_empty());

    write_output_error(&mut output, &error, &policy(true)).expect("strict write");
    let value: Value = serde_json::from_slice(&output).expect("json error");

    assert_eq!(value["error"]["code"], "RUNTIME");
    assert_eq!(value["error"]["message"], "failed");
}

#[test]
fn emit_json_result_returns_whether_it_wrote_payload() {
    let mut text_output = Vec::new();
    let wrote_text =
        emit_json_result(&mut text_output, OutputFormat::Text, &Payload { value: "x" }).unwrap();

    assert!(!wrote_text);
    assert!(text_output.is_empty());

    let mut json_output = Vec::new();
    let wrote_json =
        emit_json_result(&mut json_output, OutputFormat::Json, &Payload { value: "x" }).unwrap();

    assert!(wrote_json);
    assert_eq!(String::from_utf8(json_output).unwrap(), "{\"value\":\"x\"}\n");
}
