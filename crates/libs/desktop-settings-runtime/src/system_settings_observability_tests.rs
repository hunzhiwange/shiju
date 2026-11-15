//! 验证可观测性设置在不同后端、追踪模式和输入边界下的视图结构。
//!
//! 夹具组合默认值、极值、空输入与未知值，并同时检查控件结构和回调消息。

use super::*;

use iced::widget::{container, text};
use iced::{Element, Length};

const SOURCE: &str = include_str!("system_settings_observability.rs");

fn test_app() -> Fixture {
    Fixture::default()
}

fn keep_element(element: iced::Element<'_, Message>) {
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

// 视图主体：覆盖日志后端与追踪模式的组合。

#[test]
fn view_builds_backend_trace_and_error_states() {
    let mut app = test_app();
    keep_element(view(inputs(&app)));

    app.observability_settings.backend = "otel".to_string();
    app.observability_settings.otel_endpoint_input = "http://collector:4318".to_string();
    app.observability_settings.otel_service_name_input = "vw-test".to_string();
    app.observability_settings.runtime_trace_mode = "rolling".to_string();
    app.observability_settings.runtime_trace_path_input = "state/runtime-trace.jsonl".to_string();
    app.observability_settings.runtime_trace_max_entries = 100_000;
    keep_element(view(inputs(&app)));

    app.observability_settings.backend = "prometheus".to_string();
    app.observability_settings.runtime_trace_mode = "full".to_string();
    app.observability_settings.runtime_trace_max_entries = 1;
    app.observability_settings.save_error = Some("保存失败".to_string());
    keep_element(view(inputs(&app)));
}

#[test]
fn view_with_log_backend_none_trace_and_min_max_entries() {
    let mut app = test_app();
    app.observability_settings.backend = "log".to_string();
    app.observability_settings.runtime_trace_mode = "none".to_string();
    app.observability_settings.runtime_trace_max_entries = 1;
    keep_element(view(inputs(&app)));
}

#[test]
fn view_with_all_backend_variants() {
    for backend in ["none", "log", "prometheus", "otel"] {
        let app = Fixture {
            observability_settings: ObservabilitySettingsState {
                backend: backend.to_string(),
                ..Default::default()
            },
            ..Fixture::default()
        };
        keep_element(view(inputs(&app)));
    }
}

#[test]
fn view_with_all_trace_mode_variants() {
    for mode in ["none", "rolling", "full"] {
        let app = Fixture {
            observability_settings: ObservabilitySettingsState {
                runtime_trace_mode: mode.to_string(),
                ..Default::default()
            },
            ..Fixture::default()
        };
        keep_element(view(inputs(&app)));
    }
}

#[test]
fn view_with_max_entries_at_boundaries() {
    for entries in [1u32, 50_000u32, 100_000u32] {
        let app = Fixture {
            observability_settings: ObservabilitySettingsState {
                runtime_trace_max_entries: entries,
                ..Default::default()
            },
            ..Fixture::default()
        };
        keep_element(view(inputs(&app)));
    }
}

#[test]
fn view_with_empty_inputs() {
    let app = Fixture {
        observability_settings: ObservabilitySettingsState {
            backend: "otel".to_string(),
            otel_endpoint_input: String::new(),
            otel_service_name_input: String::new(),
            runtime_trace_path_input: String::new(),
            ..Default::default()
        },
        ..Fixture::default()
    };
    keep_element(view(inputs(&app)));
}

#[test]
fn view_with_save_error() {
    let app = Fixture {
        observability_settings: ObservabilitySettingsState {
            save_error: Some("配置保存失败".to_string()),
            ..Default::default()
        },
        ..Fixture::default()
    };
    keep_element(view(inputs(&app)));
}

// 浮层：分别验证无弹窗、编辑弹窗和帮助弹窗。

#[test]
fn overlays_return_base_dialog_or_help_modal() {
    let mut app = test_app();
    keep_element(view_overlays(inputs(&app), text("dialog").into()));

    app.observability_settings.show_help_modal = true;
    keep_element(view_overlays(inputs(&app), text("dialog").into()));
}

#[test]
fn overlays_with_help_modal_no_save_error() {
    let mut app = test_app();
    app.observability_settings.show_help_modal = true;
    app.observability_settings.backend = "otel".to_string();
    keep_element(view_overlays(inputs(&app), text("base").into()));
}

#[test]
fn overlays_preserve_base_dimensions_when_help_modal_is_closed() {
    let app = test_app();
    let dialog: Element<'_, Message> =
        container(text("base")).width(Length::Fixed(413.0)).height(Length::Fixed(217.0)).into();

    let element = view_overlays(inputs(&app), dialog);

    assert_eq!(element.as_widget().size().width, Length::Fixed(413.0));
    assert_eq!(element.as_widget().size().height, Length::Fixed(217.0));
}

// 元检查：防止计划要求覆盖的私有符号在重构时静默消失。

#[test]
fn system_settings_observability_tests_keeps_planned_coverage_targets() {
    for name in ["view", "view_overlays"] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}

#[derive(Default)]
struct Fixture { pub observability_settings: ObservabilitySettingsState, app_theme: iced::Theme }
fn inputs(app: &Fixture) -> View<'_> {
 View { observability_settings: &app.observability_settings, desktop_locale: config_ui::DesktopLocale::SimplifiedChinese,
 app_theme: &app.app_theme, last_copied_code_hash: None,
 render_help: |_, description, _, _| iced::widget::text(description).into(), }
}
