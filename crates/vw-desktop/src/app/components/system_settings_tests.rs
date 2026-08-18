// Tests for plan6 task 827.
const SOURCE: &str = include_str!("system_settings.rs");

use super::system_settings::{
    SystemTab, active_tab_help_modal_open, active_tab_save_error, is_network_tab, search_results,
    system_tab_matches_query, tabbed_page_owns_save_error, view,
};
use super::system_settings_help::help_button_bar;
use super::system_settings_search::SystemSettingsSearchTarget;
use iced::advanced::widget::Tree;

fn tree_node_count(tree: &Tree) -> usize {
    1 + tree.children.iter().map(tree_node_count).sum::<usize>()
}

fn source_declares_symbol(name: &str) -> bool {
    let needles = [
        format!("fn {name}"),
        format!("pub fn {name}"),
        format!("struct {name}"),
        format!("pub struct {name}"),
        format!("enum {name}"),
        format!("pub enum {name}"),
        format!("type {name}"),
        format!("pub type {name}"),
        format!("const {name}"),
        format!("pub const {name}"),
        format!("static {name}"),
        format!("pub static {name}"),
        format!("impl {name}"),
    ];

    needles.iter().any(|needle| SOURCE.contains(needle))
}

#[test]
fn system_settings_tests_keeps_planned_coverage_targets() {
    for name in ["SystemTab", "all", "fmt", "active_tab_help_modal_open", "view"] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}

#[test]
fn system_settings_search_matches_child_items() {
    assert!(system_tab_matches_query(SystemTab::General, "语言"));
    assert!(system_tab_matches_query(SystemTab::Agents, "创建新会话"));
    assert!(system_tab_matches_query(SystemTab::Agents, "接收者"));
    assert!(system_tab_matches_query(SystemTab::Agents, "API 密钥"));
    assert!(system_tab_matches_query(SystemTab::Cron, "最大并发"));
    assert!(system_tab_matches_query(SystemTab::Agents, "初始化"));
    assert!(system_tab_matches_query(SystemTab::Agents, "stdio"));
    assert!(system_tab_matches_query(SystemTab::WebSearch, "Brave 密钥"));
    assert!(system_tab_matches_query(SystemTab::WebSearch, "webdriver"));
    assert!(system_tab_matches_query(SystemTab::HttpRequest, "允许域名"));
}

#[test]
fn system_settings_search_results_extract_matching_child_tabs_with_parent_paths() {
    let app = crate::app::App::new().0;

    let agent_results = search_results(&app, "zntpz");
    assert!(agent_results.iter().any(|entry| {
        entry.label == "智能体"
            && entry.parent_label.is_none()
            && entry.target == SystemSettingsSearchTarget::TopLevel(SystemTab::Agents)
    }));
    assert!(agent_results.iter().any(|entry| {
        entry.parent_label == Some("智能体")
            && matches!(entry.target, SystemSettingsSearchTarget::Agents(_))
    }));

    let stdio_results = search_results(&app, "stdio");
    assert_eq!(stdio_results.len(), 1);
    assert_eq!(stdio_results[0].label, "全局服务器");
    assert_eq!(stdio_results[0].parent_label, Some("智能体 / MCP"));
    assert!(matches!(
        stdio_results[0].target,
        SystemSettingsSearchTarget::AgentMcp(_)
    ));

    let slack_results = search_results(&app, "slack");
    assert!(slack_results.iter().any(|entry| {
        entry.label == "Slack"
            && entry.parent_label == Some("智能体 / 通道")
            && matches!(entry.target, SystemSettingsSearchTarget::Channels(_))
    }));

    let proxy_results = search_results(&app, "all_proxy");
    assert_eq!(proxy_results.len(), 1);
    assert_eq!(proxy_results[0].label, "代理");
    assert_eq!(proxy_results[0].parent_label, Some("网络配置"));
    assert_eq!(
        proxy_results[0].target,
        SystemSettingsSearchTarget::Network(SystemTab::Proxy)
    );

    let tunnel_results = search_results(&app, "tailscale");
    assert_eq!(tunnel_results.len(), 1);
    assert_eq!(tunnel_results[0].label, "隧道");
    assert_eq!(tunnel_results[0].parent_label, Some("网络配置"));
    assert_eq!(
        tunnel_results[0].target,
        SystemSettingsSearchTarget::Network(SystemTab::Tunnel)
    );

    let theme_results = search_results(&app, "gallery");
    assert_eq!(theme_results.len(), 1);
    assert_eq!(theme_results[0].label, "主题");
    assert_eq!(theme_results[0].parent_label, Some("常规设置"));
    assert_eq!(
        theme_results[0].target,
        SystemSettingsSearchTarget::General(crate::app::state::GeneralSettingsTab::Themes)
    );
}

