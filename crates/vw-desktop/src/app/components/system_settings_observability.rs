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
    SETTINGS_LABEL_WIDTH, settings_divider, settings_muted_text_style, settings_page_intro,
    settings_section_card, settings_section_content, settings_segment_button_style,
    settings_text_input_style, settings_value_badge,
};
use crate::app::i18n::settings_runtime::text as runtime_text;
use crate::app::{App, Message, message};
use iced::widget::{button, column, container, row, slider, text, text_input};
use iced::{Alignment, Element, Length};

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            container(control.into()).width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn text_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        text_input(placeholder, value)
            .on_input(on_input)
            .padding([10, 12])
            .size(13)
            .style(settings_text_input_style)
            .width(Length::Fill),
    )
}

/// 构建可观测性配置设置界面的视图
///
/// 该函数创建一个完整的可观测性配置界面，包含后端选择、OTEL 配置、
/// 运行时追踪配置等选项，并支持显示帮助模态框。
///
/// # 参数
///
/// - `app`：应用状态引用，从中获取 `observability_settings` 配置数据
///
/// # 返回值
///
/// 返回一个 Iced `Element`，包含完整的可观测性配置界面
///
/// # 界面布局
///
/// 界面从上到下依次包含：
/// 1. 标题栏（标题 + 帮助按钮）
/// 2. 副标题（配置文件路径说明）
/// 3. 后端选择行（四个单选按钮）
/// 4. OTEL 端点输入行（文本输入框）
/// 5. OTEL 服务名输入行（文本输入框）
/// 6. 运行时追踪模式行（三个单选按钮）
/// 7. 追踪文件路径行（文本输入框）
/// 8. rolling 条数配置行（滑块 + 数值显示）
/// 9. 错误信息（如有保存错误）
///
/// # 示例
///
/// ```ignore
/// let element = view(&app);
/// // element 可直接用于 Iced 应用的视图渲染
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    let locale = app.desktop_locale;
    let s = &app.observability_settings;
    let help_btn = crate::app::components::system_settings_common::icon_btn(
        crate::app::assets::Icon::QuestionCircle,
        crate::app::i18n::settings_runtime::text(locale, "帮助"),
        Some(Message::Settings(message::SettingsMessage::ObservabilityHelpOpen)),
    );

    let backend_btn = |label: &'static str, v: &'static str| {
        let is_active = s.backend == v;
        button(text(label))
            .on_press(Message::Settings(message::SettingsMessage::ObservabilityBackendChanged(
                v.to_string(),
            )))
            .padding([6, 10])
            .style(move |theme: &iced::Theme, status| {
                settings_segment_button_style(theme, status, is_active)
            })
    };

    let backend_row = field_row(
        runtime_text(locale, "后端"),
        runtime_text(locale, "选择运行时观测后端。"),
        row![
            backend_btn(runtime_text(locale, "无"), "none"),
            backend_btn(runtime_text(locale, "日志"), "log"),
            backend_btn("Prometheus", "prometheus"),
            backend_btn("OTel", "otel"),
        ]
        .spacing(8),
    );

    let otel_endpoint_row = text_row(
        runtime_text(locale, "OTEL 端点"),
        runtime_text(locale, "仅 backend=otel 时生效。"),
        "http://localhost:4318",
        &s.otel_endpoint_input,
        |v| Message::Settings(message::SettingsMessage::ObservabilityOtelEndpointChanged(v)),
    );

    let otel_service_row = text_row(
        runtime_text(locale, "OTEL 服务名"),
        runtime_text(locale, "OTel 上报时使用的 service.name。"),
        "vibewindow",
        &s.otel_service_name_input,
        |v| Message::Settings(message::SettingsMessage::ObservabilityOtelServiceNameChanged(v)),
    );

    let trace_mode_btn = |label: &'static str, v: &'static str| {
        let is_active = s.runtime_trace_mode == v;
        button(text(label))
            .on_press(Message::Settings(
                message::SettingsMessage::ObservabilityRuntimeTraceModeChanged(v.to_string()),
            ))
            .padding([6, 10])
            .style(move |theme: &iced::Theme, status| {
                settings_segment_button_style(theme, status, is_active)
            })
    };

    let trace_mode_row = field_row(
        runtime_text(locale, "运行时追踪"),
        runtime_text(locale, "选择 none、rolling 或 full 模式。"),
        row![
            trace_mode_btn(runtime_text(locale, "无"), "none"),
            trace_mode_btn(runtime_text(locale, "滚动"), "rolling"),
            trace_mode_btn(runtime_text(locale, "完整"), "full"),
        ]
        .spacing(8),
    );

    let trace_path_row = text_row(
        runtime_text(locale, "追踪文件"),
        runtime_text(locale, "运行时追踪文件路径。"),
        "state/runtime-trace.jsonl",
        &s.runtime_trace_path_input,
        |v| Message::Settings(message::SettingsMessage::ObservabilityRuntimeTracePathChanged(v)),
    );

    let max_entries_slider = slider(1.0..=100_000.0, s.runtime_trace_max_entries as f32, |v| {
        Message::Settings(message::SettingsMessage::ObservabilityRuntimeTraceMaxEntriesChanged(
            v.round() as u32,
        ))
    })
    .width(Length::Fixed(280.0));

    let max_entries_row = field_row(
        runtime_text(locale, "rolling 条数"),
        runtime_text(locale, "rolling 模式下保留的最大条目数。"),
        row![max_entries_slider, settings_value_badge(format!("{}", s.runtime_trace_max_entries))]
            .spacing(16)
            .align_y(Alignment::Center),
    );

    let col = column![
        row![
            container(settings_page_intro(
                runtime_text(locale, "可观测性配置"),
                runtime_text(locale, "配置运行时观测后端、OTEL 上报和追踪存储策略。")
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        settings_section_card(
            runtime_text(locale, "观测后端"),
            runtime_text(locale, "选择观测后端并配置 OTel 端点。")
        ),
        settings_section_content(
            column![
                backend_row,
                settings_divider(),
                otel_endpoint_row,
                settings_divider(),
                otel_service_row
            ]
            .spacing(0),
        ),
        settings_section_card(
            runtime_text(locale, "运行时追踪"),
            runtime_text(locale, "控制 trace 模式、文件路径和 rolling 保留量。")
        ),
        settings_section_content(
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
