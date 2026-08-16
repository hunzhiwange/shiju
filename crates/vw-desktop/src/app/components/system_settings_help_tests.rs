use super::*;
use crate::app::App;
use crate::app::components::system_settings::SystemTab;

const SUPPORTED_HELP: &[(SystemTab, &str, &str)] = &[
    (SystemTab::General, "常规设置帮助", GENERAL_HELP),
    (SystemTab::Editor, "编辑器设置帮助", EDITOR_HELP),
    (SystemTab::Projects, "项目设置帮助", PROJECTS_HELP),
    (SystemTab::Providers, "模型提供商帮助", PROVIDERS_HELP),
    (SystemTab::Models, "模型设置帮助", MODELS_HELP),
    (SystemTab::Agents, "智能体配置帮助", AGENTS_HELP),
    (SystemTab::Channels, "通道配置帮助", CHANNELS_HELP),
    (SystemTab::Memory, "记忆系统帮助", MEMORY_HELP),
    (SystemTab::Runtime, "运行时配置帮助", RUNTIME_HELP),
    (SystemTab::Tunnel, "隧道配置帮助", TUNNEL_HELP),
    (SystemTab::Hooks, "钩子配置帮助", HOOKS_HELP),
    (SystemTab::WebSearch, "网页搜索帮助", WEB_SEARCH_HELP),
    (SystemTab::HttpRequest, "网络请求帮助", HTTP_REQUEST_HELP),
    (SystemTab::Browser, "浏览器配置帮助", BROWSER_HELP),
];

const UNSUPPORTED_TABS: &[SystemTab] = &[
    SystemTab::Cron,
    SystemTab::Staff,
    SystemTab::Autonomy,
    SystemTab::Security,
    SystemTab::GatewayClient,
    SystemTab::Gateway,
    SystemTab::Observability,
    SystemTab::Proxy,
];

fn test_app() -> App {
    App::new().0
}

#[test]
fn system_settings_help_tests_are_wired() {
    assert!(module_path!().contains("system_settings_help_tests"));
}

#[test]
fn help_content_maps_every_supported_tab_to_its_title_and_copy() {
    for &(tab, title, copy) in SUPPORTED_HELP {
        assert_eq!(help_content(tab), Some((title, copy)));
    }
}

#[test]
fn hooks_help_describes_live_behavior_and_safe_fields() {
    assert!(HOOKS_HELP.contains("插件 hook"));
    assert!(HOOKS_HELP.contains("信任"));
    assert!(HOOKS_HELP.contains("撤销"));
    assert!(HOOKS_HELP.contains("时间、工具名、耗时和成功状态"));
    assert!(HOOKS_HELP.contains("不记录工具参数、结果正文、令牌或密钥"));
    assert!(!HOOKS_HELP.contains("预留"));
}

#[test]
fn help_content_rejects_every_tab_without_help_copy() {
    for &tab in UNSUPPORTED_TABS {
        assert_eq!(help_content(tab), None);
    }
}

#[test]
fn help_open_for_tab_requires_matching_supported_tab() {
    for &(tab, _, _) in SUPPORTED_HELP {
        assert!(help_open_for_tab(Some(tab), tab));
        assert!(!help_open_for_tab(None, tab));
    }

    assert!(!help_open_for_tab(Some(SystemTab::General), SystemTab::Memory));

    for &tab in UNSUPPORTED_TABS {
        assert!(!help_open_for_tab(Some(tab), tab));
    }
}

#[test]
fn help_button_bar_exists_only_for_supported_tabs() {
    for &(tab, _, _) in SUPPORTED_HELP {
        assert!(help_button_bar(tab).is_some());
    }

    for &tab in UNSUPPORTED_TABS {
        assert!(help_button_bar(tab).is_none());
    }
}

#[test]
fn with_help_modal_returns_base_for_closed_mismatched_and_unsupported_tabs() {
    let app = test_app();
    let base = iced::widget::container(iced::widget::text("base")).into();
    let _ = with_help_modal(&app, base, SystemTab::General, None);

    let base = iced::widget::container(iced::widget::text("base")).into();
    let _ = with_help_modal(&app, base, SystemTab::General, Some(SystemTab::Memory));

    let base = iced::widget::container(iced::widget::text("base")).into();
    let _ = with_help_modal(&app, base, SystemTab::Security, Some(SystemTab::Security));
}

#[test]
fn with_help_modal_wraps_supported_open_tab() {
    let app = test_app();
    let base = iced::widget::container(iced::widget::text("base")).into();
    let _ = with_help_modal(&app, base, SystemTab::Memory, Some(SystemTab::Memory));
}