#[test]
fn system_tab_all_contains_every_display_label_once_in_order() {
    let tabs = SystemTab::all();

    assert_eq!(tabs.len(), 15);
    assert_eq!(tabs.first(), Some(&SystemTab::General));
    assert_eq!(tabs.get(1), Some(&SystemTab::Providers));
    assert_eq!(tabs.last(), Some(&SystemTab::Observability));
    assert!(!tabs.contains(&SystemTab::Proxy));
    assert!(!tabs.contains(&SystemTab::Tunnel));
    assert!(!tabs.contains(&SystemTab::Channels));
    assert!(!tabs.contains(&SystemTab::Runtime));
    assert!(!tabs.contains(&SystemTab::HttpRequest));
    assert!(!tabs.contains(&SystemTab::Browser));

    let labels = tabs.iter().map(ToString::to_string).collect::<Vec<_>>();
    assert!(labels.contains(&"常规设置".to_string()));
    assert!(!labels.contains(&"主题".to_string()));
    assert!(!labels.contains(&"对话流".to_string()));
    assert!(!labels.contains(&"快捷消息".to_string()));
    assert!(!labels.contains(&"心跳配置".to_string()));
    assert!(!labels.contains(&"ACP 配置".to_string()));
    assert!(!labels.contains(&"MCP 配置".to_string()));
    assert!(!labels.contains(&"技能配置".to_string()));
    assert!(!labels.contains(&"可靠性配置".to_string()));
    assert!(labels.contains(&"智能体".to_string()));
    assert!(!labels.contains(&"模型路由".to_string()));
    assert!(!labels.contains(&"查询模型分类".to_string()));
    assert!(!labels.contains(&"多模态配置".to_string()));
    assert!(!labels.contains(&"调度配置".to_string()));
    assert!(!labels.contains(&"存储配置".to_string()));
}

#[test]
fn system_tab_search_is_trimmed_case_insensitive_and_empty_matches_all() {
    assert!(system_tab_matches_query(SystemTab::Providers, "  API KEY "));
    assert!(system_tab_matches_query(SystemTab::Agents, "CODEX"));
    assert!(system_tab_matches_query(SystemTab::Agents, "MCP"));
    assert!(system_tab_matches_query(SystemTab::General, "   "));
    assert!(!system_tab_matches_query(SystemTab::General, "definitely-not-a-tab"));
}

#[test]
fn system_tab_display_labels_cover_every_tab_in_navigation_order() {
    let expected = [
        "常规设置",
        "提供商",
        "模型",
        "智能体",
        "员工配置",
        "定时任务",
        "记忆配置",
        "项目",
        "客服端网关",
        "服务端网关",
        "网络配置",
        "安全配置",
        "自治配置",
        "钩子配置",
        "可观测性",
    ];

    for (tab, expected_label) in SystemTab::all().into_iter().zip(expected) {
        assert_eq!(tab.to_string(), expected_label);
    }
}

#[test]
fn network_settings_groups_five_internal_tabs() {
    assert!(is_network_tab(SystemTab::WebSearch));
    assert!(is_network_tab(SystemTab::HttpRequest));
    assert!(is_network_tab(SystemTab::Browser));
    assert!(is_network_tab(SystemTab::Proxy));
    assert!(is_network_tab(SystemTab::Tunnel));
}

