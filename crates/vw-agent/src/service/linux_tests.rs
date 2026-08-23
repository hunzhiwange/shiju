//! 验证 systemd 参数引用、配置目录和服务文件定位。

use super::InitSystem;
use super::linux::{
    install_linux, linux_service_file, restart_linux, start_linux, status_linux, stop_linux,
    systemd_config_dir_args, systemd_quote_arg, uninstall_linux,
};
use crate::app::agent::config::Config;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn systemd_quote_arg_leaves_safe_values_unquoted() {
    assert_eq!(systemd_quote_arg("/home/me/.config/vibewindow"), "/home/me/.config/vibewindow");
    assert_eq!(systemd_quote_arg("/tmp/with-dash_123:456"), "/tmp/with-dash_123:456");
}

#[test]
fn systemd_quote_arg_quotes_spaces_and_escapes_special_characters() {
    assert_eq!(
        systemd_quote_arg("/Users/me/Application Support/VibeWindow"),
        "\"/Users/me/Application Support/VibeWindow\""
    );
    assert_eq!(systemd_quote_arg(r#"/tmp/a"b\c"#), r#""/tmp/a\"b\\c""#);
}

#[test]
fn systemd_quote_arg_leaves_empty_value_unquoted() {
    assert_eq!(systemd_quote_arg(""), "");
}

#[test]
fn systemd_quote_arg_quotes_unicode_values() {
    assert_eq!(systemd_quote_arg("/home/测试/vibewindow"), "\"/home/测试/vibewindow\"");
}

#[test]
fn systemd_quote_arg_quotes_shell_metacharacters_without_rewriting_them() {
    for value in ["/tmp/$HOME", "/tmp/a;b", "/tmp/a=b", "/tmp/a\nb"] {
        assert_eq!(systemd_quote_arg(value), format!("\"{value}\""));
    }
}

#[test]
fn systemd_config_dir_args_uses_parent_directory_and_quotes_when_needed() {
    let mut config = Config::default();
    config.config_path = "/Users/me/Application Support/VibeWindow/vibewindow.json".into();

    let args = systemd_config_dir_args(&config);

    assert_eq!(args, " --config-dir \"/Users/me/Application Support/VibeWindow\"");
    assert!(!args.contains("vibewindow.json"));
}

#[test]
fn systemd_config_dir_args_returns_empty_when_config_path_has_no_parent() {
    let mut config = Config::default();
    config.config_path = "/".into();

    assert_eq!(systemd_config_dir_args(&config), "");
}

#[test]
fn systemd_config_dir_args_leaves_safe_absolute_parent_unquoted() {
    let mut config = Config::default();
    config.config_path = "/home/me/.config/vibewindow/vibewindow.json".into();

    assert_eq!(systemd_config_dir_args(&config), " --config-dir /home/me/.config/vibewindow");
}

#[test]
fn systemd_config_dir_args_preserves_relative_parent() {
    let mut config = Config::default();
    config.config_path = "profiles/dev/vibewindow.json".into();

    assert_eq!(systemd_config_dir_args(&config), " --config-dir profiles/dev");
}

#[test]
fn systemd_config_dir_args_uses_filesystem_root_parent() {
    let mut config = Config::default();
    config.config_path = "/vibewindow.json".into();

    assert_eq!(systemd_config_dir_args(&config), " --config-dir /");
}

#[test]
fn linux_service_file_points_to_user_systemd_unit() {
    let file = linux_service_file(&Config::default()).unwrap();
    assert!(file.ends_with(".config/systemd/user/vibewindow.service"));
}

#[test]
fn linux_service_file_does_not_depend_on_selected_config_path() {
    let default_file = linux_service_file(&Config::default()).unwrap();
    let mut config = Config::default();
    config.config_path = "/tmp/another-profile/vibewindow.json".into();

    assert_eq!(linux_service_file(&config).unwrap(), default_file);
}

#[test]
fn auto_init_system_branches_panic_before_running_platform_commands() {
    let config = Config::default();

    assert!(catch_unwind(AssertUnwindSafe(|| install_linux(&config, InitSystem::Auto))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| start_linux(InitSystem::Auto))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| stop_linux(InitSystem::Auto))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| restart_linux(InitSystem::Auto))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| status_linux(&config, InitSystem::Auto))).is_err());
    assert!(catch_unwind(AssertUnwindSafe(|| uninstall_linux(&config, InitSystem::Auto))).is_err());
}

#[cfg(not(target_os = "linux"))]
#[test]
fn non_linux_auto_resolves_to_systemd_placeholder() {
    assert_eq!(InitSystem::Auto.resolve().unwrap(), InitSystem::Systemd);
}
