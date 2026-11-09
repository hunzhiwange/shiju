//! 可观测性配置设置界面组件
//!
//! 本模块提供可观测性（Observability）配置的可视化设置界面，用于配置
//! VibeWindow 运行时的观测后端和追踪存储行为。配置通过 Gateway 保存到
//! 共享数据库的 `observability` 字段。
//!
//! # 主要功能
//!
//! - **后端选择**：支持 none、log、prometheus、otel 四种观测后端
//! - **OTEL 配置**：配置 OpenTelemetry 的端点和服务名
//! - **运行时追踪**：支持 none、rolling、full 三种追踪模式
//! - **追踪参数**：配置追踪文件路径和最大条目数
//! - **帮助文档**：提供详细的配置说明模态框
//!
//! # 配置字段说明
//!
//! - `backend`：观测后端类型
//! - `otel_endpoint`：OTLP 采集端点（仅 otel 模式）
//! - `otel_service_name`：OTel 服务名（仅 otel 模式）
//! - `runtime_trace_mode`：运行时追踪模式
//! - `runtime_trace_path`：追踪文件路径
//! - `runtime_trace_max_entries`：rolling 模式下保留的最大条目数

use crate::app::components::system_settings_common::{
    settings_control_with_hint, settings_divider, settings_flexible_row_with_help,
    settings_group_card, settings_group_heading_with_help, settings_help_button_with_locale,
    settings_page_heading_with_help, settings_select, settings_text_input, settings_value_badge,
};
use crate::app::i18n::I18n;
use crate::app::i18n::settings_runtime::text as runtime_text;
use crate::app::{App, Message, message};
use iced::widget::{column, container, row, slider};
use iced::{Alignment, Element, Length};

#[derive(Clone, PartialEq)]
struct LabeledOption {
    value: &'static str,
    label: &'static str,
}

impl std::fmt::Display for LabeledOption {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label)
    }
}

const CONTROL_WIDTH: f32 = 320.0;

