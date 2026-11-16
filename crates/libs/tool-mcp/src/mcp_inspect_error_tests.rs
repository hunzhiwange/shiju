//! MCP 检查错误摘要的库内断言，无宿主或外部资源依赖。

use super::summarize_error;

#[test]
fn summarize_error_uses_the_outer_context() {
    let error = anyhow::anyhow!("connection refused").context("failed to inspect server");

    assert_eq!(summarize_error(&error), "failed to inspect server");
}