#[test]
fn system_tab_search_metadata_covers_every_tab() {
    let cases = [
        (SystemTab::General, "editor"),
        (SystemTab::General, "gallery"),
        (SystemTab::Projects, "workspace"),
        (SystemTab::Providers, "catalog"),
        (SystemTab::Models, "vision"),
        (SystemTab::Memory, "sqlite"),
        (SystemTab::Cron, "max_concurrent"),
        (SystemTab::Agents, "subagent"),
        (SystemTab::Agents, "reasoning summary"),
        (SystemTab::Agents, "quick messages"),
        (SystemTab::Agents, "monitor"),
        (SystemTab::Staff, "roster"),
        (SystemTab::Agents, "retry"),
        (SystemTab::Agents, "slack"),
        (SystemTab::Agents, "docker"),
        (SystemTab::Agents, "protocol"),
        (SystemTab::Agents, "allowedtools"),
        (SystemTab::Autonomy, "approval"),
        (SystemTab::Security, "canary"),
        (SystemTab::GatewayClient, "connection"),
        (SystemTab::Gateway, "webhook"),
        (SystemTab::Observability, "otel"),
        (SystemTab::Proxy, "all_proxy"),
        (SystemTab::Tunnel, "tailscale"),
        (SystemTab::Agents, "plugins"),
        (SystemTab::Hooks, "command_logger"),
        (SystemTab::WebSearch, "duckduckgo"),
        (SystemTab::HttpRequest, "allowlist"),
        (SystemTab::Browser, "webdriver"),
    ];

    for (tab, query) in cases {
        assert!(system_tab_matches_query(tab, query), "{tab:?} should match {query:?}");
    }
}

#[test]
fn active_tab_help_modal_recognizes_shared_and_tab_specific_help() {
    let mut app = crate::app::App::new().0;

    assert!(!active_tab_help_modal_open(&app, SystemTab::General));
    app.system_settings_help_tab = Some(SystemTab::General);
    assert!(active_tab_help_modal_open(&app, SystemTab::General));
    assert!(!active_tab_help_modal_open(&app, SystemTab::Providers));
    app.system_settings_help_tab = None;

    app.heartbeat_settings.show_help_modal = true;
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Heartbeat;
    assert!(active_tab_help_modal_open(&app, SystemTab::Agents));
    app.cron_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Cron));
    app.cron_settings.show_help_modal = false;
    app.scheduler_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Cron));
    app.reliability_settings.show_help_modal = true;
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Reliability;
    assert!(active_tab_help_modal_open(&app, SystemTab::Agents));
    app.autonomy_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Autonomy));
    app.security_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Security));
    app.gateway_client_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::GatewayClient));
    app.gateway_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Gateway));
    app.observability_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Observability));
    app.proxy_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Proxy));
    app.skills_settings.show_help_modal = true;
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Skills;
    assert!(active_tab_help_modal_open(&app, SystemTab::Agents));
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Runtime;
    app.system_settings_help_tab = Some(SystemTab::Runtime);
    assert!(active_tab_help_modal_open(&app, SystemTab::Agents));
    app.system_settings_help_tab = Some(SystemTab::WebSearch);
    assert!(active_tab_help_modal_open(&app, SystemTab::WebSearch));
    app.system_settings_help_tab = None;
}

#[test]
fn active_tab_help_modal_matches_shared_help_tab_for_every_category() {
    let mut app = crate::app::App::new().0;

    for tab in SystemTab::all() {
        app.system_settings_help_tab = Some(tab);
        assert_eq!(
            active_tab_help_modal_open(&app, tab),
            help_button_bar(tab).is_some(),
            "{tab:?} help availability should match its content"
        );

        let other_tab =
            if tab == SystemTab::General { SystemTab::Browser } else { SystemTab::General };
        assert!(
            !active_tab_help_modal_open(&app, other_tab),
            "{tab:?} help should not leak into {other_tab:?}"
        );
    }
}

