//! 模态框模块
//!
//! 本模块提供应用程序中使用的各种模态框UI组件，包括：
//! - 编辑网址书签的模态框
//! - 添加网址书签的模态框
//!
//! 模态框采用统一的外壳样式，包含圆角边框、阴影效果和主题适配的背景色。

use super::ui;
use crate::app::assets::Icon;
use crate::app::components::system_settings_common::{
    settings_muted_text_style, settings_panel_style, settings_section_card,
};
use crate::app::message::ViewMessage;
use crate::app::{App, Message};
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{Space, button, column, container, row, text, text_input};
use iced::{Color, Element, Length, Theme};

const WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE: &str = r#"[
  {
    "name": "session_id",
    "domain": "example.com",
    "days": 365,
    "url_filter": "https://example.com/"
  }
]"#;

fn can_add_web_bookmark(title: &str, url: &str) -> bool {
    !title.trim().is_empty() && !url.trim().is_empty()
}

/// 创建模态框的外壳容器
///
/// 为模态框内容提供统一的视觉样式，包括：
/// - 内边距（18px）
/// - 圆角边框（16px圆角）
/// - 阴影效果（模糊半径30px，向下偏移10px）
/// - 主题适配的背景色
///
/// # 参数
/// - `content`: 模态框的内容元素
///
/// # 返回
/// 包装后的容器元素，可直接用于渲染
fn modal_shell<'a>(content: Element<'a, Message>) -> Element<'a, Message> {
    container(content)
        .padding([22, 24])
        .style(|theme: &Theme| {
            let mut style = settings_panel_style(theme);
            style.border.radius = 24.0.into();
            style.shadow = iced::Shadow {
                color: Color::BLACK.scale_alpha(0.22),
                offset: iced::Vector::new(0.0, 18.0),
                blur_radius: 34.0,
            };
            style
        })
        .into()
}

