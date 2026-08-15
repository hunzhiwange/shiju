// Tests for plan6 task 827.
const SOURCE: &str = include_str!("system_settings.rs");

use super::system_settings::{
    SystemTab, active_tab_help_modal_open, active_tab_save_error, system_tab_matches_query, view,
};
use super::system_settings_help::help_button_bar;

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
    assert!(system_tab_matches_query(SystemTab::QuickMessages, "创建新会话"));
    assert!(system_tab_matches_query(SystemTab::Agents, "API 密钥"));
    assert!(system_tab_matches_query(SystemTab::Scheduler, "最大并发"));
    assert!(system_tab_matches_query(SystemTab::Acp, "初始化"));
    assert!(system_tab_matches_query(SystemTab::Mcp, "stdio"));
    assert!(system_tab_matches_query(SystemTab::WebSearch, "Brave 密钥"));
    assert!(system_tab_matches_query(SystemTab::HttpRequest, "允许域名"));
}

#[test]
fn system_tab_all_contains_every_display_label_once_in_order() {
    let tabs = SystemTab::all();

    assert_eq!(tabs.len(), 43);
    assert_eq!(tabs.first(), Some(&SystemTab::General));
    assert_eq!(tabs.get(1), Some(&SystemTab::Themes));
    assert_eq!(tabs.last(), Some(&SystemTab::Transcription));
    assert_eq!(tabs.iter().filter(|tab| **tab == SystemTab::HttpRequest).count(), 1);

    let labels = tabs.iter().map(ToString::to_string).collect::<Vec<_>>();
    assert!(labels.contains(&"常规设置".to_string()));
    assert!(labels.contains(&"主题".to_string()));
    assert!(labels.contains(&"快捷消息".to_string()));
    assert!(labels.contains(&"ACP 配置".to_string()));
    assert!(labels.contains(&"MCP 配置".to_string()));
    assert!(labels.contains(&"委托代理配置".to_string()));
    assert!(labels.contains(&"转录配置".to_string()));
}

#[test]
fn system_tab_search_is_trimmed_case_insensitive_and_empty_matches_all() {
    assert!(system_tab_matches_query(SystemTab::Providers, "  API KEY "));
    assert!(system_tab_matches_query(SystemTab::Acp, "CODEX"));
    assert!(system_tab_matches_query(SystemTab::Mcp, "MCP"));
    assert!(system_tab_matches_query(SystemTab::General, "   "));
    assert!(!system_tab_matches_query(SystemTab::General, "definitely-not-a-tab"));
}

#[test]
fn system_tab_display_labels_cover_every_tab_in_navigation_order() {
    let expected = [
        "常规设置",
        "主题",
        "对话流",
        "快捷消息",
        "编辑器",
        "项目",
        "提供商",
        "模型",
        "嵌入路由",
        "记忆配置",
        "模型路由",
        "查询分类",
        "目标循环配置",
        "心跳配置",
        "定时任务配置",
        "标准流程配置",
        "调度配置",
        "委托代理配置",
        "员工配置",
        "代理通信配置",
        "协调配置",
        "可靠性配置",
        "通道配置",
        "运行时配置",
        "ACP 配置",
        "MCP 配置",
        "自治配置",
        "安全配置",
        "客户端网关",
        "服务端网关",
        "可观测性配置",
        "存储配置",
        "代理配置",
        "隧道配置",
        "Composio",
        "技能配置",
        "钩子配置",
        "研究配置",
        "网页搜索配置",
        "网络请求配置",
        "浏览器配置",
        "多模态配置",
        "转录配置",
    ];

    for (tab, expected_label) in SystemTab::all().into_iter().zip(expected) {
        assert_eq!(tab.to_string(), expected_label);
    }
}

