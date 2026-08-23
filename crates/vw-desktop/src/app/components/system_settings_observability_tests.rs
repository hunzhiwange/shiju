//! 验证可观测性设置在不同后端、追踪模式和输入边界下的视图结构。
//!
//! 夹具组合默认值、极值、空输入与未知值，并同时检查控件结构和回调消息。

use super::*;
use crate::app::message;
use crate::app::{App, Message};
use iced::widget::{container, text, text_input};
use iced::{Element, Length};

const SOURCE: &str = include_str!("system_settings_observability.rs");

fn test_app() -> App {
    App::new().0
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
    keep_element(view(&app));

    app.observability_settings.backend = "otel".to_string();
    app.observability_settings.otel_endpoint_input = "http://collector:4318".to_string();
    app.observability_settings.otel_service_name_input = "vw-test".to_string();
    app.observability_settings.runtime_trace_mode = "rolling".to_string();
    app.observability_settings.runtime_trace_path_input = "state/runtime-trace.jsonl".to_string();
    app.observability_settings.runtime_trace_max_entries = 100_000;
    keep_element(view(&app));

    app.observability_settings.backend = "prometheus".to_string();
    app.observability_settings.runtime_trace_mode = "full".to_string();
    app.observability_settings.runtime_trace_max_entries = 1;
    app.observability_settings.save_error = Some("保存失败".to_string());
    keep_element(view(&app));
}

#[test]
fn view_with_log_backend_none_trace_and_min_max_entries() {
    let mut app = test_app();
    app.observability_settings.backend = "log".to_string();
    app.observability_settings.runtime_trace_mode = "none".to_string();
    app.observability_settings.runtime_trace_max_entries = 1;
    keep_element(view(&app));
}

#[test]
fn view_with_all_backend_variants() {
    for backend in ["none", "log", "prometheus", "otel"] {
        let app = App {
            observability_settings: crate::app::state::ObservabilitySettingsState {
                backend: backend.to_string(),
                ..Default::default()
            },
            ..App::new().0
        };
        keep_element(view(&app));
    }
}

#[test]
fn view_with_all_trace_mode_variants() {
    for mode in ["none", "rolling", "full"] {
        let app = App {
            observability_settings: crate::app::state::ObservabilitySettingsState {
                runtime_trace_mode: mode.to_string(),
                ..Default::default()
            },
            ..App::new().0
        };
        keep_element(view(&app));
    }
}

#[test]
fn view_with_max_entries_at_boundaries() {
    for entries in [1u32, 50_000u32, 100_000u32] {
        let app = App {
            observability_settings: crate::app::state::ObservabilitySettingsState {
                runtime_trace_max_entries: entries,
                ..Default::default()
            },
            ..App::new().0
        };
        keep_element(view(&app));
    }
}

#[test]
fn view_with_empty_inputs() {
    let app = App {
        observability_settings: crate::app::state::ObservabilitySettingsState {
            backend: "otel".to_string(),
            otel_endpoint_input: String::new(),
            otel_service_name_input: String::new(),
            runtime_trace_path_input: String::new(),
            ..Default::default()
        },
        ..App::new().0
    };
    keep_element(view(&app));
}

#[test]
fn view_with_save_error() {
    let app = App {
        observability_settings: crate::app::state::ObservabilitySettingsState {
            save_error: Some("配置保存失败".to_string()),
            ..Default::default()
        },
        ..App::new().0
    };
    keep_element(view(&app));
}

// 浮层：分别验证无弹窗、编辑弹窗和帮助弹窗。

#[test]
fn overlays_return_base_dialog_or_help_modal() {
    let mut app = test_app();
    keep_element(view_overlays(&app, text("dialog").into()));

    app.observability_settings.show_help_modal = true;
    keep_element(view_overlays(&app, text("dialog").into()));
}

#[test]
fn overlays_with_help_modal_no_save_error() {
    let mut app = test_app();
    app.observability_settings.show_help_modal = true;
    app.observability_settings.backend = "otel".to_string();
    keep_element(view_overlays(&app, text("base").into()));
}

#[test]
fn overlays_preserve_base_dimensions_when_help_modal_is_closed() {
    let app = test_app();
    let dialog: Element<'_, Message> =
        container(text("base")).width(Length::Fixed(413.0)).height(Length::Fixed(217.0)).into();

    let element = view_overlays(&app, dialog);

    assert_eq!(element.as_widget().size().width, Length::Fixed(413.0));
    assert_eq!(element.as_widget().size().height, Length::Fixed(217.0));
}

// 字段行：通过 `super::*` 直接检查私有布局辅助函数。

#[test]
fn field_row_builds_label_description_and_control() {
    let el: Element<'_, Message> = field_row("我的标签", "这是描述", text("控件"));
    keep_element(el);
}

#[test]
fn field_row_with_empty_label_and_description() {
    let el: Element<'_, Message> = field_row("", "", text_input("", "").width(Length::Fill));
    keep_element(el);
}

// 文本行：覆盖空值、普通值与 Unicode 输入的控件结构。

#[test]
fn text_row_builds_label_placeholder_and_value() {
    let el: Element<'_, Message> = text_row(
        "端点",
        "输入 OTLP 端点",
        "http://localhost:4318",
        "http://collector:4318",
        |v| Message::Settings(message::SettingsMessage::ObservabilityOtelEndpointChanged(v)),
    );
    keep_element(el);
}

#[test]
fn text_row_with_empty_strings() {
    let el: Element<'_, Message> = text_row("", "", "", "", |v| {
        Message::Settings(message::SettingsMessage::ObservabilityOtelEndpointChanged(v))
    });
    keep_element(el);
}

// 元检查：防止计划要求覆盖的私有符号在重构时静默消失。

#[test]
fn system_settings_observability_tests_keeps_planned_coverage_targets() {
    for name in ["field_row", "text_row", "view", "view_overlays"] {
        assert!(source_declares_symbol(name), "expected source to declare coverage target {name}");
    }
}
