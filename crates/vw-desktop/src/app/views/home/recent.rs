//! 首页视图模块，负责项目入口、最近项目和常用工具入口的界面组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::{Image, Space, button, column, container, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Theme, Vector};

use crate::app::assets::{self, Icon};
use crate::app::components::system_settings_common::settings_panel_style;
use crate::app::{App, Message, message};

use super::common::{
    home_cjk_emphasis_font, home_emphasis_font, is_dark_theme, parse_hex_color, primary_button,
    project_accent_color, project_avatar,
};

/// 渲染对应界面。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub(super) fn render_body(app: &App) -> Element<'_, Message> {
    if app.recent_projects.is_empty() {
        render_empty_state().into()
    } else {
        render_recent_projects_body(app)
    }
}

fn render_empty_state<'a>() -> iced::widget::Container<'a, Message> {
    let logo = container(
        Image::new(assets::get_image(Icon::Logo))
            .width(Length::Fixed(180.0))
            .height(Length::Fixed(180.0)),
    )
    .width(Length::Fixed(180.0))
    .height(Length::Fixed(180.0))
    .clip(true)
    .style(|_theme: &Theme| iced::widget::container::Style {
        background: None,
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 90.0.into() },
        ..Default::default()
    });

    container(
        column![
            container(logo).width(Length::Fill).center_x(Length::Fill),
            Space::new().height(Length::Fixed(16.0)),
            text("暂无最近项目").size(16).style(|theme: &Theme| {
                let color = if is_dark_theme(theme) {
                    theme.extended_palette().background.base.text
                } else {
                    theme.palette().text
                };
                text::Style { color: Some(color) }
            }),
            Space::new().height(Length::Fixed(6.0)),
            text("点击右上角\"选择文件夹\"开始").size(12).style(|theme: &Theme| {
                let color = if is_dark_theme(theme) {
                    theme.extended_palette().background.base.text
                } else {
                    theme.extended_palette().secondary.base.text
                };
                text::Style { color: Some(color) }
            }),
        ]
        .align_x(iced::alignment::Horizontal::Center)
        .spacing(0),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .center_x(Length::Fill)
    .center_y(Length::Fill)
    .padding(40)
}

