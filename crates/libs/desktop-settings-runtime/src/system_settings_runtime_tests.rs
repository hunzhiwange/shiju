//! 验证本机与容器运行时选项、推理级别和条件设置区块。
//!
//! 本机与容器配置分别构造，确认视图可构建且选项标签稳定。

use super::*;

use iced::Element;

fn test_app() -> Fixture {
    Fixture::default()
}

#[test]
fn system_settings_runtime_tests_are_wired() {
    assert!(module_path!().contains("system_settings_runtime_tests"));
}

#[test]
fn labeled_option_displays_label_not_value() {
    let option = LabeledOption { value: "xhigh", label: "超高" };

    assert_eq!(option.to_string(), "超高");
}

#[test]
fn view_builds_native_runtime_settings() {
    let mut app = test_app();
    app.runtime_settings.kind = "native".to_string();
    let _: Element<'_, Message> = view(inputs(&app));

    app.runtime_settings.reasoning_enabled_input = "unknown".to_string();
    app.runtime_settings.reasoning_level_input = "unknown".to_string();
    let _: Element<'_, Message> = view(inputs(&app));
}

#[test]
fn view_builds_docker_runtime_settings() {
    let mut app = test_app();
    app.runtime_settings.kind = "docker".to_string();
    app.runtime_settings.docker_image = "ubuntu:24.04".to_string();
    app.runtime_settings.docker_network = "host".to_string();
    app.runtime_settings.docker_memory_limit_mb_input = String::new();
    app.runtime_settings.docker_cpu_limit_input = "1.5".to_string();
    app.runtime_settings.docker_read_only_rootfs = false;
    app.runtime_settings.docker_mount_workspace = false;
    app.runtime_settings.docker_allowed_workspace_roots_input =
        "/workspace/a\n/workspace/b".to_string();
    app.runtime_settings.reasoning_enabled_input = "true".to_string();
    app.runtime_settings.reasoning_level_input = "high".to_string();

    let _: Element<'_, Message> = view(inputs(&app));
}



struct Fixture { pub runtime_settings: RuntimeSettingsState, app_theme: iced::Theme }
fn inputs(app: &Fixture) -> View<'_> {
 View { runtime_settings: &app.runtime_settings,select_focus_scope: "system-settings-select-focus", desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
 app_theme: &app.app_theme,
 render_help: |_, description, _, _| iced::widget::text(description).into(), }
}

impl Default for Fixture { fn default() -> Self { Self { runtime_settings: Default::default(), app_theme: iced::Theme::Light } } }
