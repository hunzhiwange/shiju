//! 测试命令行审批的序列化、输入解析和安全拒绝默认值。
//! 故障读取器专门确认交互输入异常时默认拒绝，不因输入错误扩大授权。

use super::{ApprovalRequest, ApprovalResponse};
use std::io::{self, BufRead, Read};

#[test]
fn approval_response_serializes_as_lowercase_values() {
    assert_eq!(serde_json::to_string(&ApprovalResponse::Yes).unwrap(), "\"yes\"");
    assert_eq!(serde_json::to_string(&ApprovalResponse::No).unwrap(), "\"no\"");
    assert_eq!(serde_json::to_string(&ApprovalResponse::Always).unwrap(), "\"always\"");
}

#[test]
fn approval_request_round_trips_tool_and_arguments() {
    let request = ApprovalRequest {
        tool_name: "shell".to_string(),
        arguments: serde_json::json!({"command": "pwd"}),
    };

    let encoded = serde_json::to_string(&request).unwrap();
    let decoded: ApprovalRequest = serde_json::from_str(&encoded).unwrap();

    assert_eq!(decoded.tool_name, "shell");
    assert_eq!(decoded.arguments["command"], "pwd");
}

#[test]
fn parse_cli_response_accepts_yes_always_and_defaults_to_no() {
    assert_eq!(super::cli::parse_cli_response("y\n"), ApprovalResponse::Yes);
    assert_eq!(super::cli::parse_cli_response(" YES "), ApprovalResponse::Yes);
    assert_eq!(super::cli::parse_cli_response("a"), ApprovalResponse::Always);
    assert_eq!(super::cli::parse_cli_response("Always\n"), ApprovalResponse::Always);
    assert_eq!(super::cli::parse_cli_response(""), ApprovalResponse::No);
    assert_eq!(super::cli::parse_cli_response("maybe"), ApprovalResponse::No);
}

#[test]
fn prompt_cli_with_io_writes_prompt_and_reads_response() {
    let request = ApprovalRequest {
        tool_name: "shell".to_string(),
        arguments: serde_json::json!({"command": "pwd"}),
    };
    let mut output = Vec::new();

    let response = super::cli::prompt_cli_with_io(&request, "yes\n".as_bytes(), &mut output);

    let output = String::from_utf8(output).unwrap();
    assert_eq!(response, ApprovalResponse::Yes);
    assert!(output.contains("Agent wants to execute: shell"));
    assert!(output.contains("command: pwd"));
    assert!(output.contains("[Y]es / [N]o / [A]lways for shell: "));
}

#[test]
fn prompt_cli_with_io_defaults_to_no_on_read_error() {
    let request = ApprovalRequest {
        tool_name: "file_write".to_string(),
        arguments: serde_json::json!({"path": "notes.md"}),
    };
    let mut output = Vec::new();

    let response = super::cli::prompt_cli_with_io(&request, ErrorReader, &mut output);

    assert_eq!(response, ApprovalResponse::No);
    assert!(!output.is_empty());
}

#[test]
fn prompt_cli_interactive_defaults_to_no_on_noninteractive_stdin() {
    let request = ApprovalRequest {
        tool_name: "shell".to_string(),
        arguments: serde_json::json!({"command": "pwd"}),
    };

    let response = super::cli::prompt_cli_interactive(&request);

    assert_eq!(response, ApprovalResponse::No);
}

struct ErrorReader;

impl Read for ErrorReader {
    fn read(&mut self, _buf: &mut [u8]) -> io::Result<usize> {
        Err(io::Error::other("read failed"))
    }
}

impl BufRead for ErrorReader {
    fn fill_buf(&mut self) -> io::Result<&[u8]> {
        Err(io::Error::other("read failed"))
    }

    fn consume(&mut self, _amt: usize) {}
}
