// 验证代理开关、作用域别名及地址列表字段更新。
// 测试夹具围绕目标分支构造可控状态；跨模块行为由对应集成测试负责。

use super::*;
type App = ProxySettingsState;
use config_proxy::ProxyScope;

fn app() -> App {
    App::default()
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn update_ignores_unrelated_messages() {
    let mut app = app();
    app.enabled = true;
    app.http_proxy = "http://proxy.example".to_string();
    app.save_error = Some("old error".to_string());
    app.show_help_modal = true;

    let _ = update(&mut app, SettingsMessage::None);

    assert!(app.enabled);
    assert_eq!(app.http_proxy, "http://proxy.example");
    assert_eq!(app.save_error.as_deref(), Some("old error"));
    assert!(app.show_help_modal);
}

#[test]
fn enabled_toggle_updates_value_and_clears_stale_error() {
    let mut app = app();
    app.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::ProxyEnabledToggled(true));
    assert!(app.enabled);
    assert!(app.save_error.is_none());

    app.save_error = Some("another error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyEnabledToggled(false));
    assert!(!app.enabled);
    assert!(app.save_error.is_none());
}

#[test]
fn scope_change_accepts_environment_aliases_case_and_whitespace() {
    let mut app = app();

    for input in ["environment", " ENV ", "系统环境"] {
        app.save_error = Some("old error".to_string());
        let _ = update(&mut app, SettingsMessage::ProxyScopeTextChanged(input.to_string()));

        assert_eq!(app.scope, ProxyScope::Environment, "input: {input}");
        assert!(app.save_error.is_none(), "input: {input}");
    }
}

// 状态转换：覆盖交互前后的字段同步、选择关系与清理规则。
// 连续操作仍需保持模块不变量，并清除已经失效的瞬态反馈。
#[test]
fn scope_change_accepts_services_aliases_case_and_whitespace() {
    let mut app = app();

    for input in ["services", " SERVICE ", "指定服务"] {
        app.save_error = Some("old error".to_string());
        let _ = update(&mut app, SettingsMessage::ProxyScopeTextChanged(input.to_string()));

        assert_eq!(app.scope, ProxyScope::Services, "input: {input}");
        assert!(app.save_error.is_none(), "input: {input}");
    }
}

#[test]
fn scope_change_falls_back_to_vibewindow_for_other_inputs() {
    let mut app = app();

    for input in ["ShiJu", "vibewindow", "unknown", "  "] {
        app.scope = ProxyScope::Environment;
        app.save_error = Some("old error".to_string());
        let _ = update(&mut app, SettingsMessage::ProxyScopeTextChanged(input.to_string()));

        assert_eq!(app.scope, ProxyScope::Vibewindow, "input: {input}");
        assert!(app.save_error.is_none(), "input: {input}");
    }
}

#[test]
fn proxy_address_changes_preserve_input_and_clear_stale_error() {
    let mut app = app();

    app.save_error = Some("http error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyHttpChanged(" http://proxy ".to_string()));
    assert_eq!(app.http_proxy, " http://proxy ");
    assert!(app.save_error.is_none());

    app.save_error = Some("https error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyHttpsChanged(" https://proxy ".to_string()));
    assert_eq!(app.https_proxy, " https://proxy ");
    assert!(app.save_error.is_none());

    app.save_error = Some("all error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyAllChanged(" socks5://proxy ".to_string()));
    assert_eq!(app.all_proxy, " socks5://proxy ");
    assert!(app.save_error.is_none());
}

// 失败与边界：覆盖非法、缺失、过期或冲突场景的既定处理规则。
// 重点确认现有状态与错误反馈不会被意外的部分更新污染。
#[test]
fn proxy_list_changes_preserve_mixed_separators_and_clear_stale_error() {
    let mut app = app();
    let no_proxy = " localhost, 127.0.0.1\n*.internal ";
    let services = " provider.openai,tool.http_request\n browser ";

    app.save_error = Some("no proxy error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyNoProxyChanged(no_proxy.to_string()));
    assert_eq!(app.no_proxy_input, no_proxy);
    assert!(app.save_error.is_none());

    app.save_error = Some("services error".to_string());
    let _ = update(&mut app, SettingsMessage::ProxyServicesChanged(services.to_string()));
    assert_eq!(app.services_input, services);
    assert!(app.save_error.is_none());
}

#[test]
fn help_messages_toggle_modal_without_clearing_save_error() {
    let mut app = app();
    app.save_error = Some("old error".to_string());

    let _ = update(&mut app, SettingsMessage::ProxyHelpOpen);
    assert!(app.show_help_modal);
    assert_eq!(app.save_error.as_deref(), Some("old error"));

    let _ = update(&mut app, SettingsMessage::ProxyHelpClose);
    assert!(!app.show_help_modal);
    assert_eq!(app.save_error.as_deref(), Some("old error"));
}

// 任务与结果：覆盖任务构造、调度或结果回写对公开状态的影响。
// 完成、失败与过期响应均按当前场景的既定契约处理。
#[test]
fn proxy_changes_schedule_persistence_tasks() {
    let mut app = app();

    for message in [
        SettingsMessage::ProxyEnabledToggled(true),
        SettingsMessage::ProxyHttpChanged(String::new()),
        SettingsMessage::ProxyNoProxyChanged("localhost,\n127.0.0.1".to_string()),
    ] {
        let task = update(&mut app, message);
        assert!(matches!(task, Effect::Persist(_)));
    }
}