#[test]
fn active_tab_help_modal_does_not_open_static_tabs_from_tab_specific_flags() {
    let mut app = crate::app::App::new().0;
    app.heartbeat_settings.show_help_modal = true;
    app.cron_settings.show_help_modal = true;
    app.scheduler_settings.show_help_modal = true;
    app.reliability_settings.show_help_modal = true;
    app.autonomy_settings.show_help_modal = true;
    app.security_settings.show_help_modal = true;
    app.gateway_client_settings.show_help_modal = true;
    app.gateway_settings.show_help_modal = true;
    app.observability_settings.show_help_modal = true;
    app.proxy_settings.show_help_modal = true;
    app.skills_settings.show_help_modal = true;
    app.web_search_settings.show_help_modal = true;

    for tab in [
        SystemTab::General,
        SystemTab::Agents,
        SystemTab::Editor,
        SystemTab::Projects,
        SystemTab::Providers,
        SystemTab::Tunnel,
        SystemTab::Hooks,
        SystemTab::WebSearch,
        SystemTab::HttpRequest,
        SystemTab::Browser,
    ] {
        assert!(
            !active_tab_help_modal_open(&app, tab),
            "{tab:?} should not have tab-specific help"
        );
    }
}

#[test]
fn active_tab_save_error_routes_each_settings_state_and_rejects_static_tabs() {
    let mut app = crate::app::App::new().0;

    macro_rules! assert_error_route {
        ($tab:ident, $field:ident) => {{
            let expected = stringify!($field);
            app.$field.save_error = Some(expected.to_string());
            assert_eq!(active_tab_save_error(&app, SystemTab::$tab), Some(expected));
            app.$field.save_error = None;
        }};
    }

    assert_error_route!(Providers, provider_settings);
    app.model_settings.save_error = Some("model_settings".to_string());
    assert_eq!(active_tab_save_error(&app, SystemTab::Models), Some("model_settings"));
    app.model_settings.save_error = None;
    app.model_settings.active_tab = crate::app::state::ModelConfigTab::ModelRoutes;
    assert_error_route!(Models, model_routes_settings);
    app.model_settings.active_tab = crate::app::state::ModelConfigTab::QueryClassification;
    assert_error_route!(Models, query_classification_settings);
    app.model_settings.active_tab = crate::app::state::ModelConfigTab::Multimodal;
    assert_error_route!(Models, multimodal_settings);
    app.model_settings.active_tab = crate::app::state::ModelConfigTab::Models;
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Heartbeat;
    assert_error_route!(Agents, heartbeat_settings);
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Agents;
    assert_error_route!(Cron, cron_settings);
    app.scheduler_settings.save_error = Some("scheduler_settings".to_string());
    assert_eq!(active_tab_save_error(&app, SystemTab::Cron), Some("scheduler_settings"));
    app.scheduler_settings.save_error = None;
    assert_error_route!(Agents, agents_settings);
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Acp;
    assert_error_route!(Agents, acp_settings);
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Mcp;
    assert_error_route!(Agents, mcp_settings);
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Skills;
    assert_error_route!(Agents, skills_settings);
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Runtime;
    assert_error_route!(Agents, runtime_settings);
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Reliability;
    assert_error_route!(Agents, reliability_settings);
    assert_error_route!(Staff, staff_settings);
    assert_error_route!(Channels, channels_settings);
    assert_error_route!(Memory, memory_settings);
    assert_error_route!(Runtime, runtime_settings);
    assert_error_route!(Autonomy, autonomy_settings);
    assert_error_route!(Security, security_settings);
    assert_error_route!(GatewayClient, gateway_client_settings);
    assert_error_route!(Gateway, gateway_settings);
    assert_error_route!(Observability, observability_settings);
    app.memory_settings.active_tab = crate::app::state::MemorySettingsTab::Storage;
    assert_error_route!(Memory, storage_settings);
    assert_error_route!(Proxy, proxy_settings);
    assert_error_route!(Tunnel, tunnel_settings);
    assert_error_route!(Hooks, hooks_settings);
    assert_error_route!(WebSearch, web_search_settings);
    assert_error_route!(HttpRequest, http_request_settings);
    assert_error_route!(Browser, browser_settings);
    for tab in [
        SystemTab::General,
        SystemTab::Editor,
        SystemTab::Projects,
    ] {
        assert_eq!(active_tab_save_error(&app, tab), None);
    }
    assert_eq!(active_tab_save_error(&app, SystemTab::Providers), None);
}

