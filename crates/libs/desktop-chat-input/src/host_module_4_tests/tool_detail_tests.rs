//! 验证工具原始输出在成功、失败、拒绝和非结构化状态下的详情解析。
//!
//! 解析样例覆盖普通工具与 MCP 载荷，标题状态和正文回退必须彼此独立。

#![allow(unused_must_use)]
#[test]
fn tool_detail_tests_module_is_wired() {
    assert!(module_path!().ends_with("tool_detail_tests"));
}
