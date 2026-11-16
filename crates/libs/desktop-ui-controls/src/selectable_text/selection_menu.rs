use super::is_dark_theme;
use iced::{Element, Length, Theme, Color, Background, Border};
use iced::widget::{button, container, column, text};
use crate::text_editor_context_menu::TextContextAction;
pub fn text_context_menu<'a, MenuMessage: Clone + 'a>(
    i18n: desktop_i18n::I18n,
    message: impl Fn(TextContextAction) -> MenuMessage,
) -> Element<'a, MenuMessage> {
    let menu_item = |label: &'static str, action| {
        button(
            container(text(label).size(12))
                .width(Length::Fill)
                .padding([5, 9])
                .align_x(iced::alignment::Horizontal::Left),
        )
        .width(Length::Fill)
        .padding(0)
        .style(chat_context_menu_button_style)
        .on_press(message(action))
    };

    container(
        column![
            menu_item(i18n.__("复制"), TextContextAction::Copy),
            menu_item(i18n.__("添加到对话"), TextContextAction::Append),
            menu_item(i18n.__("百度搜索"), TextContextAction::Baidu),
            menu_item(i18n.__("Google 搜索"), TextContextAction::Google),
            menu_item(i18n.__("Bing 搜索"), TextContextAction::Bing),
        ]
        .spacing(0)
        .width(Length::Fixed(134.0)),
    )
    .padding([3, 3])
    .style(chat_context_menu_style)
    .into()
}

pub fn chat_context_menu_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let hovered_bg = if is_dark_theme(theme) {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.05)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xEA, 0xED, 0xF1, 0.85)
    };
    let pressed_bg = if is_dark_theme(theme) {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.09)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xE2, 0xE6, 0xEB, 0.95)
    };

    iced::widget::button::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: match status {
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::button::Status::Pressed => Some(Background::Color(pressed_bg)),
            // iced 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            iced::widget::button::Status::Hovered => Some(Background::Color(hovered_bg)),
            _ => None,
        },
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 5.0.into() },
        // text_color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        text_color: theme.palette().text,
        ..Default::default()
    }
}

pub fn chat_context_menu_style(theme: &Theme) -> iced::widget::container::Style {
    let background = if is_dark_theme(theme) {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0x22, 0x22, 0x24, 0.98)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xF3, 0xF4, 0xF6, 0.98)
    };
    let border = if is_dark_theme(theme) {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0x3A, 0x3A, 0x3F, 0.95)
    } else {
        // Color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Color::from_rgba8(0xDD, 0xE1, 0xE6, 1.0)
    };

    iced::widget::container::Style {
        // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        background: Some(Background::Color(background)),
        // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        border: Border { width: 1.0, color: border, radius: 8.0.into() },
        // shadow 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shadow: iced::Shadow {
            // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            color: Color::BLACK.scale_alpha(if is_dark_theme(theme) { 0.20 } else { 0.08 }),
            // offset 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            offset: iced::Vector::new(0.0, 4.0),
            // blur_radius 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            blur_radius: 10.0,
        },
        ..Default::default()
    }
}