/// 创建编辑网址书签的模态框
///
/// 该模态框允许用户编辑现有的网址书签，包括：
/// - 标题：书签显示名称
/// - 网址：书签URL
/// - 宽度/高度：窗口尺寸（可选）
/// - Cookie配置：以JSON格式配置需要注入的Cookie
///
/// # 参数
/// - `app`: 应用状态引用，包含编辑中的书签数据
///
/// # 返回
/// - `Some((Element, Message))`: 模态框元素和关闭消息的元组
/// - `None`: 如果没有正在编辑的书签
fn edit_web_bookmark_modal<'a>(app: &'a App) -> Option<(Element<'a, Message>, Message)> {
    // 关闭模态框的消息
    let close = Message::View(ViewMessage::WebBookmarkEditCancel);

    // 标题输入框
    let title_input = text_input("", &app.edit_web_bookmark_title_input)
        .on_input(|s| Message::View(ViewMessage::WebBookmarkEditTitleChanged(s)))
        .width(Length::Fill)
        .padding([8, 10])
        .size(13)
        .style(ui::figma_text_input_style);
    // URL输入框
    let url_input = text_input("", &app.edit_web_bookmark_url_input)
        .on_input(|s| Message::View(ViewMessage::WebBookmarkEditUrlChanged(s)))
        .width(Length::Fill)
        .padding([8, 10])
        .size(13)
        .style(ui::figma_text_input_style);
    // 宽度输入框（可选）
    let width_input = text_input("", &app.edit_web_bookmark_width_input)
        .on_input(|s| Message::View(ViewMessage::WebBookmarkEditWidthChanged(s)))
        .width(Length::Fill)
        .padding([8, 10])
        .size(13)
        .style(ui::figma_text_input_style);
    // 高度输入框（可选）
    let height_input = text_input("", &app.edit_web_bookmark_height_input)
        .on_input(|s| Message::View(ViewMessage::WebBookmarkEditHeightChanged(s)))
        .width(Length::Fill)
        .padding([8, 10])
        .size(13)
        .style(ui::figma_text_input_style);

    // Cookie配置编辑器（多行文本编辑器）
    let cookie_configs_editor =
        iced::widget::text_editor(&app.edit_web_bookmark_cookie_configs_editor)
            .on_action(|action| {
                Message::View(ViewMessage::WebBookmarkEditCookieConfigsChanged(action))
            })
            .height(Length::Fixed(120.0))
            .style(ui::figma_text_editor_style);

    // 获取正在编辑的书签索引
    let idx = app.editing_web_bookmark.unwrap_or(0);

    // 顶部标题栏：标题 + 关闭按钮
    let top_bar = row![
        text("编辑网址").size(16),
        Space::new().width(Length::Fill),
        button(ui::icon_svg(Icon::X, 14.0).style(|theme: &Theme, _| iced::widget::svg::Style {
            color: Some(theme.palette().text),
        }))
        .on_press(close.clone())
        .style(ui::icon_button_style)
        .padding(6)
    ]
    .align_y(iced::Alignment::Center);

    // Cookie 配置帮助气泡：展示示例 JSON
    let cookie_tooltip = {
        // 构建提示气泡内容：标题 + 示例文本
        ui::tooltip_bubble_el(
            column![text("配置示例").size(12), text(WEB_BOOKMARK_COOKIE_CONFIG_EXAMPLE).size(11),]
                .spacing(6)
                .into(),
        )
    };

    // 构建模态框主体内容
    let content = column![
        top_bar,
        settings_section_card("书签信息", "设置标题、网址以及独立窗口的默认尺寸。"),
        column![text("标题").size(12).style(settings_muted_text_style), title_input].spacing(6),
        column![text("网址").size(12).style(settings_muted_text_style), url_input].spacing(6),
        row![
            column![text("宽度（可选）").size(12).style(settings_muted_text_style), width_input]
                .spacing(6),
            column![text("高度（可选）").size(12).style(settings_muted_text_style), height_input]
                .spacing(6),
        ]
        .spacing(10),
        settings_section_card("Cookie 配置", "支持按 JSON 为指定站点注入 Cookie。"),
        column![
            row![
                text("Cookie配置(JSON)").size(12).style(settings_muted_text_style),
                Tooltip::new(
                    button(text("?").size(12)).style(ui::cool_icon_button_style).padding([4, 8]),
                    cookie_tooltip,
                    TooltipPosition::Top,
                )
                .gap(8.0),
                Tooltip::new(
                    button(text("插入例子").size(12))
                        .on_press(Message::View(
                            ViewMessage::WebBookmarkEditCookieConfigsInsertExample,
                        ))
                        .style(ui::primary_button_style)
                        .padding([4, 10]),
                    ui::tooltip_bubble("插入预设 Cookie 配置示例"),
                    TooltipPosition::Top,
                )
                .gap(8.0),
            ]
            .align_y(iced::Alignment::Center)
            .spacing(8),
            cookie_configs_editor
        ]
        .spacing(6),
        // 底部操作按钮栏：取消、删除、保存
        row![
            // 取消按钮
            Tooltip::new(
                button(ui::icon_svg(ui::action_icon("取消"), 14.0).style(move |_t: &Theme, _| {
                    iced::widget::svg::Style { color: Some(Color::WHITE) }
                },))
                .on_press(close.clone())
                .style(ui::cool_icon_button_style)
                .padding([8, 12]),
                ui::tooltip_bubble("取消"),
                TooltipPosition::Top,
            )
            .gap(8.0),
            Space::new().width(Length::Fill),
            // 删除按钮
            Tooltip::new(
                button(ui::icon_svg(ui::action_icon("删除"), 14.0).style(move |_t: &Theme, _| {
                    iced::widget::svg::Style { color: Some(Color::WHITE) }
                },))
                .on_press(Message::View(ViewMessage::WebBookmarkRemove(idx)))
                .style(ui::cool_icon_button_style)
                .padding([8, 12]),
                ui::tooltip_bubble("删除"),
                TooltipPosition::Top,
            )
            .gap(8.0),
            // 保存按钮
            Tooltip::new(
                button(ui::icon_svg(ui::action_icon("保存"), 14.0).style(move |_t: &Theme, _| {
                    iced::widget::svg::Style { color: Some(Color::WHITE) }
                },))
                .on_press(Message::View(ViewMessage::WebBookmarkEditSave))
                .style(ui::primary_button_style)
                .padding([8, 14]),
                ui::tooltip_bubble("保存"),
                TooltipPosition::Top,
            )
            .gap(8.0),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(10),
    ]
    .spacing(14)
    .width(Length::Fixed(560.0));

    // 返回包装在模态框外壳中的内容元素
    Some((modal_shell(content.into()), close))
}

/// 创建添加网址书签的模态框
///
/// 该模态框允许用户添加新的网址书签，包括：
/// - 标题：书签显示名称（必填）
/// - 网址：书签URL（必填）
/// - 宽度/高度：窗口尺寸（可选）
///
/// 只有当标题和网址都不为空时，添加按钮才会被激活。
///
/// # 参数
/// - `app`: 应用状态引用，包含输入中的书签数据
///
/// # 返回
/// - `Some((Element, Message))`: 模态框元素和关闭消息的元组
/// - `None`: 理论上不会返回（该模态框显示时总是返回 Some）
fn add_web_bookmark_modal<'a>(app: &'a App) -> Option<(Element<'a, Message>, Message)> {
    // 关闭模态框的消息（切换网址链接菜单）
    let close = Message::View(ViewMessage::ToggleWebLinksMenu);

    // 标题输入框
    let title_input = text_input("", &app.web_bookmark_title_input)
        .on_input(|s| Message::View(ViewMessage::WebBookmarkTitleChanged(s)))
        .width(Length::Fill)
        .padding([8, 10])
        .size(13)
        .style(ui::figma_text_input_style);
    // URL输入框
    let url_input = text_input("", &app.web_bookmark_url_input)
        .on_input(|s| Message::View(ViewMessage::WebBookmarkUrlChanged(s)))
        .width(Length::Fill)
        .padding([8, 10])
        .size(13)
        .style(ui::figma_text_input_style);
    // 宽度输入框（可选）
    let width_input = text_input("", &app.web_bookmark_width_input)
        .on_input(|s| Message::View(ViewMessage::WebBookmarkWidthChanged(s)))
        .width(Length::Fill)
        .padding([8, 10])
        .size(13)
        .style(ui::figma_text_input_style);
    // 高度输入框（可选）
    let height_input = text_input("", &app.web_bookmark_height_input)
        .on_input(|s| Message::View(ViewMessage::WebBookmarkHeightChanged(s)))
        .width(Length::Fill)
        .padding([8, 10])
        .size(13)
        .style(ui::figma_text_input_style);

    // 顶部标题栏：标题 + 关闭按钮
    let top_bar = row![
        text("添加网址").size(16),
        Space::new().width(Length::Fill),
        button(ui::icon_svg(Icon::X, 14.0).style(|theme: &Theme, _| iced::widget::svg::Style {
            color: Some(theme.palette().text),
        }))
        .on_press(close.clone())
        .style(ui::icon_button_style)
        .padding(6)
    ]
    .align_y(iced::Alignment::Center);

    let content = column![
        top_bar,
        settings_section_card("新增网址书签", "保存一个可在应用中心快速打开的网页入口。"),
        column![text("标题").size(12).style(settings_muted_text_style), title_input].spacing(6),
        column![text("网址").size(12).style(settings_muted_text_style), url_input].spacing(6),
        row![
            column![text("宽度（可选）").size(12).style(settings_muted_text_style), width_input]
                .spacing(6),
            column![text("高度（可选）").size(12).style(settings_muted_text_style), height_input]
                .spacing(6),
        ]
        .spacing(10),
        // 底部操作按钮栏：取消、添加
        row![
            // 取消按钮
            Tooltip::new(
                button(ui::icon_svg(ui::action_icon("取消"), 14.0).style(move |_t: &Theme, _| {
                    iced::widget::svg::Style { color: Some(Color::WHITE) }
                },))
                .on_press(close.clone())
                .style(ui::cool_icon_button_style)
                .padding([8, 12]),
                ui::tooltip_bubble("取消"),
                TooltipPosition::Top,
            )
            .gap(8.0),
            Space::new().width(Length::Fill),
            // 添加按钮（仅当标题和URL都不为空时激活）
            Tooltip::new(
                {
                    // 检查是否可以添加：标题和URL都非空
                    let can_add = can_add_web_bookmark(
                        &app.web_bookmark_title_input,
                        &app.web_bookmark_url_input,
                    );
                    // 根据是否可添加决定图标颜色
                    let icon_color =
                        if can_add { Color::WHITE } else { Color::WHITE.scale_alpha(0.65) };
                    let mut btn = button(ui::icon_svg(ui::action_icon("添加"), 14.0).style(
                        move |_t: &Theme, _| iced::widget::svg::Style { color: Some(icon_color) },
                    ))
                    .padding([8, 14]);
                    // 仅当可以添加时设置点击事件和主要按钮样式
                    if can_add {
                        btn = btn
                            .on_press(Message::View(ViewMessage::WebBookmarkAddSave))
                            .style(ui::primary_button_style);
                    } else {
                        btn = btn.style(ui::cool_icon_button_style);
                    }
                    btn
                },
                ui::tooltip_bubble("添加"),
                TooltipPosition::Top,
            )
            .gap(8.0),
        ]
        .align_y(iced::Alignment::Center)
        .spacing(10),
    ]
    .spacing(14)
    .width(Length::Fixed(560.0));

    // 返回包装在模态框外壳中的内容元素
    Some((modal_shell(content.into()), close))
}

/// 根据应用状态返回当前活动的模态框
///
/// 该函数根据应用的状态决定显示哪个模态框：
/// - 如果 `editing_web_bookmark` 为 `Some`，显示编辑书签模态框
/// - 如果 `show_web_links_menu` 为 `true`，显示添加书签模态框
/// - 否则不显示任何模态框
///
/// # 参数
/// - `app`: 应用状态引用
///
/// # 返回
/// - `Some((Element, Message))`: 当前活动的模态框元素和关闭消息
/// - `None`: 没有活动的模态框
pub(super) fn active_modal<'a>(app: &'a App) -> Option<(Element<'a, Message>, Message)> {
    // 优先检查是否有正在编辑的书签
    if app.editing_web_bookmark.is_some() {
        edit_web_bookmark_modal(app)
    } else if app.show_web_links_menu {
        // 否则检查是否显示添加书签菜单
        add_web_bookmark_modal(app)
    } else {
        // 无活动模态框
        None
    }
}

#[cfg(test)]
#[path = "modals_tests.rs"]
mod modals_tests;
