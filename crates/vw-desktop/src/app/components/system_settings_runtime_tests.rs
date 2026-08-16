use super::*;
use crate::app::App;
use iced::Element;
use iced::advanced::widget::Tree;
use iced::widget::text;

fn test_app() -> App {
    App::new().0
}

fn element_tree(element: Element<'_, Message>) -> Tree {
    Tree::new(element.as_widget())
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
fn runtime_rows_accept_controls_and_callbacks() {
    let field: Element<'_, Message> = field_row("运行时类型", "选择执行环境。", text("native"));
    let input: Element<'_, Message> =
        text_row("镜像", "默认执行镜像。", "alpine:3.20", "ubuntu:24.04", |_| {
            Message::GatewayHealthTick
        });
    let hinted_input: Element<'_, Message> = text_row_with_hint(
        "根目录",
        "说明",
        "路径",
        "/workspace",
        "建议仅放行明确需要挂载的根目录。",
        |_| Message::GatewayHealthTick,
    );

    assert_eq!(element_tree(field).children.len(), 2);
    assert_eq!(element_tree(input).children.len(), 2);
    assert_eq!(element_tree(hinted_input).children[1].children.len(), 2);
}

#[test]
fn runtime_reasoning_options_cover_auto_enabled_disabled_and_levels() {
    let source = include_str!("system_settings_runtime.rs");

    assert!(source.contains("value: \"auto\", label: \"自动\""));
    assert!(source.contains("value: \"true\", label: \"启用\""));
    assert!(source.contains("value: \"false\", label: \"禁用\""));
    assert!(source.contains("value: \"minimal\", label: \"最小\""));
    assert!(source.contains("value: \"xhigh\", label: \"超高\""));
}

#[test]
fn view_builds_native_runtime_settings() {
    let mut app = test_app();
    app.runtime_settings.kind = "native".to_string();
    let _: Element<'_, Message> = view(&app);

    app.runtime_settings.reasoning_enabled_input = "unknown".to_string();
    app.runtime_settings.reasoning_level_input = "unknown".to_string();
    let _: Element<'_, Message> = view(&app);
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

    let _: Element<'_, Message> = view(&app);
}

#[test]
fn view_builds_wasm_runtime_settings() {
    let mut app = test_app();
    app.runtime_settings.kind = "wasm".to_string();
    app.runtime_settings.wasm_tools_dir = "tools/custom".to_string();
    app.runtime_settings.wasm_fuel_limit_input = "2000000".to_string();
    app.runtime_settings.wasm_memory_limit_mb_input = "128".to_string();
    app.runtime_settings.wasm_max_module_size_mb_input = "80".to_string();
    app.runtime_settings.wasm_allow_workspace_read = true;
    app.runtime_settings.wasm_allow_workspace_write = true;
    app.runtime_settings.wasm_allowed_hosts_input = "example.com:443\napi.example.com".to_string();
    app.runtime_settings.wasm_require_workspace_relative_tools_dir = false;
    app.runtime_settings.wasm_reject_symlink_modules = false;
    app.runtime_settings.wasm_reject_symlink_tools_dir = false;
    app.runtime_settings.wasm_strict_host_validation = false;
    app.runtime_settings.wasm_capability_escalation_mode = "clamp".to_string();
    app.runtime_settings.wasm_module_hash_policy = "enforce".to_string();
    app.runtime_settings.wasm_module_sha256_input =
        "tool:0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef".to_string();
    app.runtime_settings.reasoning_enabled_input = "false".to_string();
    app.runtime_settings.reasoning_level_input = "xhigh".to_string();

    let _: Element<'_, Message> = view(&app);
}

#[test]
fn view_adds_only_the_selected_runtime_specific_section() {
    let mut app = test_app();

    app.runtime_settings.kind = "native".to_string();
    assert_eq!(element_tree(view(&app)).children.len(), 5);

    app.runtime_settings.kind = "unknown".to_string();
    assert_eq!(element_tree(view(&app)).children.len(), 5);

    app.runtime_settings.kind = "docker".to_string();
    let docker = element_tree(view(&app));
    assert_eq!(docker.children.len(), 6);
    assert_eq!(docker.children[5].children.len(), 2);

    app.runtime_settings.kind = "wasm".to_string();
    let wasm = element_tree(view(&app));
    assert_eq!(wasm.children.len(), 6);
    assert_eq!(wasm.children[5].children.len(), 2);
}