#[test]
fn root_settings_skips_errors_owned_by_tabbed_pages() {
    for tab in [
        SystemTab::Models,
        SystemTab::Cron,
        SystemTab::Agents,
        SystemTab::Staff,
        SystemTab::Channels,
        SystemTab::Memory,
        SystemTab::Autonomy,
        SystemTab::Security,
        SystemTab::Gateway,
        SystemTab::WebSearch,
        SystemTab::HttpRequest,
        SystemTab::Browser,
        SystemTab::Proxy,
        SystemTab::Tunnel,
    ] {
        assert!(tabbed_page_owns_save_error(tab), "{tab:?} should own its save error");
    }

    let mut app = crate::app::App::new().0;
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Models;

    let local_without_error = Tree::new(super::system_settings_models::view(&app).as_widget());
    let root_without_error = Tree::new(view(&app).as_widget());

    app.model_settings.save_error = Some("model save failed".to_string());
    let local_with_error = Tree::new(super::system_settings_models::view(&app).as_widget());
    let root_with_error = Tree::new(view(&app).as_widget());

    let local_delta = tree_node_count(&local_with_error) - tree_node_count(&local_without_error);
    let root_delta = tree_node_count(&root_with_error) - tree_node_count(&root_without_error);
    assert!(local_delta > 0);
    assert_eq!(root_delta, local_delta);
}

#[test]
fn root_settings_keeps_errors_for_plain_pages() {
    for tab in [
        SystemTab::Providers,
        SystemTab::Runtime,
        SystemTab::GatewayClient,
        SystemTab::Observability,
        SystemTab::Hooks,
    ] {
        assert!(!tabbed_page_owns_save_error(tab), "{tab:?} should use the root banner");
    }

    let mut app = crate::app::App::new().0;
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Providers;
    let without_error = Tree::new(view(&app).as_widget());

    app.provider_settings.save_error = Some("provider save failed".to_string());
    let with_error = Tree::new(view(&app).as_widget());

    assert!(tree_node_count(&with_error) > tree_node_count(&without_error));
}

#[test]
fn view_builds_hidden_empty_search_and_every_visible_tab() {
    let mut app = crate::app::App::new().0;

    app.show_system_settings = false;
    drop(view(&app));

    app.show_system_settings = true;
    app.system_settings_query = "definitely-not-a-tab".to_string();
    drop(view(&app));

    app.system_settings_query.clear();
    for tab in SystemTab::all() {
        app.system_settings_tab = tab;
        drop(view(&app));
    }
    for tab in [SystemTab::HttpRequest, SystemTab::Browser] {
        app.system_settings_tab = tab;
        drop(view(&app));
    }
}

#[test]
fn view_builds_active_tab_help_and_save_error_state() {
    let mut app = crate::app::App::new().0;
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Agents;
    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Heartbeat;
    app.heartbeat_settings.show_help_modal = true;
    app.heartbeat_settings.save_error = Some("heartbeat save failed".to_string());

    drop(view(&app));
}

#[test]
fn view_builds_agent_config_overlays_for_each_nested_owner() {
    let mut app = crate::app::App::new().0;
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Agents;

    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Mcp;
    app.mcp_settings.delete_pending = Some(0);
    drop(view(&app));

    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Skills;
    app.skills_settings.show_help_modal = true;
    drop(view(&app));

    app.agents_settings.active_config_tab = crate::app::state::AgentConfigTab::Reliability;
    app.reliability_settings.show_help_modal = true;
    drop(view(&app));
}

#[test]
fn view_builds_when_search_filters_out_active_tab_but_content_has_error() {
    let mut app = crate::app::App::new().0;
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Cron;
    app.system_settings_query = "speech".to_string();
    app.cron_settings.save_error = Some("cron failed".to_string());

    drop(view(&app));
}
