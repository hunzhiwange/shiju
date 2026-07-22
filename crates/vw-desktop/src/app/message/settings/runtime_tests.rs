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
    assert_eq!(normalize_runtime_kind(" WASM"), "wasm");
    assert_eq!(normalize_runtime_kind("remote"), "native");

    assert_eq!(normalize_reasoning_enabled(" TRUE "), "true");
    assert_eq!(normalize_reasoning_enabled("False"), "false");
    assert_eq!(normalize_reasoning_enabled("enabled"), "auto");

    assert_eq!(normalize_capability_mode(" CLAMP "), "clamp");
    assert_eq!(normalize_capability_mode("allow"), "deny");
    assert_eq!(normalize_module_hash_policy(" DISABLED "), "disabled");
    assert_eq!(normalize_module_hash_policy("Enforce"), "enforce");
    assert_eq!(normalize_module_hash_policy("ignore"), "warn");
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

    assert_eq!(parse_required_u64("0", "field", 1, 9).unwrap(), 1);
    assert_eq!(parse_required_u64("5", "field", 1, 9).unwrap(), 5);
    assert_eq!(parse_required_u64("10", "field", 1, 9).unwrap(), 9);
    assert_eq!(parse_required_u64("none", "field", 1, 9).unwrap_err(), "field 必须是整数");
}

#[test]
fn list_and_module_hash_parsers_trim_entries_and_report_malformed_lines() {
    assert_eq!(parse_csv_lines(" /a, , /b\n/a\n  "), vec!["/a", "/b", "/a"]);

    let hashes = parse_module_sha256_map(" tool-a : aaa,\n tool-b:bbb ").unwrap();
    assert_eq!(hashes.get("tool-a").map(String::as_str), Some("aaa"));
    assert_eq!(hashes.get("tool-b").map(String::as_str), Some("bbb"));
    assert!(parse_module_sha256_map(" \n, ").unwrap().is_empty());
    assert_eq!(
        parse_module_sha256_map("tool-a:aaa\nmalformed").unwrap_err(),
        "第 2 行模块摘要必须使用 module:sha256 格式"
    );
    assert_eq!(parse_module_sha256_map("tool-a:").unwrap_err(), "第 1 行模块摘要不能为空");
    assert_eq!(parse_module_sha256_map(":aaa").unwrap_err(), "第 1 行模块摘要不能为空");
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
fn runtime_update_applies_wasm_security_and_resource_fields() {
    let mut app = app();

    apply(&mut app, RuntimeMessage::WasmToolsDirChanged(" custom/tools ".to_string()));
    apply(&mut app, RuntimeMessage::WasmFuelLimitChanged("2000000".to_string()));
    apply(&mut app, RuntimeMessage::WasmMemoryLimitMbChanged("128".to_string()));
    apply(&mut app, RuntimeMessage::WasmMaxModuleSizeMbChanged("32".to_string()));
    apply(&mut app, RuntimeMessage::WasmAllowWorkspaceReadToggled(true));
    apply(&mut app, RuntimeMessage::WasmAllowWorkspaceWriteToggled(true));
    apply(&mut app, RuntimeMessage::WasmAllowedHostsChanged("a.test, b.test".to_string()));
    apply(&mut app, RuntimeMessage::WasmRequireWorkspaceRelativeToolsDirToggled(false));
    apply(&mut app, RuntimeMessage::WasmRejectSymlinkModulesToggled(false));
    apply(&mut app, RuntimeMessage::WasmRejectSymlinkToolsDirToggled(false));
    apply(&mut app, RuntimeMessage::WasmStrictHostValidationToggled(false));
    apply(&mut app, RuntimeMessage::WasmCapabilityEscalationModeChanged("CLAMP".to_string()));
    apply(&mut app, RuntimeMessage::WasmModuleHashPolicyChanged("ENFORCE".to_string()));
    apply(&mut app, RuntimeMessage::WasmModuleSha256Changed("tool:abc".to_string()));

    assert_eq!(app.runtime_settings.wasm_tools_dir, " custom/tools ");
    assert_eq!(app.runtime_settings.wasm_fuel_limit_input, "2000000");
    assert_eq!(app.runtime_settings.wasm_memory_limit_mb_input, "128");
    assert_eq!(app.runtime_settings.wasm_max_module_size_mb_input, "32");
    assert!(app.runtime_settings.wasm_allow_workspace_read);
    assert!(app.runtime_settings.wasm_allow_workspace_write);
    assert_eq!(app.runtime_settings.wasm_allowed_hosts_input, "a.test, b.test");
    assert!(!app.runtime_settings.wasm_require_workspace_relative_tools_dir);
    assert!(!app.runtime_settings.wasm_reject_symlink_modules);
    assert!(!app.runtime_settings.wasm_reject_symlink_tools_dir);
    assert!(!app.runtime_settings.wasm_strict_host_validation);
    assert_eq!(app.runtime_settings.wasm_capability_escalation_mode, "clamp");
    assert_eq!(app.runtime_settings.wasm_module_hash_policy, "enforce");
    assert_eq!(app.runtime_settings.wasm_module_sha256_input, "tool:abc");
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
fn runtime_persistence_validation_rejects_each_invalid_numeric_or_hash_input() {
    assert_persist_error(
        |app| app.runtime_settings.docker_memory_limit_mb_input = "-1".to_string(),
        "Docker 内存限制 必须是非负整数",
    );
    assert_persist_error(
        |app| app.runtime_settings.docker_cpu_limit_input = "NaN".to_string(),
        "Docker CPU 限制 必须大于 0",
    );
    assert_persist_error(
        |app| app.runtime_settings.wasm_fuel_limit_input = "invalid".to_string(),
        "WASM fuel_limit 必须是整数",
    );
    assert_persist_error(
        |app| app.runtime_settings.wasm_memory_limit_mb_input = "invalid".to_string(),
        "WASM memory_limit_mb 必须是整数",
    );
    assert_persist_error(
        |app| app.runtime_settings.wasm_max_module_size_mb_input = "invalid".to_string(),
        "WASM max_module_size_mb 必须是整数",
    );
    assert_persist_error(
        |app| app.runtime_settings.wasm_module_sha256_input = "invalid".to_string(),
        "第 1 行模块摘要必须使用 module:sha256 格式",
    );
}

#[test]
fn valid_runtime_settings_schedule_persistence_task() {
    let mut app = app();
    app.runtime_settings.docker_memory_limit_mb_input = String::new();
    app.runtime_settings.docker_cpu_limit_input = "1.25".to_string();
    app.runtime_settings.wasm_module_sha256_input = "tool-a:abc, tool-b:def".to_string();
    app.runtime_settings.reasoning_enabled_input = "auto".to_string();
    app.runtime_settings.reasoning_level_input = "high".to_string();

    let task = persist_runtime_settings(&mut app).expect("valid settings should persist");

    assert!(iced_runtime::task::into_stream(task.collect()).is_some());
}