#[test]
fn system_tab_search_metadata_covers_every_tab() {
    let cases = [
        (SystemTab::General, "auto save"),
        (SystemTab::Themes, "gallery"),
        (SystemTab::DialogueFlow, "reasoning summary"),
        (SystemTab::QuickMessages, "quick messages"),
        (SystemTab::Editor, "code"),
        (SystemTab::Projects, "workspace"),
        (SystemTab::Providers, "catalog"),
        (SystemTab::Models, "llm"),
        (SystemTab::EmbeddingRoutes, "vector"),
        (SystemTab::Memory, "sqlite"),
        (SystemTab::ModelRoutes, "routing"),
        (SystemTab::QueryClassification, "category"),
        (SystemTab::GoalLoop, "automation"),
        (SystemTab::Heartbeat, "monitor"),
        (SystemTab::Cron, "history retention"),
        (SystemTab::Scheduler, "max_concurrent"),
        (SystemTab::Agents, "subagent"),
        (SystemTab::Staff, "roster"),
        (SystemTab::AgentsIpc, "communication"),
        (SystemTab::Coordination, "agents"),
        (SystemTab::Reliability, "retry"),
        (SystemTab::Channels, "slack"),
        (SystemTab::Runtime, "docker"),
        (SystemTab::Acp, "protocol"),
        (SystemTab::Mcp, "allowedtools"),
        (SystemTab::Autonomy, "approval"),
        (SystemTab::Security, "canary"),
        (SystemTab::GatewayClient, "connection"),
        (SystemTab::Gateway, "webhook"),
        (SystemTab::Observability, "otel"),
        (SystemTab::Storage, "database"),
        (SystemTab::Proxy, "all_proxy"),
        (SystemTab::Tunnel, "tailscale"),
        (SystemTab::Composio, "oauth"),
        (SystemTab::Skills, "plugins"),
        (SystemTab::Hooks, "command_logger"),
        (SystemTab::Research, "search"),
        (SystemTab::WebSearch, "duckduckgo"),
        (SystemTab::HttpRequest, "allowlist"),
        (SystemTab::Browser, "webdriver"),
        (SystemTab::Multimodal, "vision"),
        (SystemTab::Transcription, "speech"),
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
    assert!(active_tab_help_modal_open(&app, SystemTab::Heartbeat));
    app.cron_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Cron));
    app.scheduler_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Scheduler));
    app.agents_ipc_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::AgentsIpc));
    app.coordination_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Coordination));
    app.reliability_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Reliability));
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
    assert!(active_tab_help_modal_open(&app, SystemTab::Skills));
    app.research_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Research));
    app.web_search_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::WebSearch));
    app.transcription_settings.show_help_modal = true;
    assert!(active_tab_help_modal_open(&app, SystemTab::Transcription));
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
            if tab == SystemTab::General { SystemTab::Transcription } else { SystemTab::General };
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
    app.agents_ipc_settings.show_help_modal = true;
    app.coordination_settings.show_help_modal = true;
    app.reliability_settings.show_help_modal = true;
    app.autonomy_settings.show_help_modal = true;
    app.security_settings.show_help_modal = true;
    app.gateway_client_settings.show_help_modal = true;
    app.gateway_settings.show_help_modal = true;
    app.observability_settings.show_help_modal = true;
    app.proxy_settings.show_help_modal = true;
    app.skills_settings.show_help_modal = true;
    app.research_settings.show_help_modal = true;
    app.web_search_settings.show_help_modal = true;
    app.transcription_settings.show_help_modal = true;

    for tab in [
        SystemTab::General,
        SystemTab::DialogueFlow,
        SystemTab::QuickMessages,
        SystemTab::Editor,
        SystemTab::Projects,
        SystemTab::Providers,
        SystemTab::Storage,
        SystemTab::Tunnel,
        SystemTab::Composio,
        SystemTab::Hooks,
        SystemTab::HttpRequest,
        SystemTab::Browser,
        SystemTab::Multimodal,
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
    assert_error_route!(Models, model_settings);
    assert_error_route!(EmbeddingRoutes, embedding_routes_settings);
    assert_error_route!(ModelRoutes, model_routes_settings);
    assert_error_route!(QueryClassification, query_classification_settings);
    assert_error_route!(GoalLoop, goal_loop_settings);
    assert_error_route!(Heartbeat, heartbeat_settings);
    assert_error_route!(Cron, cron_settings);
    assert_error_route!(Scheduler, scheduler_settings);
    assert_error_route!(Agents, agents_settings);
    assert_error_route!(Staff, staff_settings);
    assert_error_route!(AgentsIpc, agents_ipc_settings);
    assert_error_route!(Coordination, coordination_settings);
    assert_error_route!(Reliability, reliability_settings);
    assert_error_route!(Channels, channels_settings);
    assert_error_route!(Memory, memory_settings);
    assert_error_route!(Runtime, runtime_settings);
    assert_error_route!(Acp, acp_settings);
    assert_error_route!(Mcp, mcp_settings);
    assert_error_route!(Autonomy, autonomy_settings);
    assert_error_route!(Security, security_settings);
    assert_error_route!(GatewayClient, gateway_client_settings);
    assert_error_route!(Gateway, gateway_settings);
    assert_error_route!(Observability, observability_settings);
    assert_error_route!(Storage, storage_settings);
    assert_error_route!(Proxy, proxy_settings);
    assert_error_route!(Tunnel, tunnel_settings);
    assert_error_route!(Composio, composio_settings);
    assert_error_route!(Skills, skills_settings);
    assert_error_route!(Hooks, hooks_settings);
    assert_error_route!(Research, research_settings);
    assert_error_route!(WebSearch, web_search_settings);
    assert_error_route!(HttpRequest, http_request_settings);
    assert_error_route!(Browser, browser_settings);
    assert_error_route!(Multimodal, multimodal_settings);
    assert_error_route!(Transcription, transcription_settings);

    for tab in [
        SystemTab::General,
        SystemTab::DialogueFlow,
        SystemTab::QuickMessages,
        SystemTab::Editor,
        SystemTab::Projects,
    ] {
        assert_eq!(active_tab_save_error(&app, tab), None);
    }
    assert_eq!(active_tab_save_error(&app, SystemTab::Providers), None);
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
}

#[test]
fn view_builds_active_tab_help_and_save_error_state() {
    let mut app = crate::app::App::new().0;
    app.show_system_settings = true;
    app.system_settings_tab = SystemTab::Heartbeat;
    app.heartbeat_settings.show_help_modal = true;
    app.heartbeat_settings.save_error = Some("heartbeat save failed".to_string());

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
