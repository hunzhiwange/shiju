//! 验证网关配置的地址解析、默认值和环境覆盖规则。
//!
//! 地址样例覆盖主机、端口、协议和环境来源，非法输入必须走明确回退。

#[test]
fn interactive_config_paths_reject_blocking_gateway_wrappers() {
    // Cover the entry points and persistence helpers migrated by the desktop config plan.
    // Session, Git and MCP operations have separate latency boundaries.
    let sources = [
        ("App::new", include_str!("impls/app_init/new.rs")),
        ("startup", include_str!("impls/app_init/startup.rs")),
        ("App::update", include_str!("impls/app_update.rs")),
        ("App::view", include_str!("impls/app_view.rs")),
        ("projects", include_str!("projects.rs")),
        ("project preferences", include_str!("config_project_chat.rs")),
        ("chat", include_str!("message/chat/session.rs")),
        ("layout", include_str!("message/view/layout.rs")),
        ("chat popovers", include_str!("message/view/panel/popovers.rs")),
        ("file tree", include_str!("message/project/file_tree.rs")),
        ("task board", include_str!("message/task_board/update.rs")),
        ("theme", include_str!("message/view/theme.rs")),
        ("general settings", include_str!("message/settings/general.rs")),
        ("dialogue flow", include_str!("message/settings/dialogue_flow.rs")),
        ("Gateway settings", include_str!("message/settings/gateway_client.rs")),
    ];
    let blocking_wrappers = [
        "run_gateway_call",
        "block_on_gateway",
        "block_on",
        "set_config_field",
        "save_app_config",
        "load_app_config",
        "load_project_chat_preferences",
        "save_project_chat_preferences",
        "load_system_settings_config",
        "update_system_settings_config",
        "load_full_agent_config",
        "load_full_agent_config_result",
        "load_global_acp_config_result",
        "load_enabled_acp_config_result",
        "load_gateway_config_result",
        "load_tools_list_via_gateway",
        "gateway_load_archived_session_ids",
    ];
    for (name, source) in sources {
        for token in source.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_')) {
            assert!(!blocking_wrappers.contains(&token), "{name} uses blocking {token}");
        }
    }
}
