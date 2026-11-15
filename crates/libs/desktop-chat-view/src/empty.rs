//! 渲染聊天面板空状态。
//! 空状态保持为纯 UI 组件，不携带会话业务逻辑。

use iced::widget::svg;
use iced::widget::{Image, Space, column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use desktop_assets::assets::{self, Icon};
use crate::presentation::{bold_font, icon_svg, is_dark_theme};
fn muted_icon_color(theme: &Theme) -> Color {
    if is_dark_theme(theme) { theme.palette().text.scale_alpha(0.75) }
    else { theme.extended_palette().secondary.base.text.scale_alpha(0.85) }
}
pub struct EmptySessionInput {
    pub greeting: Option<String>,
    pub title: String,
    pub path: String,
    pub git: Option<(String, String)>,
}

pub fn empty_session_title(work_mode: bool) -> &'static str {
    match work_mode {
        false => "识矩软件智能体",
        true => "您的专属 AI 工作助理",
    }
}

#[cfg(test)]
fn login_greeting(name: &str, hour: u32) -> String {
    let greeting = if hour < 12 { "上午好" } else { "下午好" };
    format!("{greeting}，{name}")
}

fn placeholder_chip_style(theme: &Theme) -> iced::widget::container::Style {
    let is_dark = is_dark_theme(theme);
    iced::widget::container::Style {
        background: Some(Background::Color(if is_dark {
            Color::from_rgba8(24, 25, 29, 0.92)
        } else {
            Color::from_rgba8(247, 248, 250, 1.0)
        })),
        border: Border {
            width: 1.0,
            color: if is_dark {
                Color::from_rgba8(45, 48, 54, 0.92)
            } else {
                Color::from_rgba8(226, 231, 237, 1.0)
            },
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

/// 执行 session_loading_placeholder 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn session_loading_placeholder<'a, Message: 'a>(label: &'a str) -> Element<'a, Message> {
    let skeleton_line = |width: f32, height: f32, emphasized: bool| {
        container(Space::new().width(Length::Fixed(width)).height(Length::Fixed(height))).style(
            move |theme: &Theme| {
                let is_dark = is_dark_theme(theme);
                iced::widget::container::Style {
                    background: Some(Background::Color(if is_dark {
                        if emphasized {
                            Color::from_rgba8(255, 255, 255, 0.12)
                        } else {
                            Color::from_rgba8(255, 255, 255, 0.08)
                        }
                    } else if emphasized {
                        Color::from_rgba8(15, 23, 42, 0.10)
                    } else {
                        Color::from_rgba8(15, 23, 42, 0.06)
                    })),
                    border: Border {
                        width: 0.0,
                        color: Color::TRANSPARENT,
                        radius: if height > 12.0 { 16.0.into() } else { 999.0.into() },
                    },
                    ..Default::default()
                }
            },
        )
    };

    let logo = container(
        Image::new(assets::get_image(Icon::Logo))
            .width(Length::Fixed(56.0))
            .height(Length::Fixed(56.0)),
    )
    .width(Length::Fixed(56.0))
    .height(Length::Fixed(56.0));

    let lead_message = container(
        column![
            skeleton_line(118.0, 10.0, true),
            skeleton_line(282.0, 10.0, false),
            skeleton_line(236.0, 10.0, false),
        ]
        .spacing(8),
    )
    .padding([14, 16])
    .width(Length::Fixed(360.0))
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        iced::widget::container::Style {
            background: Some(Background::Color(if is_dark_theme(theme) {
                Color::from_rgba8(255, 255, 255, 0.04)
            } else {
                palette.background.weak.color.scale_alpha(0.48)
            })),
            border: Border {
                width: 1.0,
                color: palette.background.strong.color.scale_alpha(0.14),
                radius: 18.0.into(),
            },
            ..Default::default()
        }
    });

    let reply_message = container(
        column![
            skeleton_line(104.0, 10.0, true),
            skeleton_line(224.0, 10.0, false),
            skeleton_line(188.0, 10.0, false),
        ]
        .spacing(8),
    )
    .padding([14, 16])
    .width(Length::Fixed(304.0))
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        iced::widget::container::Style {
            background: Some(Background::Color(if is_dark_theme(theme) {
                palette.primary.base.color.scale_alpha(0.14)
            } else {
                palette.primary.weak.color.scale_alpha(0.52)
            })),
            border: Border {
                width: 1.0,
                color: palette.primary.strong.color.scale_alpha(0.16),
                radius: 18.0.into(),
            },
            ..Default::default()
        }
    });

    let timeline_block = column![
        row![
            skeleton_line(88.0, 8.0, false),
            Space::new().width(Length::Fill),
            skeleton_line(62.0, 8.0, false),
        ]
        .align_y(Alignment::Center),
        lead_message,
        row![Space::new().width(Length::Fill), reply_message].align_y(Alignment::Center),
        container(
            column![
                skeleton_line(134.0, 10.0, true),
                skeleton_line(372.0, 10.0, false),
                skeleton_line(326.0, 10.0, false),
                skeleton_line(214.0, 10.0, false),
            ]
            .spacing(8),
        )
        .padding([16, 18])
        .width(Length::Fixed(440.0))
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(if is_dark_theme(theme) {
                    Color::from_rgba8(255, 255, 255, 0.03)
                } else {
                    palette.background.base.color.scale_alpha(0.64)
                })),
                border: Border {
                    width: 1.0,
                    color: palette.background.strong.color.scale_alpha(0.12),
                    radius: 18.0.into(),
                },
                ..Default::default()
            }
        }),
    ]
    .spacing(12)
    .width(Length::Fill);

    container(
        container(
            column![
                row![
                    logo,
                    text(label)
                        .size(20)
                        .font(bold_font()),
                ]
                .spacing(14)
                .align_y(Alignment::Center),
                timeline_block,
            ]
            .spacing(16),
        )
        .width(Length::Fill)
        .max_width(660.0),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .padding(20)
    .into()
}