fn render_recent_projects_body(app: &App) -> Element<'_, Message> {
    let hero_logo = container(
        Image::new(assets::get_image(Icon::Logo))
            .width(Length::Fixed(140.0))
            .height(Length::Fixed(140.0)),
    )
    .width(Length::Fixed(140.0))
    .height(Length::Fixed(140.0))
    .clip(true)
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        iced::widget::container::Style {
            background: Some(Background::Color(palette.background.base.color)),
            border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 70.0.into() },
            ..Default::default()
        }
    });

    let hero = container(
        row![
            hero_logo,
            column![
                text("识矩").size(22).font(home_cjk_emphasis_font()).style(|theme: &Theme| {
                    let color = if is_dark_theme(theme) {
                        theme.extended_palette().background.base.text
                    } else {
                        theme.palette().text
                    };
                    text::Style { color: Some(color) }
                }),
                text("选择文件夹开始一个项目，或从下方最近项目继续。").size(12).style(
                    |theme: &Theme| {
                        let color = if is_dark_theme(theme) {
                            theme.extended_palette().background.base.text
                        } else {
                            theme.extended_palette().secondary.base.text
                        };
                        text::Style { color: Some(color) }
                    }
                ),
            ]
            .spacing(4),
            container(Space::new()).width(Length::Fill),
            primary_button(
                "选择文件夹",
                Message::Project(message::ProjectMessage::OpenFolderPressed),
            ),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .padding(16)
    .width(Length::Fill)
    .style(|theme: &Theme| {
        let mut style = settings_panel_style(theme);
        style.border.radius = 22.0.into();
        style.background = Some(Background::Color(if is_dark_theme(theme) {
            theme.extended_palette().background.base.color.scale_alpha(0.94)
        } else {
            Color::WHITE.scale_alpha(0.92)
        }));
        style
    });

    let recent_count = container(text(format!("{} 个项目", app.recent_projects.len())).size(11))
        .padding([5, 10])
        .style(|theme: &Theme| {
            let palette = theme.extended_palette();
            iced::widget::container::Style {
                background: Some(Background::Color(if is_dark_theme(theme) {
                    palette.background.weak.color.scale_alpha(0.48)
                } else {
                    Color::WHITE.scale_alpha(0.78)
                })),
                border: Border {
                    width: 1.0,
                    color: palette.background.strong.color.scale_alpha(0.68),
                    radius: 999.0.into(),
                },
                text_color: Some(theme.palette().text.scale_alpha(0.72)),
                ..Default::default()
            }
        });

    let mut recent_list = column![].spacing(10).width(Length::Fill);
    for (index, path) in app.recent_projects.iter().enumerate() {
        recent_list = recent_list.push(recent_project_row(app, index, path));
    }

    let section = column![
        hero,
        row![
            column![
                text("最近项目").size(15).font(home_cjk_emphasis_font()),
                text("继续最近的工作区入口，并直接处理常用操作。").size(12).style(
                    |theme: &Theme| iced::widget::text::Style {
                        color: Some(theme.palette().text.scale_alpha(0.62)),
                    }
                ),
            ]
            .spacing(4)
            .width(Length::Fill),
            recent_count,
        ]
        .spacing(10)
        .align_y(Alignment::Center),
        recent_list,
    ]
    .spacing(14)
    .width(Length::Fill);

    scrollable(container(section).padding([18, 16]).width(Length::Fill)).height(Length::Fill).into()
}

fn recent_project_row<'a>(app: &App, index: usize, path: &str) -> Element<'a, Message> {
    let name = recent_project_name(app, index, path);
    let project_meta = app.recent_projects_meta.iter().find(|meta| meta.path == path);
    let custom_icon = project_meta.and_then(|meta| meta.icon.as_deref());
    let custom_color =
        project_meta.and_then(|meta| meta.icon_color.as_deref()).and_then(parse_hex_color);
    let accent = custom_color.unwrap_or_else(|| project_accent_color(path));
    let selected = app.project_path.as_ref().is_some_and(|current| current == path);

    let status_badge = container(text(if selected { "当前工作区" } else { "最近项目" }).size(11))
        .padding([4, 10])
        .style(move |theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(if is_dark_theme(theme) {
                accent.scale_alpha(if selected { 0.28 } else { 0.16 })
            } else {
                accent.scale_alpha(if selected { 0.14 } else { 0.10 })
            })),
            border: Border {
                width: 1.0,
                color: accent.scale_alpha(if is_dark_theme(theme) { 0.34 } else { 0.22 }),
                radius: 999.0.into(),
            },
            text_color: Some(if selected { accent } else { accent.scale_alpha(0.92) }),
            ..Default::default()
        });

    let meta = column![
        row![
            text(name.clone()).size(14).font(home_emphasis_font()).style(|theme: &Theme| {
                let color = if is_dark_theme(theme) {
                    theme.extended_palette().background.base.text
                } else {
                    theme.palette().text
                };
                text::Style { color: Some(color) }
            }),
            status_badge,
        ]
        .spacing(8)
        .align_y(Alignment::Center),
        text(path.to_string()).size(11).style(|theme: &Theme| {
            let color = if is_dark_theme(theme) {
                theme.palette().text.scale_alpha(0.68)
            } else {
                theme.extended_palette().secondary.base.text
            };
            text::Style { color: Some(color) }
        }),
    ]
    .spacing(6)
    .width(Length::Fill);

    let open_tag = container(text(if selected { "继续处理" } else { "打开" }).size(11))
        .padding([6, 12])
        .style(move |theme: &Theme| iced::widget::container::Style {
            background: Some(Background::Color(if is_dark_theme(theme) {
                accent.scale_alpha(0.18)
            } else {
                accent.scale_alpha(0.10)
            })),
            border: Border {
                width: 1.0,
                color: accent.scale_alpha(if is_dark_theme(theme) { 0.38 } else { 0.22 }),
                radius: 999.0.into(),
            },
            text_color: Some(accent),
            ..Default::default()
        });

    button(
        container(
            row![
                project_avatar(&name, path, custom_icon, custom_color, 48.0),
                container(meta).width(Length::Fill),
                open_tag,
            ]
            .spacing(12)
            .align_y(Alignment::Center),
        )
        .padding([14, 14]),
    )
    .on_press(Message::Project(message::ProjectMessage::OpenRecentPressed(path.to_string())))
    .width(Length::Fill)
    .style(move |theme: &Theme, status| {
        let palette = theme.extended_palette();
        let hovered = matches!(status, iced::widget::button::Status::Hovered);
        let pressed = matches!(status, iced::widget::button::Status::Pressed);
        let dark = is_dark_theme(theme);

        iced::widget::button::Style {
            background: Some(Background::Color(if dark {
                if pressed {
                    palette.background.weak.color.scale_alpha(0.92)
                } else {
                    palette.background.base.color.scale_alpha(if selected { 0.98 } else { 0.90 })
                }
            } else if pressed {
                Color::WHITE.scale_alpha(0.98)
            } else {
                Color::WHITE.scale_alpha(if selected { 0.98 } else { 0.92 })
            })),
            border: Border {
                width: 1.0,
                color: if selected {
                    accent.scale_alpha(if dark { 0.56 } else { 0.28 })
                } else if hovered {
                    accent.scale_alpha(if dark { 0.32 } else { 0.18 })
                } else {
                    palette.background.strong.color.scale_alpha(if dark { 0.72 } else { 0.36 })
                },
                radius: 18.0.into(),
            },
            shadow: iced::Shadow {
                color: if selected {
                    accent.scale_alpha(if dark { 0.20 } else { 0.10 })
                } else {
                    Color::BLACK.scale_alpha(if hovered {
                        if dark { 0.14 } else { 0.07 }
                    } else if dark {
                        0.08
                    } else {
                        0.04
                    })
                },
                offset: Vector::new(0.0, if hovered || selected { 10.0 } else { 4.0 }),
                blur_radius: if hovered || selected { 20.0 } else { 10.0 },
            },
            text_color: theme.palette().text,
            ..Default::default()
        }
    })
    .into()
}

fn recent_project_name(app: &App, index: usize, path: &str) -> String {
    if let Some(name) = app.recent_projects_edits.get(index).map(|name| name.trim()) {
        if !name.is_empty() {
            return name.to_owned();
        }
    }

    if let Some(meta) = app.recent_projects_meta.iter().find(|meta| meta.path == path) {
        let name = meta.name.trim();
        if !name.is_empty() {
            return name.to_owned();
        }
    }

    project_name_from_path(path)
}

fn project_name_from_path(path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return "未命名项目".to_string();
    }
    let without_trailing_separator = trimmed.trim_end_matches(|ch| ch == '/' || ch == '\\');
    let display_path =
        if without_trailing_separator.is_empty() { trimmed } else { without_trailing_separator };

    if let Some(name) = display_path.rsplit(['/', '\\']).find(|segment| !segment.is_empty()) {
        return name.to_string();
    }

    std::path::Path::new(display_path)
        .file_name()
        .and_then(|segment| segment.to_str())
        .filter(|segment| !segment.trim().is_empty())
        .unwrap_or(display_path)
        .to_string()
}
#[cfg(test)]
#[path = "recent_tests.rs"]
mod recent_tests;
