use super::{ApprovalRequest, ApprovalResponse, summarize_args};
use std::io::{self, BufRead, IsTerminal, Write};

/// 显示审批提示并从标准输入读取用户响应。
pub(super) fn prompt_cli_interactive(request: &ApprovalRequest) -> ApprovalResponse {
    if prompt_cli_should_default_to_no_without_reading_stdin() {
        return ApprovalResponse::No;
    }

    let stdin = io::stdin();
    let stderr = io::stderr();
    prompt_cli_with_io(request, stdin.lock(), stderr.lock())
}

fn prompt_cli_should_default_to_no_without_reading_stdin() -> bool {
    #[cfg(test)]
    {
        true
    }

    #[cfg(not(test))]
    {
        !io::stdin().is_terminal()
    }
}

pub(super) fn prompt_cli_with_io(
    request: &ApprovalRequest,
    mut reader: impl BufRead,
    mut writer: impl Write,
) -> ApprovalResponse {
    let summary = summarize_args(&request.arguments);

    let _ = writeln!(writer);
    let _ = writeln!(writer, "🔧 Agent wants to execute: {}", request.tool_name);
    let _ = writeln!(writer, "   {summary}");
    let _ = write!(writer, "   [Y]es / [N]o / [A]lways for {}: ", request.tool_name);
    let _ = writer.flush();

    let mut line = String::new();
    if reader.read_line(&mut line).is_err() {
        return ApprovalResponse::No;
    }

    parse_cli_response(&line)
}

pub(super) fn parse_cli_response(line: &str) -> ApprovalResponse {
    match line.trim().to_ascii_lowercase().as_str() {
        "y" | "yes" => ApprovalResponse::Yes,
        "a" | "always" => ApprovalResponse::Always,
        _ => ApprovalResponse::No,
    }
}
