//! 系统设置中 multimodal 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    SETTINGS_CONTROL_TEXT_SIZE, settings_checkbox_style, settings_divider, settings_group_card,
    settings_group_heading, settings_muted_text_style, settings_page_heading, settings_row,
    settings_select, settings_value_badge,
};
use crate::app::message::settings::{MultimodalMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{checkbox, column, row, text};
use iced::{Alignment, Element, Length};

const NO_IMAGE_MODEL_OPTION: &str = "请选择";

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    settings_row(label, description, 220.0, control)
}

fn number_row<'a>(
    label: &'static str,
    description: &'static str,
    value: u32,
    min: u32,
    max: u32,
    suffix: &'static str,
    on_change: impl Fn(u32) -> Message + 'a,
) -> Element<'a, Message> {
    let display = if suffix.is_empty() { value.to_string() } else { format!("{value} {suffix}") };

    field_row(
        label,
        description,
        row![
            NumberInput::new(value as f32, min as f32, max as f32, 1.0, 0, 0.15, move |raw| {
                on_change(raw.round() as u32)
            })
            .settings_style(),
            settings_value_badge(display),
        ]
        .spacing(16)
        .align_y(Alignment::Center),
    )
}

fn bool_row<'a>(
    label: &'static str,
    description: &'static str,
    checked: bool,
    checkbox_label: &'static str,
    on_toggle: impl Fn(bool) -> Message + 'a,
) -> Element<'a, Message> {
    field_row(
        label,
        description,
        checkbox(checked)
            .label(checkbox_label)
            .size(16)
            .text_size(SETTINGS_CONTROL_TEXT_SIZE)
            .on_toggle(on_toggle)
            .style(settings_checkbox_style),
    )
}

fn image_model_options(app: &App) -> Vec<String> {
    std::iter::once(
        crate::app::i18n::I18n::new(app.desktop_locale).__(NO_IMAGE_MODEL_OPTION).to_string(),
    )
    .chain(
        app.model_settings
            .providers
            .iter()
            .filter(|provider| !provider.id.trim().is_empty())
            .flat_map(|provider| {
                provider
                    .models
                    .iter()
                    .filter(|model| {
                        model.enabled
                            && !model.id.trim().is_empty()
                            && model.detail["capabilities"]["input"]["image"].as_bool()
                                == Some(true)
                    })
                    .map(|model| format!("{}/{}", provider.id, model.id))
            }),
    )
    .collect()
}

fn image_model_control(app: &App) -> Element<'_, Message> {
    let options = image_model_options(app);
    let configured = app.multimodal_settings.ocr_model.trim();
    let selected = if configured.is_empty() {
        Some(crate::app::i18n::I18n::new(app.desktop_locale).__(NO_IMAGE_MODEL_OPTION).to_string())
    } else {
        options.iter().find(|model| model.as_str() == configured).cloned()
    };
    let has_options = options.len() > 1;
    let configured_is_available = selected.is_some();

    let picker = settings_select(options, selected, move |value| {
        let model = (value
            != crate::app::i18n::I18n::new(app.desktop_locale).__(NO_IMAGE_MODEL_OPTION))
        .then_some(value)
        .unwrap_or_default();
        Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::OcrModelChanged(model)))
    })
    .placeholder(crate::app::i18n::I18n::new(app.desktop_locale).__("请选择支持图片输入的模型"))
    .width(Length::Fill);

    let mut status = column![picker].spacing(6);
    if app.model_settings.loading {
        status = status.push(
            text(crate::app::i18n::I18n::new(app.desktop_locale).__("正在刷新模型目录…"))
                .size(11)
                .style(settings_muted_text_style),
        );
    }
    if let Some(error) = app.model_settings.save_error.as_deref() {
        status = status.push(
            text(
                crate::__!(
                    crate::app::i18n::I18n::new(app.desktop_locale),
                    "模型目录刷新失败：%s",
                    error
                )
                .expect("valid image status template"),
            )
            .size(11)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .style(settings_muted_text_style),
        );
    }
    if !configured.is_empty() && !has_options {
        status = status.push(
            text(
                crate::__!(
                    crate::app::i18n::I18n::new(app.desktop_locale),
                    "当前配置：%s（目录中暂无可用匹配项）",
                    configured
                )
                .expect("valid image status template"),
            )
            .size(11)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .style(settings_muted_text_style),
        );
    } else if !configured.is_empty() && !configured_is_available {
        status = status.push(
            text(
                crate::__!(
                    crate::app::i18n::I18n::new(app.desktop_locale),
                    "当前配置：%s（模型未启用或不支持图片输入）",
                    configured
                )
                .expect("valid image status template"),
            )
            .size(11)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .style(settings_muted_text_style),
        );
    } else if !has_options {
        status = status.push(
            text(
                crate::app::i18n::I18n::new(app.desktop_locale)
                    .__("暂无已启用且明确支持图片输入的模型。"),
            )
            .size(11)
            .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
            .style(settings_muted_text_style),
        );
    }

    status.into()
}

