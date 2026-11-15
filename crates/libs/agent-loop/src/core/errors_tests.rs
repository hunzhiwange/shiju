//! 测试工具循环取消与迭代上限错误的稳定识别规则。
//! 错误链和纯文本伪装分别测试，避免依赖脆弱的字符串包含判断。

use super::*;

#[test]
fn tool_loop_cancelled_displays_stable_message() {
    assert_eq!(ToolLoopCancelled.to_string(), "tool loop cancelled");
}

#[test]
fn is_tool_loop_cancelled_detects_direct_cancel_error() {
    let cancelled = anyhow::Error::new(ToolLoopCancelled);

    assert!(is_tool_loop_cancelled(&cancelled));
}

#[test]
fn is_tool_loop_cancelled_detects_cancel_error_in_chain() {
    let cancelled = anyhow::Error::new(ToolLoopCancelled).context("outer failure");

    assert!(is_tool_loop_cancelled(&cancelled));
}

#[test]
fn is_tool_loop_cancelled_rejects_unrelated_error() {
    let other = anyhow::anyhow!("different failure");

    assert!(!is_tool_loop_cancelled(&other));
}

#[test]
fn is_tool_loop_cancelled_rejects_text_only_cancel_message() {
    let other = anyhow::anyhow!("tool loop cancelled");

    assert!(!is_tool_loop_cancelled(&other));
}

#[test]
fn is_tool_iteration_limit_error_detects_direct_message() {
    let limit = anyhow::anyhow!("Agent exceeded maximum tool iterations");

    assert!(is_tool_iteration_limit_error(&limit));
}

#[test]
fn is_tool_iteration_limit_error_detects_message_in_chain() {
    let limit =
        anyhow::anyhow!("Agent exceeded maximum tool iterations (100)").context("outer failure");

    assert!(is_tool_iteration_limit_error(&limit));
}

#[test]
fn is_tool_iteration_limit_error_detects_outer_context_message() {
    let limit = anyhow::anyhow!("inner failure").context("Agent exceeded maximum tool iterations");

    assert!(is_tool_iteration_limit_error(&limit));
}

#[test]
fn is_tool_iteration_limit_error_rejects_unrelated_error() {
    let other = anyhow::anyhow!("different failure");

    assert!(!is_tool_iteration_limit_error(&other));
}

#[test]
fn is_tool_iteration_limit_error_rejects_case_mismatch() {
    let other = anyhow::anyhow!("agent exceeded maximum tool iterations");

    assert!(!is_tool_iteration_limit_error(&other));
}
