//! toolset 安全上下文测试。
//!
//! 验证 full-access 放开当前工具执行的完整安全策略。

use super::security_for_tool_context;
use crate::app::agent::config::Config;
use crate::app::agent::security::ShellRedirectPolicy;
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
    assert!(full_access_security.forbidden_paths.is_empty());
    assert_eq!(full_access_security.max_actions_per_hour, u32::MAX);
    assert!(!full_access_security.require_approval_for_medium_risk);
    assert!(!full_access_security.block_high_risk_commands);
    assert!(full_access_security.allow_unsafe_shell_patterns);
    assert!(!full_access_security.bash_security_enabled);
}

#[test]
fn full_access_context_allows_high_risk_curl_request() {
    let workspace_dir = PathBuf::from("/tmp/vw-toolset-full-access-workspace");
    let mut config = Config { workspace_dir: workspace_dir.clone(), ..Config::default() };
    config.security.bash.enabled = true;
    let command = "curl --location 'http://example.com/api?access_key=test' \
        --header 'Content-Type: application/x-www-form-urlencoded' \
        --header 'Cookie: session=test-session' \
        --data 'company_id=123&company_name=test'";

    let security =
        security_for_tool_context(&config, &workspace_dir, &ToolUseContext::new("test", None));
    assert!(security.validate_command_execution(command, false).is_err());

    let full_access_context = ToolUseContext::new("test", None).with_full_access_enabled(true);
    let full_access_security =
        security_for_tool_context(&config, &workspace_dir, &full_access_context);

    assert!(full_access_security.validate_command_execution(command, false).is_ok());
}

#[test]
fn full_access_context_allows_shell_paths_outside_workspace() {
    let workspace_dir = PathBuf::from("/tmp/vw-toolset-full-access-workspace");
    let mut config = Config { workspace_dir: workspace_dir.clone(), ..Config::default() };
    config.security.bash.enabled = true;
    let command = "cat /etc/hosts";

    let security =
        security_for_tool_context(&config, &workspace_dir, &ToolUseContext::new("test", None));
    assert!(security.validate_command_execution(command, false).is_err());

    let full_access_context = ToolUseContext::new("test", None).with_full_access_enabled(true);
    let full_access_security =
        security_for_tool_context(&config, &workspace_dir, &full_access_context);

    assert!(full_access_security.validate_command_execution(command, false).is_ok());
}

#[test]
fn full_access_context_bypasses_all_bash_security_checks() {
    let workspace_dir = PathBuf::from("/tmp/vw-toolset-full-access-workspace");
    let mut config = Config { workspace_dir: workspace_dir.clone(), ..Config::default() };
    config.security.bash.enabled = true;
    let command = "for item in {one,two}; do ls /Users/xiangmin/.vibewindow/skills/$item; done";

    let security =
        security_for_tool_context(&config, &workspace_dir, &ToolUseContext::new("test", None));
    assert!(security.validate_command_execution(command, false).is_err());

    let full_access_context = ToolUseContext::new("test", None).with_full_access_enabled(true);
    let full_access_security =
        security_for_tool_context(&config, &workspace_dir, &full_access_context);

    assert!(full_access_security.validate_command_execution(command, false).is_ok());
}

#[test]
fn full_access_context_preserves_and_allows_redirected_shell_command() {
    let workspace_dir = PathBuf::from("/tmp/vw-toolset-full-access-workspace");
    let mut config = Config { workspace_dir: workspace_dir.clone(), ..Config::default() };
    config.security.bash.enabled = true;
    let command = "printf redirected > /tmp/vw-full-access-output";
    let full_access_context = ToolUseContext::new("test", None).with_full_access_enabled(true);

    let security = security_for_tool_context(&config, &workspace_dir, &full_access_context);

    assert_eq!(security.shell_redirect_policy, ShellRedirectPolicy::Allow);
    assert_eq!(security.apply_shell_redirect_policy(command), command);
    assert!(security.validate_command_execution(command, false).is_ok());
}