/// 执行 empty_session_placeholder 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn empty_session_placeholder<'a, Message: 'a>(input: EmptySessionInput) -> Element<'a, Message> {
    let logo = container(
        Image::new(assets::get_image(Icon::Logo))
            .width(Length::Fixed(96.0))
            .height(Length::Fixed(96.0)),
    )
    .width(Length::Fixed(96.0))
    .height(Length::Fixed(96.0));

    let path_text = input.path;
    let mut content = column![logo].spacing(12).align_x(iced::alignment::Horizontal::Center);
    if let Some(greeting) = input.greeting {
        content = content.push(
            text(greeting)
            .size(26)
            .font(bold_font()),
        );
    }
    content = content
        .push(
            text(input.title)
            .size(26)
            .font(bold_font()),
        )
        .push(text(path_text).size(14).style(|theme: &Theme| iced::widget::text::Style {
            color: Some(theme.palette().text.scale_alpha(0.72)),
        }));

    if let Some((branch_text, modified_text)) = input.git {
        let branch_row = container(
            row![
                icon_svg(Icon::GitBranch).style(|theme: &Theme, _status| svg::Style {
                    color: Some(muted_icon_color(theme))
                }),
                text(branch_text).size(15),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .style(placeholder_chip_style);
        let modified_row = container(
            row![
                icon_svg(Icon::Clock).style(|theme: &Theme, _status| svg::Style {
                    color: Some(muted_icon_color(theme))
                }),
                text(modified_text).size(15),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .style(placeholder_chip_style);

        content = content.push(
            column![branch_row, modified_row]
                .spacing(10)
                .align_x(iced::alignment::Horizontal::Center),
        );
    }

    container(container(content).padding([30, 34]))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .center_y(Length::Fill)
        .padding(28)
        .into()
}
#[cfg(test)]
#[path = "empty_tests.rs"]
mod empty_tests;