/// 构建或处理 `view` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回可交给 Iced 渲染树使用的 `Element`，其中已绑定必要的消息回调。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn view(app: &App) -> Element<'_, Message> {
    let s = &app.multimodal_settings;

    let content = column![
        settings_page_heading(
            crate::app::i18n::I18n::new(app.desktop_locale).__("多模态配置"),
            crate::app::i18n::I18n::new(app.desktop_locale).__("配置本地图片附件的数量与大小上限，并决定是否允许抓取远程图片 URL。"),
        ),
        settings_group_heading(
            crate::app::i18n::I18n::new(app.desktop_locale).__("图像输入限制"),
            crate::app::i18n::I18n::new(app.desktop_locale).__("输入框附件优先使用本地文件；这里控制单次请求最多允许的图片数量、每张图片大小上限，以及是否允许抓取远程图片 URL。"),
        ),
        settings_group_card(
            column![
                number_row(crate::app::i18n::I18n::new(app.desktop_locale).__("最大图片数量"), crate::app::i18n::I18n::new(app.desktop_locale).__("单次请求最多允许上传的图片数。"), s.max_images, 1, 16, crate::app::i18n::I18n::new(app.desktop_locale).__("张"), |value| {
                    Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::MaxImagesChanged(
                        value,
                    )))
                }),
                settings_divider(),
                number_row(crate::app::i18n::I18n::new(app.desktop_locale).__("单张图片大小上限"), crate::app::i18n::I18n::new(app.desktop_locale).__("每张图片允许的最大体积。"), s.max_image_size_mb, 1, 20, "MB", |value| {
                    Message::Settings(SettingsMessage::Multimodal(
                        MultimodalMessage::MaxImageSizeMbChanged(value),
                    ))
                }),
                settings_divider(),
                bool_row(
                    crate::app::i18n::I18n::new(app.desktop_locale).__("允许远程抓取"),
                    crate::app::i18n::I18n::new(app.desktop_locale).__("本地附件始终可用；开启后才允许抓取 http/https 远程图片 URL。"),
                    s.allow_remote_fetch,
                    crate::app::i18n::I18n::new(app.desktop_locale).__("允许抓取 http/https 远程图片 URL"),
                    |value| {
                        Message::Settings(SettingsMessage::Multimodal(
                            MultimodalMessage::AllowRemoteFetchToggled(value),
                        ))
                    },
                ),
            ]
            .spacing(0),
        ),
        settings_group_heading(
            crate::app::i18n::I18n::new(app.desktop_locale).__("大模型视觉理解（含文字识别）"),
            crate::app::i18n::I18n::new(app.desktop_locale).__("使用多模态大模型识别图片文字并生成视觉摘要，再交给主模型处理。分析结果为有界、不可信的附件数据，可能省略图片细节。"),
        ),
        settings_group_card(
            column![
                bool_row(
                    crate::app::i18n::I18n::new(app.desktop_locale).__("模型支持图片"),
                    crate::app::i18n::I18n::new(app.desktop_locale).__("开启后优先由当前大模型直接理解图片，不先转换为文字和视觉摘要。"),
                    s.model_supports_images,
                    crate::app::i18n::I18n::new(app.desktop_locale).__("由当前模型直接处理图片"),
                    |value| {
                        Message::Settings(SettingsMessage::Multimodal(
                            MultimodalMessage::ModelSupportsImagesToggled(value),
                        ))
                    },
                ),
                settings_divider(),
                bool_row(
                    crate::app::i18n::I18n::new(app.desktop_locale).__("启用视觉模型分析"),
                    crate::app::i18n::I18n::new(app.desktop_locale).__("关闭“模型支持图片”后，使用下方多模态大模型识别文字并理解图片，无需本地 Python OCR 库。"),
                    s.ocr_enabled,
                    crate::app::i18n::I18n::new(app.desktop_locale).__("由多模态大模型分析图片附件"),
                    |value| {
                        Message::Settings(SettingsMessage::Multimodal(
                            MultimodalMessage::OcrEnabledToggled(value),
                        ))
                    },
                ),
                settings_divider(),
                field_row(
                    crate::app::i18n::I18n::new(app.desktop_locale).__("图片模型"),
                    crate::app::i18n::I18n::new(app.desktop_locale).__("选择用于文字识别和视觉摘要的多模态大模型；只显示已启用且明确支持图片输入的 provider/model。"),
                    image_model_control(app),
                ),
            ]
            .spacing(0),
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    content.into()
}
#[cfg(test)]
#[path = "system_settings_multimodal_tests.rs"]
mod system_settings_multimodal_tests;
