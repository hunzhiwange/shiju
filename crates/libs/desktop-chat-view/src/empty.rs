//! 渲染聊天面板空状态。
//! 空状态保持为纯 UI 组件，不携带会话业务逻辑。

use iced::widget::svg;
use iced::widget::{Image, Space, column, container, row, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme};

use desktop_assets::assets::{self, Icon};
use crate::presentation::{bold_font, icon_svg, is_dark_theme};
pub struct EmptySessionInput {
    pub greeting: Option<String>,
    pub title: String,
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

/// 执行 session_loading_placeholder 对应的模块功能。
///
/// 参数含义保持与函数签名一致；返回值用于调用方继续组合处理，错误由返回类型显式表达。
pub fn session_loading_placeholder<'a, Message: Clone + 'static>(label: &'a str) -> Element<'a, Message> {
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

/// Keep the welcome and scene selector readable when a split pane is resized.
pub fn empty_session_placeholder<'a, Message: Clone + 'static>(
    input: EmptySessionInput,
    scene_selector: impl Fn(bool) -> Element<'a, Message> + 'a,
) -> Element<'a, Message> {
    iced::widget::responsive(move |available| {
        let compact = available.width < 510.0 || available.height < 208.0;
        let mut content = column![].spacing(if compact { 6 } else { 10 }).align_x(Alignment::Center);
        if available.height >= 150.0 {
            let logo_size = if compact { 36 } else { 52 };
            content = content.push(
                Image::new(assets::get_image(Icon::Logo)).width(logo_size).height(logo_size),
            );
        }

        if let Some(greeting) = &input.greeting {
            content = content.push(
                text(greeting.clone())
                    .size(if compact { 13 } else { 16 })
                    .align_x(Alignment::Center)
                    .style(context_text),
            );
        }
        content = content
            .push(
                text(input.title.clone())
                    .size(if compact { 29 } else { 38 })
                    .font(bold_font())
                    .wrapping(text::Wrapping::WordOrGlyph)
                    .align_x(Alignment::Center),
            )
            .push(scene_selector(available.width < 310.0));

        if let Some((branch, modified)) = &input.git {
            let detail = |icon, label: String| {
                row![
                    icon_svg(icon).style(|theme: &Theme, _| svg::Style { color: context_text(theme).color }),
                    text(label).size(12).wrapping(text::Wrapping::WordOrGlyph).style(context_text),
                ]
                .spacing(6)
                .align_y(Alignment::Center)
            };
            content = content.push(
                row![detail(Icon::GitBranch, branch.clone()), detail(Icon::Clock, modified.clone())]
                    .spacing(if compact { 10 } else { 16 })
                    .wrap()
                    .vertical_spacing(5)
                    .align_x(iced::alignment::Horizontal::Center),
            );
        }

        container(content)
            .center_x(Length::Fill)
            .center_y(Length::Fill)
            .padding([8, 10])
            .into()
    })
    .into()
}

fn context_text(theme: &Theme) -> text::Style {
    text::Style { color: Some(theme.palette().text.scale_alpha(0.72)) }
}

#[cfg(test)]
#[path = "empty_tests.rs"]
mod empty_tests;

impl EmptySessionInput {
    pub fn from_context(
        i18n: desktop_i18n::I18n,
        name: Option<&str>,
        hour: u32,
        work_mode: bool,
        git: Option<(String, String)>,
    ) -> Self {
        Self {
            greeting: name.map(|name| {
                i18n.format(if hour < 12 { "上午好，%s" } else { "下午好，%s" }, &[name.into()])
                    .expect("valid greeting template")
            }),
            title: i18n.__(empty_session_title(work_mode)).to_owned(),
            git: git.map(|(branch, modified)| {
                (i18n.format("分支 %s", &[branch.into()]).expect("valid branch template"), modified)
            }),
        }
    }
}
