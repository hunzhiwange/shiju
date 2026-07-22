//! 转录配置设置界面组件
//!
//! 本模块提供语音/音频转录功能的配置界面，允许用户：
//! - 启用或禁用转录功能
//! - 配置转录 API 的 URL
//! - 指定转录模型名称
//! - 设置转录语言（可选）
//! - 限制最大转录音频时长
//!
//! 该界面会生成对应的消息来更新应用程序状态，
//! 最终配置将通过 Gateway 保存到共享数据库的 `transcription` 字段中。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_divider, settings_help_button,
    settings_muted_text_style, settings_page_intro, settings_panel, settings_section_card,
    settings_text_input_style, settings_value_badge,
};
use crate::app::{App, Message, message};
use iced::widget::{checkbox, column, container, row, slider, text, text_input};
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

fn enabled_toggled(value: bool) -> Message {
    Message::Settings(message::SettingsMessage::TranscriptionEnabledToggled(value))
}

fn api_url_changed(value: String) -> Message {
    Message::Settings(message::SettingsMessage::TranscriptionApiUrlChanged(value))
}

fn model_changed(value: String) -> Message {
    Message::Settings(message::SettingsMessage::TranscriptionModelChanged(value))
}

fn language_changed(value: String) -> Message {
    Message::Settings(message::SettingsMessage::TranscriptionLanguageChanged(value))
}

fn max_duration_secs_changed(value: f32) -> Message {
    Message::Settings(message::SettingsMessage::TranscriptionMaxDurationSecsChanged(
        value.round() as u64
    ))
}

/// 渲染转录配置设置界面
///
/// 该函数创建一个包含多个配置选项的表单界面，用于管理语音/音频转录功能的设置。
///
/// # 参数
///
/// * `app` - 应用程序状态的引用，包含当前的转录配置
///
/// # 返回值
///
/// 返回一个 `Element<Message>`，表示渲染后的 UI 元素。
/// 如果帮助模态框被打开，返回值将是一个包含模态层的堆叠布局。
///
/// # 界面组成
///
/// 1. 标题和帮助按钮
/// 2. 启用开关
/// 3. API URL 输入框
/// 4. 模型名称输入框
/// 5. 语言代码输入框
/// 6. 最大音频时长滑块
/// 7. 可选的帮助模态框
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.transcription_settings;
    let help_btn =
        settings_help_button(Message::Settings(message::SettingsMessage::TranscriptionHelpOpen));

    let enabled_row = field_row(
        "启用",
        "控制是否启用语音/音频转录。",
        checkbox(s.enabled)
            .label("启用语音转录（支持语音/音频消息的频道）")
            .on_toggle(enabled_toggled)
            .style(settings_checkbox_style),
    );

    let api_url_row = text_row(
        "API URL",
        "转录服务的请求地址。",
        "https://api.groq.com/openai/v1/audio/transcriptions",
        &s.api_url,
        api_url_changed,
    );

    let model_row =
        text_row("模型", "转录模型名称。", "whisper-large-v3-turbo", &s.model, model_changed);

    let language_row = text_row(
        "语言",
        "可选语言代码，留空时自动检测。",
        "可选，例如 en / zh（留空为自动）",
        &s.language,
        language_changed,
    );

    let max_duration_slider =
        slider(1.0..=3600.0, s.max_duration_secs as f32, max_duration_secs_changed)
            .width(Length::Fixed(280.0));

    let max_duration_row = field_row(
        "最长音频",
        "可转录音频的最大时长。",
        row![max_duration_slider, settings_value_badge(format!("{} s", s.max_duration_secs)),]
            .spacing(16)
            .align_y(Alignment::Center),
    );

    let col = column![
        row![
            container(settings_page_intro(
                "转录配置",
                "配置语音/音频转录功能的开关、接口和模型参数。"
            ))
            .width(Length::Fill),
            help_btn
        ]
        .align_y(Alignment::Start),
        settings_section_card("基础行为", "控制启用状态与转录接口参数。"),
        settings_panel(
            column![
                enabled_row,
                settings_divider(),
                api_url_row,
                settings_divider(),
                model_row,
                settings_divider(),
                language_row,
                settings_divider(),
                max_duration_row,
            ]
            .spacing(0),
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    col.into()
}

pub fn view_overlays<'a>(app: &'a App, dialog: Element<'a, Message>) -> Element<'a, Message> {
    let s = &app.transcription_settings;
    if !s.show_help_modal {
        return dialog;
    }

    let help_text = r#"转录配置说明

一、作用
- transcription 用于配置语音/音频消息的自动转录行为。
- 当前默认对接 Groq Whisper API。

二、字段含义
1) enabled
- 是否启用转录功能。

2) api_url
- 转录 API 地址。
- 默认值：https://api.groq.com/openai/v1/audio/transcriptions

3) model
- 转录模型名称。
- 默认值：whisper-large-v3-turbo

4) language
- 可选语言代码（例如 "en"、"zh"）。
- 为空时由后端自动检测。

5) max_duration_secs
- 可转录音频的最大时长（秒）。
- UI 范围：1-3600，默认值：120。

三、示例
{
  "transcription": {
    "enabled": false,
    "api_url": "https://api.groq.com/openai/v1/audio/transcriptions",
    "model": "whisper-large-v3-turbo",
    "language": null,
    "max_duration_secs": 120
  }
}
"#;

    crate::app::components::system_settings_common::with_settings_help_modal(
        app,
        dialog,
        "Transcription 配置帮助",
        help_text,
        Message::Settings(message::SettingsMessage::TranscriptionHelpClose),
    )
}
#[cfg(test)]
#[path = "system_settings_transcription_tests.rs"]
mod system_settings_transcription_tests;