/// 渲染观测后端、OTel 连接与运行时追踪设置。
pub fn view(app: &App) -> Element<'_, Message> {
    let locale = app.desktop_locale;
    let s = &app.observability_settings;
    let i18n = I18n::new(locale);
    let theme = &app.app_theme;
    let help_btn = settings_help_button_with_locale(
        locale,
        Message::Settings(message::SettingsMessage::ObservabilityHelpOpen),
    );

    let backend_options = [
        LabeledOption { value: "none", label: runtime_text(locale, "无") },
        LabeledOption { value: "log", label: runtime_text(locale, "日志") },
        LabeledOption { value: "prometheus", label: "Prometheus" },
        LabeledOption { value: "otel", label: "OTel" },
    ];
    let selected_backend = backend_options.iter().find(|option| option.value == s.backend).cloned();
    let backend_row = settings_flexible_row_with_help(
        i18n,
        theme,
        runtime_text(locale, "后端"),
        runtime_text(locale, "选择运行时观测后端。"),
        CONTROL_WIDTH,
        settings_select(backend_options, selected_backend, |option| {
            Message::Settings(message::SettingsMessage::ObservabilityBackendChanged(
                option.value.to_string(),
            ))
        })
        .width(Length::Fill),
    );

    let inactive = runtime_text(locale, "当前模式下不生效。");
    let otel_endpoint = settings_text_input("http://localhost:4318", &s.otel_endpoint_input)
        .on_input(|value| {
            Message::Settings(message::SettingsMessage::ObservabilityOtelEndpointChanged(value))
        })
        .width(Length::Fill);
    let otel_endpoint_row = settings_flexible_row_with_help(
        i18n,
        theme,
        runtime_text(locale, "OTEL 端点"),
        runtime_text(locale, "仅 backend=otel 时生效。"),
        CONTROL_WIDTH,
        if s.backend == "otel" {
            otel_endpoint.into()
        } else {
            settings_control_with_hint(otel_endpoint, inactive)
        },
    );

    let otel_service_row = settings_flexible_row_with_help(
        i18n,
        theme,
        runtime_text(locale, "OTEL 服务名"),
        runtime_text(locale, "OTel 上报时使用的 service.name。"),
        CONTROL_WIDTH,
        settings_text_input("vibewindow", &s.otel_service_name_input)
            .on_input(|value| {
                Message::Settings(message::SettingsMessage::ObservabilityOtelServiceNameChanged(
                    value,
                ))
            })
            .width(Length::Fill),
    );

    let trace_options = [
        LabeledOption { value: "none", label: runtime_text(locale, "无") },
        LabeledOption { value: "rolling", label: runtime_text(locale, "滚动") },
        LabeledOption { value: "full", label: runtime_text(locale, "完整") },
    ];
    let selected_trace =
        trace_options.iter().find(|option| option.value == s.runtime_trace_mode).cloned();
    let trace_mode_row = settings_flexible_row_with_help(
        i18n,
        theme,
        runtime_text(locale, "运行时追踪"),
        runtime_text(locale, "选择 none、rolling 或 full 模式。"),
        CONTROL_WIDTH,
        settings_select(trace_options, selected_trace, |option| {
            Message::Settings(message::SettingsMessage::ObservabilityRuntimeTraceModeChanged(
                option.value.to_string(),
            ))
        })
        .width(Length::Fill),
    );

    let trace_path_row = settings_flexible_row_with_help(
        i18n,
        theme,
        runtime_text(locale, "追踪文件"),
        runtime_text(locale, "运行时追踪文件路径。"),
        CONTROL_WIDTH,
        settings_text_input("state/runtime-trace.jsonl", &s.runtime_trace_path_input)
            .on_input(|value| {
                Message::Settings(message::SettingsMessage::ObservabilityRuntimeTracePathChanged(
                    value,
                ))
            })
            .width(Length::Fill),
    );

    let max_entries_slider = slider(1.0..=100_000.0, s.runtime_trace_max_entries as f32, |v| {
        Message::Settings(message::SettingsMessage::ObservabilityRuntimeTraceMaxEntriesChanged(
            v.round() as u32,
        ))
    })
    .width(Length::Fill);

    let max_entries =
        row![max_entries_slider, settings_value_badge(s.runtime_trace_max_entries.to_string())]
            .spacing(16)
            .align_y(Alignment::Center);
    let max_entries_row = settings_flexible_row_with_help(
        i18n,
        theme,
        runtime_text(locale, "rolling 条数"),
        runtime_text(locale, "rolling 模式下保留的最大条目数。"),
        CONTROL_WIDTH,
        if s.runtime_trace_mode == "rolling" {
            max_entries.into()
        } else {
            settings_control_with_hint(max_entries, inactive)
        },
    );

    let col = column![
        row![
            container(settings_page_heading_with_help(
                i18n,
                theme,
                runtime_text(locale, "可观测性配置"),
                runtime_text(locale, "配置运行时观测后端、OTEL 上报和追踪存储策略。")
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        settings_group_heading_with_help(
            i18n,
            theme,
            runtime_text(locale, "观测后端"),
            runtime_text(locale, "选择观测后端并配置 OTel 端点。")
        ),
        settings_group_card(
            column![
                backend_row,
                settings_divider(),
                otel_endpoint_row,
                settings_divider(),
                otel_service_row
            ]
            .spacing(0),
        ),
        settings_group_heading_with_help(
            i18n,
            theme,
            runtime_text(locale, "运行时追踪"),
            runtime_text(locale, "控制 trace 模式、文件路径和 rolling 保留量。")
        ),
        settings_group_card(
            column![
                trace_mode_row,
                settings_divider(),
                trace_path_row,
                settings_divider(),
                max_entries_row
            ]
            .spacing(0),
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    col.into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let locale = app.desktop_locale;
    let s = &app.observability_settings;
    if !s.show_help_modal {
        return dialog;
    }

    let help_text = crate::app::i18n::settings_runtime::observability_help(locale);

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        runtime_text(locale, "可观测性配置帮助"),
        help_text,
        Message::Settings(message::SettingsMessage::ObservabilityHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_observability_tests.rs"]
mod system_settings_observability_tests;
