use super::*;
use crate::app::state::{SecurityListKind, SecuritySettingsTab};
use crate::app::{App, Message};
use iced::Element;
use iced::widget::{slider, text};

const SOURCE: &str = include_str!("system_settings_security.rs");

fn test_app() -> App {
    App::new().0
}

fn keep_element(element: Element<'_, Message>) {
    std::hint::black_box(element);
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
fn labeled_option_displays_label_not_value() {
    let option = LabeledOption { value: "landlock", label: "Landlock" };

    assert_eq!(option.to_string(), "Landlock");
}

#[test]
fn sandbox_enabled_options_are_stable_and_user_facing() {
    let options = sandbox_enabled_options();
    let values: Vec<_> = options.iter().map(|option| option.value).collect();
    let labels: Vec<_> = options.iter().map(|option| option.label).collect();

    assert_eq!(values, ["auto", "true", "false"]);
    assert_eq!(labels, ["自动检测", "强制启用", "强制禁用"]);
}

#[test]
fn sandbox_backend_options_follow_enabled_policy() {
    let disabled_options = sandbox_backend_options("false");
    let disabled_values: Vec<_> = disabled_options.iter().map(|option| option.value).collect();
    let disabled_labels: Vec<_> = disabled_options.iter().map(|option| option.label).collect();
    let forced_values: Vec<_> =
        sandbox_backend_options("true").iter().map(|option| option.value).collect();
    let auto_values: Vec<_> =
        sandbox_backend_options("auto").iter().map(|option| option.value).collect();

    assert_eq!(disabled_values, ["none"]);
    assert_eq!(disabled_labels, ["禁止沙箱"]);
    assert_eq!(forced_values, ["auto", "landlock", "firejail", "bubblewrap", "docker"]);
    assert_eq!(auto_values, ["auto", "landlock", "firejail", "bubblewrap", "docker", "none"]);
}

#[test]
fn sandbox_backend_options_treat_unknown_enabled_values_as_auto_policy() {
    let expected = ["auto", "landlock", "firejail", "bubblewrap", "docker", "none"];

    for enabled in ["", "TRUE", " false", "unsupported"] {
        let values: Vec<_> =
            sandbox_backend_options(enabled).iter().map(|option| option.value).collect();
        assert_eq!(values, expected, "enabled={enabled:?}");
    }
}

#[test]
fn sandbox_backend_description_covers_known_and_fallback_backends() {
    assert!(sandbox_backend_description("landlock").contains("Landlock"));
    assert!(sandbox_backend_description("firejail").contains("Firejail"));
    assert!(sandbox_backend_description("bubblewrap").contains("Bubblewrap"));
    assert!(sandbox_backend_description("docker").contains("Docker"));
    assert!(sandbox_backend_description("none").contains("不使用任何沙箱"));
    assert!(sandbox_backend_description("unknown").contains("自动选择"));
}

#[test]
fn common_rows_accept_security_controls() {
    let field: Element<'_, Message> = field_row("沙箱启用", "选择策略。", text("auto"));
    let input: Element<'_, Message> = text_row(
        "审计日志路径",
        "指定审计日志文件。",
        "audit.log",
        "audit.log",
        |_| Message::GatewayHealthTick,
    );
    let checkbox: Element<'_, Message> =
        bool_row("审计日志", "记录安全事件。", true, "开启", |_| {
            Message::GatewayHealthTick
        });
    let numeric: Element<'_, Message> = slider_row(
        "最大内存",
        "限制执行进程可使用的最大内存。",
        slider(32.0..=65_536.0, 512.0, |_| Message::GatewayHealthTick),
        "512 MB",
    );

    drop(field);
    drop(input);
    drop(checkbox);
    drop(numeric);
}

