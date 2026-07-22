//! toolset 安全上下文测试。
//!
//! 验证 full-access 放开当前工具执行所需的工作区与命令白名单判定，但不削弱
//! forbidden path 等更高优先级的保护。

use super::security_for_tool_context;
use crate::app::agent::config::Config;
use crate::app::agent::tools::ToolUseContext;
use std::path::PathBuf;

#[test]
fn full_access_context_disables_workspace_only_for_tool_security() {
    let workspace_dir = PathBuf::from("/tmp/vw-toolset-full-access-workspace");
    let config = Config { workspace_dir: workspace_dir.clone(), ..Config::default() };

    let security =
        security_for_tool_context(&config, &workspace_dir, &ToolUseContext::new("test", None));
    assert!(security.workspace_only, "default tool security should remain workspace_only");

    let full_access_context = ToolUseContext::new("test", None).with_full_access_enabled(true);
    let full_access_security =
        security_for_tool_context(&config, &workspace_dir, &full_access_context);

    assert!(
        !full_access_security.workspace_only,
        "full access must disable workspace_only for the current tool execution context"
    );
    assert_eq!(
        full_access_security.forbidden_paths, security.forbidden_paths,
        "full access should not weaken forbidden path protections"
    );
}

#[test]
fn full_access_context_allows_commands_outside_configured_allowlist() {
    let workspace_dir = PathBuf::from("/tmp/vw-toolset-full-access-workspace");
    let config = Config { workspace_dir: workspace_dir.clone(), ..Config::default() };

    let security =
        security_for_tool_context(&config, &workspace_dir, &ToolUseContext::new("test", None));
    assert!(!security.is_command_allowed("curl https://example.com"));

    let full_access_context = ToolUseContext::new("test", None).with_full_access_enabled(true);
    let full_access_security =
        security_for_tool_context(&config, &workspace_dir, &full_access_context);

    assert!(full_access_security.is_command_allowed("curl https://example.com"));
}
