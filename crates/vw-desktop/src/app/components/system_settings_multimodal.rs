//! 系统设置中 multimodal 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::number_input::NumberInput;
use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, settings_checkbox_style, settings_muted_text_style, settings_page_intro,
    settings_panel, settings_pick_list_menu_style, settings_pick_list_style, settings_section_card,
    settings_value_badge,
};
use crate::app::message::settings::{MultimodalMessage, SettingsMessage};
use crate::app::{App, Message};
use iced::widget::{checkbox, column, container, pick_list, row, text};
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
        checkbox(checked).label(checkbox_label).on_toggle(on_toggle).style(settings_checkbox_style),
    )
}

fn image_model_options(app: &App) -> Vec<String> {
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
                        && model.detail["capabilities"]["input"]["image"].as_bool() == Some(true)
                })
                .map(|model| format!("{}/{}", provider.id, model.id))
        })
        .collect()
}

fn image_model_control(app: &App) -> Element<'_, Message> {
    let options = image_model_options(app);
    let configured = app.multimodal_settings.ocr_model.trim();
    let selected = options.iter().find(|model| model.as_str() == configured).cloned();
    let has_options = !options.is_empty();
    let configured_is_available = selected.is_some();

    let picker = pick_list(options, selected, |value| {
        Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::OcrModelChanged(value)))
    })
    .placeholder("请选择支持图片输入的模型")
    .padding([10, 14])
    .text_size(13)
    .style(settings_pick_list_style)
    .menu_style(settings_pick_list_menu_style)
    .width(Length::Fixed(320.0));

    let mut status = column![picker].spacing(6);
    if app.model_settings.loading {
        status = status.push(text("正在刷新模型目录…").size(11).style(settings_muted_text_style));
    }
    if let Some(error) = app.model_settings.save_error.as_deref() {
        status = status.push(
            text(format!("模型目录刷新失败：{error}")).size(11).style(settings_muted_text_style),
        );
    }
    if !configured.is_empty() && !has_options {
        status = status.push(
            text(format!("当前配置：{configured}（目录中暂无可用匹配项）"))
                .size(11)
                .style(settings_muted_text_style),
        );
    } else if !configured.is_empty() && !configured_is_available {
        status = status.push(
            text(format!("当前配置：{configured}（模型未启用或不支持图片输入）"))
                .size(11)
                .style(settings_muted_text_style),
        );
    } else if !has_options {
        status = status.push(
            text("暂无已启用且明确支持图片输入的模型。").size(11).style(settings_muted_text_style),
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
        settings_page_intro(
            "多模态配置",
            "配置本地图片附件的数量与大小上限，并决定是否允许抓取远程图片 URL。",
        ),
        settings_section_card(
            "图像输入限制",
            "输入框附件优先使用本地文件；这里控制单次请求最多允许的图片数量、每张图片大小上限，以及是否允许抓取远程图片 URL。",
        ),
        settings_panel(
            column![
                number_row("最大图片数量", "单次请求最多允许上传的图片数。", s.max_images, 1, 16, "张", |value| {
                    Message::Settings(SettingsMessage::Multimodal(MultimodalMessage::MaxImagesChanged(
                        value,
                    )))
                }),
                number_row("单张图片大小上限", "每张图片允许的最大体积。", s.max_image_size_mb, 1, 20, "MB", |value| {
                    Message::Settings(SettingsMessage::Multimodal(
                        MultimodalMessage::MaxImageSizeMbChanged(value),
                    ))
                }),
                bool_row(
                    "允许远程抓取",
                    "本地附件始终可用；开启后才允许抓取 http/https 远程图片 URL。",
                    s.allow_remote_fetch,
                    "允许抓取 http/https 远程图片 URL",
                    |value| {
                        Message::Settings(SettingsMessage::Multimodal(
                            MultimodalMessage::AllowRemoteFetchToggled(value),
                        ))
                    },
                ),
            ]
            .spacing(0),
        ),
        settings_section_card(
            "图片文字提取（OCR）",
            "使用独立图片模型把附件转换为受限文本，再交给主模型处理。提取结果是不可信附件数据，且不等于完整视觉理解。",
        ),
        settings_panel(
            column![
                bool_row(
                    "启用 OCR",
                    "启用前必须选择一个可用的图片输入模型。",
                    s.ocr_enabled,
                    "对图片附件执行文字提取",
                    |value| {
                        Message::Settings(SettingsMessage::Multimodal(
                            MultimodalMessage::OcrEnabledToggled(value),
                        ))
                    },
                ),
                field_row(
                    "图片模型",
                    "只显示已启用且明确支持图片输入的 provider/model。",
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