#[test]
fn view_builds_default_disabled_firejail_and_threshold_states() {
    let mut app = test_app();
    keep_element(view(&app));

    app.security_settings.sandbox_enabled_input = "false".to_string();
    app.security_settings.sandbox_backend_input = "none".to_string();
    app.security_settings.resources_memory_monitoring = false;
    app.security_settings.audit_enabled = false;
    app.security_settings.otp_enabled = true;
    app.security_settings.estop_enabled = true;
    app.security_settings.syscall_anomaly_enabled = false;
    app.security_settings.syscall_anomaly_strict_mode = true;
    app.security_settings.canary_tokens = false;
    app.security_settings.semantic_guard = true;
    app.security_settings.semantic_guard_threshold = 0.0;
    keep_element(view(&app));

    app.security_settings.sandbox_enabled_input = "true".to_string();
    app.security_settings.sandbox_backend_input = "firejail".to_string();
    app.security_settings.sandbox_firejail_args = vec![
        "--private".to_string(),
        "--net=none".to_string(),
        "--read-only=/workspace".to_string(),
    ];
    app.security_settings.resources_max_memory_mb = 65_536;
    app.security_settings.resources_max_cpu_time_seconds = 3600;
    app.security_settings.resources_max_subprocesses = 10_000;
    app.security_settings.audit_log_path = "logs/security-audit.log".to_string();
    app.security_settings.audit_max_size_mb = 10_000;
    app.security_settings.audit_sign_events = true;
    app.security_settings.otp_method_input = "cli-prompt".to_string();
    app.security_settings.otp_token_ttl_secs = 600;
    app.security_settings.otp_cache_valid_secs = 3600;
    app.security_settings.otp_gated_actions = vec!["shell".to_string(), "file_write".to_string()];
    app.security_settings.otp_gated_domains =
        vec!["example.com".to_string(), "api.internal".to_string()];
    app.security_settings.otp_gated_domain_categories =
        vec!["payments".to_string(), "admin".to_string()];
    app.security_settings.estop_state_file = "/tmp/vibewindow-estop.json".to_string();
    app.security_settings.estop_require_otp_to_resume = false;
    app.security_settings.syscall_anomaly_alert_on_unknown_syscall = false;
    app.security_settings.syscall_anomaly_max_denied_events_per_minute = 10_000;
    app.security_settings.syscall_anomaly_max_total_events_per_minute = 100_000;
    app.security_settings.syscall_anomaly_max_alerts_per_minute = 10_000;
    app.security_settings.syscall_anomaly_alert_cooldown_secs = 3600;
    app.security_settings.syscall_anomaly_log_path = "logs/syscall-anomalies.log".to_string();
    app.security_settings.syscall_anomaly_baseline_syscalls =
        vec!["read".to_string(), "write".to_string(), "openat".to_string()];
    app.security_settings.semantic_guard_collection = "security_vectors".to_string();
    app.security_settings.semantic_guard_threshold = 1.0;
    keep_element(view(&app));
}

#[test]
fn view_builds_each_security_tab_and_unicode_list_items() {
    let mut app = test_app();
    app.security_settings.otp_gated_domains = vec!["内部.example".to_string()];

    for tab in [
        SecuritySettingsTab::Bash,
        SecuritySettingsTab::SandboxAndResources,
        SecuritySettingsTab::Audit,
        SecuritySettingsTab::OtpAndEstop,
        SecuritySettingsTab::Syscall,
        SecuritySettingsTab::Guard,
    ] {
        app.security_settings.active_tab = tab;
        keep_element(view(&app));
    }

    assert!(!SOURCE.contains("Font::MONOSPACE"));
}

#[test]
fn list_editor_builds_bounded_scrolling_saved_items() {
    let items = vec!["read".to_string(), "write".to_string(), "中文调用".to_string()];

    keep_element(list_editor(
        SecurityListKind::SyscallAnomalyBaselineSyscalls,
        "系统调用基线",
        "说明",
        "例如 openat",
        &items,
        "",
    ));

    assert!(SOURCE.contains("LIST_EDITOR_MAX_HEIGHT"));
    assert!(SOURCE.contains("Scrollbar::new().width(4).scroller_width(4)"));
    assert!(SOURCE.contains("right: 12.0"));
}

#[test]
fn view_builds_when_sandbox_selection_values_are_unknown() {
    let mut app = test_app();
    app.security_settings.sandbox_enabled_input = "unsupported".to_string();
    app.security_settings.sandbox_backend_input = "unsupported".to_string();

    keep_element(view(&app));
}

#[test]
fn overlays_return_base_dialog_or_help_modal() {
    let mut app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    app.security_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog").into()));
}

#[test]
fn system_settings_security_tests_keep_planned_coverage_targets() {
    for name in [
        "field_row",
        "text_row",
        "list_editor",
        "tab_button",
        "bool_row",
        "slider_row",
        "sandbox_enabled_options",
        "sandbox_backend_options",
        "sandbox_backend_description",
        "view",
        "view_overlays",
    ] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}
