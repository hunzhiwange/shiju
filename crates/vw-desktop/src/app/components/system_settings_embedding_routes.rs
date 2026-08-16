//! 系统设置中 embedding routes 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    SETTINGS_LABEL_WIDTH, danger_action_btn_style, primary_action_btn_style,
    rounded_action_btn_style, settings_error_banner, settings_muted_text_style, settings_panel,
    settings_panel_style, settings_section_card, settings_success_banner,
    settings_text_input_style,
};
use crate::app::message::settings::EmbeddingRoutesMessage;
use crate::app::{App, Message, message};
use iced::widget::{button, column, container, row, text, text_input};
use iced::{Alignment, Element, Length};

fn field_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            text_input(placeholder, value)
                .on_input(on_input)
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn secure_field_row<'a>(
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    container(
        row![
            column![
                text(label).size(13),
                text(description).size(11).style(settings_muted_text_style),
            ]
            .spacing(4)
            .width(Length::Fixed(SETTINGS_LABEL_WIDTH)),
            text_input(placeholder, value)
                .secure(true)
                .on_input(on_input)
                .padding([10, 12])
                .size(13)
                .style(settings_text_input_style)
                .width(Length::Fill),
        ]
        .spacing(22)
        .align_y(Alignment::Center),
    )
    .padding([14, 0])
    .width(Length::Fill)
    .into()
}

fn template_button<'a>(
    label: &'static str,
    description: &'static str,
    message: EmbeddingRoutesMessage,
) -> Element<'a, Message> {
    button(
        column![text(label).size(13), text(description).size(11).style(settings_muted_text_style),]
            .spacing(4),
    )
    .on_press(Message::Settings(message::SettingsMessage::EmbeddingRoutes(message)))
    .padding([10, 12])
    .style(rounded_action_btn_style)
    .into()
}

/// 构建记忆配置中嵌入路由页签的界面片段与交互数据。
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
pub fn embedded_view(app: &App) -> Element<'_, Message> {
    let s = &app.embedding_routes_settings;

    let header = row![
        container(text("按场景配置嵌入提供商、模型、维度和 API Key；在“嵌入与检索”中用 hint:模式名 引用。").style(settings_muted_text_style))
            .width(Length::Fill),
        button(text("添加路由"))
            .on_press(Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                EmbeddingRoutesMessage::AddRoute,
            )))
            .padding([8, 12])
            .style(primary_action_btn_style),
    ]
    .spacing(8)
    .align_y(Alignment::Start);

    let templates_panel = settings_panel(
        column![
            text("快捷模板").size(13),
            text("一键插入常用配置，插入后仍可按需改 provider、模型、维度和 API Key。")
                .size(11)
                .style(settings_muted_text_style),
            row![
                template_button(
                    "阿里 DashScope",
                    "alibaba-cn / text-embedding-v4 / 1024",
                    EmbeddingRoutesMessage::InsertAlibabaTemplate,
                ),
                template_button(
                    "豆包 Vision 250615",
                    "custom:Volcengine Ark / doubao-embedding-vision-250615",
                    EmbeddingRoutesMessage::InsertDoubaoTemplate,
                ),
                template_button(
                    "识矩",
                    "custom:router.shiju.cloud/v1 / text-embedding-v4 / 1024",
                    EmbeddingRoutesMessage::InsertShijuTemplate,
                ),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(10),
    );

    let mut content = column![
        header,
        templates_panel,
        settings_section_card(
            "路由规则",
            "每条路由定义一个匹配模式，并映射到具体的嵌入 provider/model。"
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    if s.save_error.is_none() && s.save_success {
        content = content.push(settings_success_banner("保存成功"));
    }
    if let Some(error) = s.save_error.as_deref() {
        content = content.push(settings_error_banner(error));
    }

    if s.routes.is_empty() {
        content = content.push(settings_panel(
            column![
                text("暂无嵌入路由").size(14),
                text("点击上方模板按钮或右上角「添加路由」创建第一条规则。")
                    .size(12)
                    .style(settings_muted_text_style)
            ]
            .spacing(6),
        ));
    }

    for (index, route) in s.routes.iter().enumerate() {
        let pattern_row = field_row(
            "匹配模式",
            "用于匹配业务或任务场景。",
            "semantic",
            &route.pattern,
            move |value| {
                Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::PatternChanged(index, value),
                ))
            },
        );
        let provider_row = field_row(
            "提供商",
            "嵌入请求所使用的 provider。",
            "alibaba-cn",
            &route.provider,
            move |value| {
                Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::ProviderChanged(index, value),
                ))
            },
        );
        let model_row = field_row(
            "模型",
            "具体使用的 embedding 模型。",
            "text-embedding-v4",
            &route.model,
            move |value| {
                Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::ModelChanged(index, value),
                ))
            },
        );
        let dimensions_row = field_row(
            "维度",
            "留空时按模型默认维度运行。",
            "1024",
            &route.dimensions,
            move |value| {
                Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::DimensionsChanged(index, value),
                ))
            },
        );
        let api_key_row = secure_field_row(
            "API Key",
            "可选覆盖凭据；阿里填 DashScope API Key。",
            "DASHSCOPE_API_KEY",
            &route.api_key_input,
            move |value| {
                Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::ApiKeyChanged(index, value),
                ))
            },
        );

        let route_title = row![
            text(format!("路由 {}", index + 1)).size(14),
            text("匹配模式 -> 提供商/模型").size(12).style(|t: &iced::Theme| {
                iced::widget::text::Style { color: Some(t.palette().text.scale_alpha(0.65)) }
            }),
            container(text(" ")).width(Length::Fill),
            button(text("删除"))
                .on_press(Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::RemoveRoute(index),
                )))
                .padding([6, 10])
                .style(danger_action_btn_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let card = container(
            column![
                route_title,
                settings_panel(
                    column![pattern_row, provider_row, model_row, dimensions_row, api_key_row]
                        .spacing(0)
                )
            ]
            .spacing(12),
        )
        .padding(14)
        .width(Length::Fill)
        .style(settings_panel_style);

        content = content.push(card);
    }

    content = content.push(
        row![
            container(text(" ")).width(Length::Fill),
            button(text("保存"))
                .on_press(Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::Save,
                )))
                .padding([8, 12])
                .style(rounded_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    container(content).width(Length::Fill).into()
}
#[cfg(test)]
#[path = "system_settings_embedding_routes_tests.rs"]
mod system_settings_embedding_routes_tests;
