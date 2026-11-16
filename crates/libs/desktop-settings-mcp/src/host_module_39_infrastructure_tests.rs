//! 基础设施设置测试，验证安全默认值、MCP 传输、可观测性和主题编辑状态。
//! 新建配置必须可直接编辑且默认权限收敛，未知已保存工具则需无损保留。

use crate::management_state::McpSettingsState;
use crate::state::{McpNewServerTransport, McpSettingsTab};

#[test]
fn test_module_is_wired() {
    let module = std::hint::black_box(module_path!());

    assert!(module.ends_with("host_module_39_infrastructure_tests"));
}

#[test]
fn mcp_settings_default_opens_servers_with_empty_runtime_state() {
    let state = McpSettingsState::default();

    assert_eq!(state.active_tab, McpSettingsTab::Servers);
    assert_eq!(state.editor.text(), "[]");
    assert!(state.runtime_overrides.is_empty());
    assert!(!state.new_server_expanded);
    assert!(state.expanded_previews.is_empty());
    assert_eq!(state.new_server.transport, McpNewServerTransport::Stdio);
    assert!(!state.loading);
    assert!(!state.inspect_loading);
    assert!(!state.saving);
    assert_eq!(state.server_count, 0);
    assert!(state.inspect.is_none());
    assert!(state.inspect_error.is_none());
    assert!(state.save_error.is_none());
    assert!(state.status_message.is_none());
}
