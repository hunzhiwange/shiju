//! 系统设置中 embedding routes 配置页面的界面拼装与交互消息转换。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::system_settings_common::{
    danger_action_btn_style, primary_action_btn_style, rounded_action_btn_style,
    settings_error_banner, settings_flexible_row_with_help, settings_group_card,
    settings_group_heading_with_help, settings_hover_help_label, settings_muted_text_style,
    settings_page_heading_with_help, settings_panel, settings_section_content,
    settings_success_banner, settings_text_input,
};
use crate::app::i18n::I18n;
use crate::app::i18n::settings_models::connection_text::text as connection_text;
use crate::app::message::settings::EmbeddingRoutesMessage;
use crate::app::{App, Message, message};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

fn field_row<'a>(
    app: &App,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    settings_flexible_row_with_help(
        I18n::new(app.desktop_locale),
        &app.app_theme,
        label,
        description,
        220.0,
        settings_text_input(placeholder, value).on_input(on_input).width(Length::Fill),
    )
    .into()
}

fn secure_field_row<'a>(
    app: &App,
    label: &'static str,
    description: &'static str,
    placeholder: &'static str,
    value: &'a str,
    on_input: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    settings_flexible_row_with_help(
        I18n::new(app.desktop_locale),
        &app.app_theme,
        label,
        description,
        220.0,
        settings_text_input(placeholder, value).secure(true).on_input(on_input).width(Length::Fill),
    )
    .into()
}

fn template_button<'a>(
    label: &'static str,
    description: &'static str,
    message: EmbeddingRoutesMessage,
) -> Element<'a, Message> {
    button(
        column![
            text(label).size(13),
            text(description)
                .size(12)
                .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
                .style(settings_muted_text_style),
        ]
        .spacing(4),
    )
    .on_press(Message::Settings(message::SettingsMessage::EmbeddingRoutes(message)))
    .padding([10, 12])
    .width(Length::Fill)
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
    let i18n = I18n::new(app.desktop_locale);

    let header = row![
        container(settings_page_heading_with_help(
            i18n,
            &app.app_theme,
            i18n.__("嵌入路由"),
            connection_text(
                app.desktop_locale,
                "按场景配置嵌入提供商、模型、维度和 API Key；在“嵌入与检索”中用 hint:模式名 引用。"
            ),
        ))
        .width(Length::Fill),
        button(text(connection_text(app.desktop_locale, "添加路由")))
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
            settings_group_heading_with_help(
                i18n,
                &app.app_theme,
                connection_text(app.desktop_locale, "快捷模板"),
                connection_text(
                    app.desktop_locale,
                    "一键插入常用配置，插入后仍可按需改 provider、模型、维度和 API Key。"
                ),
            ),
            row![
                template_button(
                    connection_text(app.desktop_locale, "阿里 DashScope"),
                    "alibaba-cn / text-embedding-v4 / 1024",
                    EmbeddingRoutesMessage::InsertAlibabaTemplate,
                ),
                template_button(
                    connection_text(app.desktop_locale, "豆包 Vision 250615"),
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
            .align_y(Alignment::Start),
        ]
        .spacing(12),
    );

    let mut content = column![
        header,
        templates_panel,
        settings_group_heading_with_help(
            i18n,
            &app.app_theme,
            connection_text(app.desktop_locale, "路由规则"),
            connection_text(
                app.desktop_locale,
                "每条路由定义一个匹配模式，并映射到具体的嵌入 provider/model。"
            )
        ),
    ]
    .spacing(16)
    .width(Length::Fill);

    if s.save_error.is_none() && s.save_success {
        content =
            content.push(settings_success_banner(connection_text(app.desktop_locale, "保存成功")));
    }
    if let Some(error) = s.save_error.as_ref() {
        content = content.push(settings_error_banner(error.display(app.desktop_locale)));
    }

    if s.routes.is_empty() {
        content = content.push(settings_panel(
            column![
                text(connection_text(app.desktop_locale, "暂无嵌入路由")).size(14),
                text(connection_text(
                    app.desktop_locale,
                    "点击上方模板按钮或右上角「添加路由」创建第一条规则。"
                ))
                .size(12)
                .style(settings_muted_text_style)
            ]
            .spacing(6),
        ));
    }

    for (index, route) in s.routes.iter().enumerate() {
        let pattern_row = field_row(
            app,
            connection_text(app.desktop_locale, "匹配模式"),
            connection_text(app.desktop_locale, "用于匹配业务或任务场景。"),
            "semantic",
            &route.pattern,
            move |value| {
                Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::PatternChanged(index, value),
                ))
            },
        );
        let provider_row = field_row(
            app,
            connection_text(app.desktop_locale, "提供商"),
            connection_text(app.desktop_locale, "嵌入请求所使用的 provider。"),
            "alibaba-cn",
            &route.provider,
            move |value| {
                Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::ProviderChanged(index, value),
                ))
            },
        );
        let model_row = field_row(
            app,
            connection_text(app.desktop_locale, "模型"),
            connection_text(app.desktop_locale, "具体使用的 embedding 模型。"),
            "text-embedding-v4",
            &route.model,
            move |value| {
                Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::ModelChanged(index, value),
                ))
            },
        );
        let dimensions_row = field_row(
            app,
            connection_text(app.desktop_locale, "维度"),
            connection_text(app.desktop_locale, "留空时按模型默认维度运行。"),
            "1024",
            &route.dimensions,
            move |value| {
                Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::DimensionsChanged(index, value),
                ))
            },
        );
        let api_key_row = secure_field_row(
            app,
            "API Key",
            connection_text(app.desktop_locale, "可选覆盖凭据；阿里填 DashScope API Key。"),
            "DASHSCOPE_API_KEY",
            &route.api_key_input,
            move |value| {
                Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::ApiKeyChanged(index, value),
                ))
            },
        );

        let route_title = row![
            settings_hover_help_label(
                i18n,
                &app.app_theme,
                connection_text(app.desktop_locale, "路由规则"),
                text(crate::app::i18n::settings_models::route_title(app.desktop_locale, index + 1))
                    .size(14),
                connection_text(app.desktop_locale, "匹配模式 -> 提供商/模型"),
            )
            .wrap(),
            container(text(" ")).width(Length::Fill),
            button(text(connection_text(app.desktop_locale, "删除")))
                .on_press(Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::RemoveRoute(index),
                )))
                .padding([6, 10])
                .style(danger_action_btn_style),
        ]
        .spacing(10)
        .align_y(Alignment::Center);

        let card = settings_group_card(
            column![
                route_title,
                settings_section_content(
                    column![pattern_row, provider_row, model_row, dimensions_row, api_key_row]
                        .spacing(0)
                )
            ]
            .spacing(12),
        )
        .padding([10, 22]);

        content = content.push(card);
    }

    content = content.push(
        row![
            container(text(" ")).width(Length::Fill),
            button(text(connection_text(app.desktop_locale, "保存")))
                .on_press(Message::Settings(message::SettingsMessage::EmbeddingRoutes(
                    EmbeddingRoutesMessage::Save,
                )))
                .padding([8, 12])
                .style(primary_action_btn_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    container(content).width(Length::Fill).into()
}
#[cfg(test)]
#[path = "system_settings_embedding_routes_tests.rs"]
mod system_settings_embedding_routes_tests;

#[cfg(test)]
#[path = "system_settings_embedding_routes_ui_refresh_11_tests.rs"]
mod ui_refresh_11_tests;
