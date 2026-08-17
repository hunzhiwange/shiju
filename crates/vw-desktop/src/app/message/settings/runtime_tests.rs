use super::*;
use crate::app::App;

fn app() -> App {
    App::new().0
}

fn apply(app: &mut App, message: RuntimeMessage) {
    let _ = update(app, SettingsMessage::Runtime(message));
}

fn assert_persist_error(configure: impl FnOnce(&mut App), expected: &str) {
    let mut app = app();
    configure(&mut app);
    assert_eq!(persist_runtime_settings(&mut app).err().as_deref(), Some(expected));
}

#[test]
fn runtime_choice_normalizers_accept_supported_values_and_reject_unknown_values() {
    assert_eq!(normalize_runtime_kind(" NATIVE "), "native");
    assert_eq!(normalize_runtime_kind("Docker"), "docker");
    assert_eq!(normalize_runtime_kind("remote"), "native");

    assert_eq!(normalize_reasoning_enabled(" TRUE "), "true");
    assert_eq!(normalize_reasoning_enabled("False"), "false");
    assert_eq!(normalize_reasoning_enabled("enabled"), "auto");
}

#[test]
fn reasoning_level_parser_normalizes_aliases_and_rejects_unknown_levels() {
    assert_eq!(parse_reasoning_level("  ").unwrap(), None);
    assert_eq!(parse_reasoning_level("minimal").unwrap(), Some("minimal".to_string()));
    assert_eq!(parse_reasoning_level("X-HIGH").unwrap(), Some("xhigh".to_string()));
    assert_eq!(parse_reasoning_level("x_high").unwrap(), Some("xhigh".to_string()));
    assert_eq!(
        parse_reasoning_level("extreme").unwrap_err(),
        "reasoning_level 仅支持 minimal / low / medium / high / xhigh"
    );
}

#[test]
fn numeric_parsers_handle_empty_bounds_and_invalid_values() {
    assert_eq!(parse_optional_u64("  ", "field").unwrap(), None);
    assert_eq!(parse_optional_u64(" 42 ", "field").unwrap(), Some(42));
    assert_eq!(parse_optional_u64("-1", "field").unwrap_err(), "field 必须是非负整数");

    assert_eq!(parse_optional_f64("", "field").unwrap(), None);
    assert_eq!(parse_optional_f64(" 1.5 ", "field").unwrap(), Some(1.5));
    for input in ["0", "-0.5", "NaN", "inf"] {
        assert_eq!(parse_optional_f64(input, "field").unwrap_err(), "field 必须大于 0");
    }
    assert_eq!(parse_optional_f64("many", "field").unwrap_err(), "field 必须是数字");
}

#[test]
fn list_parser_trims_entries() {
    assert_eq!(parse_csv_lines(" /a, , /b\n/a\n  "), vec!["/a", "/b", "/a"]);
}

#[test]
fn runtime_update_ignores_unrelated_messages_and_refresh_clears_only_the_error() {
    let mut app = app();
    app.runtime_settings.kind = "docker".to_string();
    app.runtime_settings.save_error = Some("old".to_string());

    let _ = update(&mut app, SettingsMessage::ToggleSettingsSidebar);
    assert_eq!(app.runtime_settings.kind, "docker");
    assert_eq!(app.runtime_settings.save_error.as_deref(), Some("old"));

    apply(&mut app, RuntimeMessage::Refresh);
    assert_eq!(app.runtime_settings.kind, "docker");
    assert!(app.runtime_settings.save_error.is_none());
}

#[test]
fn runtime_update_applies_docker_fields_and_normalizes_kind() {
    let mut app = app();

    apply(&mut app, RuntimeMessage::KindChanged("invalid".to_string()));
    apply(&mut app, RuntimeMessage::DockerImageChanged(" image:latest ".to_string()));
    apply(&mut app, RuntimeMessage::DockerNetworkChanged("host".to_string()));
    apply(&mut app, RuntimeMessage::DockerMemoryLimitMbChanged("256".to_string()));
    apply(&mut app, RuntimeMessage::DockerCpuLimitChanged("2.5".to_string()));
    apply(&mut app, RuntimeMessage::DockerReadOnlyRootfsToggled(false));
    apply(&mut app, RuntimeMessage::DockerMountWorkspaceToggled(false));
    apply(&mut app, RuntimeMessage::DockerAllowedWorkspaceRootsChanged(" /a, /b ".to_string()));

    assert_eq!(app.runtime_settings.kind, "native");
    assert_eq!(app.runtime_settings.docker_image, " image:latest ");
    assert_eq!(app.runtime_settings.docker_network, "host");
    assert_eq!(app.runtime_settings.docker_memory_limit_mb_input, "256");
    assert_eq!(app.runtime_settings.docker_cpu_limit_input, "2.5");
    assert!(!app.runtime_settings.docker_read_only_rootfs);
    assert!(!app.runtime_settings.docker_mount_workspace);
    assert_eq!(app.runtime_settings.docker_allowed_workspace_roots_input, " /a, /b ");
    assert!(app.runtime_settings.save_error.is_none());
}

#[test]
fn runtime_update_normalizes_reasoning_fields_and_surfaces_validation_errors() {
    let mut app = app();

    apply(&mut app, RuntimeMessage::ReasoningEnabledChanged("TRUE".to_string()));
    apply(&mut app, RuntimeMessage::ReasoningLevelChanged("x_high".to_string()));
    assert_eq!(app.runtime_settings.reasoning_enabled_input, "true");
    assert_eq!(app.runtime_settings.reasoning_level_input, "x_high");
    assert!(app.runtime_settings.save_error.is_none());

    apply(&mut app, RuntimeMessage::ReasoningLevelChanged("extreme".to_string()));
    assert_eq!(
        app.runtime_settings.save_error.as_deref(),
        Some("reasoning_level 仅支持 minimal / low / medium / high / xhigh")
    );
}

#[test]
fn runtime_persistence_validation_rejects_each_invalid_docker_input() {
    assert_persist_error(
        |app| app.runtime_settings.docker_memory_limit_mb_input = "-1".to_string(),
        "Docker 内存限制 必须是非负整数",
    );
    assert_persist_error(
        |app| app.runtime_settings.docker_cpu_limit_input = "NaN".to_string(),
        "Docker CPU 限制 必须大于 0",
    );
}

#[test]
fn valid_runtime_settings_schedule_persistence_task() {
    let mut app = app();
    app.runtime_settings.docker_memory_limit_mb_input = String::new();
    app.runtime_settings.docker_cpu_limit_input = "1.25".to_string();
    app.runtime_settings.reasoning_enabled_input = "auto".to_string();
    app.runtime_settings.reasoning_level_input = "high".to_string();

    let task = persist_runtime_settings(&mut app).expect("valid settings should persist");

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}
