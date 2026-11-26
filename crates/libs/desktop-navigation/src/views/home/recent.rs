//! 首页视图模块，负责项目入口、最近项目和常用工具入口的界面组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{Grid, Image, Space, button, column, container, hover, row, scrollable, text};
use iced::{Alignment, Background, Border, Color, Element, Length, Pixels, Theme, Vector};

use desktop_assets::assets::{self, Icon};
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{
    settings_muted_text_style, settings_panel_style,
};
use desktop_workspace::project_message::edit_meta::RecentProjectMeta;

use super::common::{
    home_cjk_emphasis_font, home_emphasis_font, icon_svg, is_dark_theme, parse_hex_color,
    primary_button, project_accent_color, project_avatar, secondary_button,
};

const SHORTCUT_ICON_SIZE: f32 = 94.0;
const TWO_COLUMN_MIN_WINDOW_WIDTH: f32 = 588.0;

#[derive(Debug, Clone)]
pub enum Message {
    OpenFolderPressed,
    #[cfg(not(target_arch = "wasm32"))]
    OpenCliWorkbench,
    #[cfg(not(target_arch = "wasm32"))]
    OpenGitWorkbench,
    OpenThirdPartyAgents,
    OpenRecentPressed(String),
}

/// 首页实际使用的只读工作区字段，不持有或复制宿主状态。
pub struct State<'a> {
    pub desktop_locale: config_ui::DesktopLocale,
    pub window_width: f32,
    pub recent_projects: &'a [String],
    pub recent_projects_edits: &'a [String],
    pub recent_projects_meta: &'a [RecentProjectMeta],
    pub project_path: Option<&'a str>,
}

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
pub fn render_body<'a>(app: &State<'_>) -> Element<'a, Message> {
    let i18n = I18n::new(app.desktop_locale);
    let recent = if app.recent_projects.is_empty() {
        container(text(i18n.__("暂无最近项目")).size(14).style(settings_muted_text_style))
            .center_x(Length::Fill)
            .padding([24, 7])
            .into()
    } else {
        render_recent_projects_body(app, i18n)
    };
    let section =
        column![render_shortcuts(i18n, app.window_width), recent].spacing(18).width(Length::Fill);

    scrollable(container(section).padding([18, 16]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill)
        .into()
}

fn render_shortcuts(i18n: I18n, window_width: f32) -> Element<'static, Message> {
    let columns = if window_width < TWO_COLUMN_MIN_WINDOW_WIDTH {
        1
    } else if cfg!(not(target_arch = "wasm32")) && window_width >= 1152.0 {
        4
    } else {
        2
    };
    let logo = Image::new(assets::get_image(Icon::Logo))
        .width(SHORTCUT_ICON_SIZE)
        .height(SHORTCUT_ICON_SIZE);
    let shortcuts =
        Grid::new().columns(columns).spacing(12).height(Length::Shrink).push(shortcut_card(
            logo.into(),
            i18n.__("识矩"),
            i18n.__("选择一个文件夹开始"),
            primary_button(i18n.__("选择文件夹"), Message::OpenFolderPressed),
        ));

    #[cfg(not(target_arch = "wasm32"))]
    let shortcuts = shortcuts
        .push(shortcut_card(
            shortcut_icon(Icon::Terminal, iced::color!(0x158ea5)),
            i18n.__("我的工作台"),
            i18n.__("使用终端与智能体"),
            secondary_button(i18n.__("打开"), Message::OpenCliWorkbench),
        ))
        .push(shortcut_card(
            shortcut_icon(Icon::GitBranch, iced::color!(0x2b8877)),
            i18n.__("Git 工作台"),
            i18n.__("查看变更和分支"),
            secondary_button(i18n.__("打开"), Message::OpenGitWorkbench),
        ));

    shortcuts
        .push(shortcut_card(
            shortcut_icon(Icon::Robot, iced::color!(0x7568d8)),
            i18n.__("第三方智能体"),
            i18n.__("安装和管理智能体"),
            secondary_button(i18n.__("打开"), Message::OpenThirdPartyAgents),
        ))
        .into()
}

fn shortcut_icon(icon: Icon, accent: Color) -> Element<'static, Message> {
    container(icon_svg(icon, 52.0).style(move |_theme: &Theme, _| iced::widget::svg::Style {
        color: Some(accent),
    }))
    .center(SHORTCUT_ICON_SIZE)
    .style(move |_theme: &Theme| iced::widget::container::Style {
        background: Some(accent.scale_alpha(0.14).into()),
        border: Border { radius: (SHORTCUT_ICON_SIZE / 2.0).into(), ..Default::default() },
        ..Default::default()
    })
    .into()
}

fn shortcut_card(
    icon: Element<'static, Message>,
    title: &'static str,
    description: &'static str,
    action: Element<'static, Message>,
) -> Element<'static, Message> {
    let details = column![
        text(title).size(22).line_height(Pixels(26.0)).font(home_cjk_emphasis_font()),
        text(description).size(16).line_height(Pixels(20.0)).style(settings_muted_text_style),
    ]
    .spacing(3);
    let content = column![details, action].spacing(8).width(Length::Fill);
    let card = container(row![icon, content].spacing(10).align_y(Alignment::Center))
        .width(Length::Fill)
        .padding([14, 12])
        .style(settings_panel_style);
    let highlight = container(Space::new())
        .width(Length::Fill)
        .height(Length::Fill)
        .style(|theme: &Theme| {
            let opacity = if is_dark_theme(theme) { 0.14 } else { 0.07 };
            iced::widget::container::Style {
                background: Some(theme.palette().primary.scale_alpha(opacity).into()),
                border: Border {
                    radius: settings_panel_style(theme).border.radius,
                    ..Default::default()
                },
                ..Default::default()
            }
        });
    hover(card, highlight)
}

fn render_recent_projects_body<'a>(app: &State<'_>, i18n: I18n) -> Element<'a, Message> {
    let recent_count = container(
        text(
            i18n.format("%u 个项目", &[app.recent_projects.len().into()])
                .expect("valid project count"),
        )
        .size(11),
    )
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

    let columns =
        if app.window_width >= TWO_COLUMN_MIN_WINDOW_WIDTH && app.recent_projects.len() > 1 {
            2
        } else {
            1
        };
    let recent_list = Grid::with_children(
        app.recent_projects
            .iter()
            .enumerate()
            .map(|(index, path)| recent_project_row(app, index, path)),
    )
    .columns(columns)
    .spacing(10)
    .height(Length::Shrink);

    column![
        row![
            column![
                text(i18n.__("最近项目")).size(15).font(home_cjk_emphasis_font()),
                text(i18n.__("继续最近的工作区入口，并直接处理常用操作。")).size(12).style(
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
    .width(Length::Fill)
    .into()
}

fn recent_project_row<'a>(app: &State<'_>, index: usize, path: &str) -> Element<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
    let name = recent_project_name(app, index, path);
    let project_meta = app.recent_projects_meta.iter().find(|meta| meta.path == path);
    let custom_icon = project_meta.and_then(|meta| meta.icon.as_deref());
    let custom_color =
        project_meta.and_then(|meta| meta.icon_color.as_deref()).and_then(parse_hex_color);
    let accent = custom_color.unwrap_or_else(|| project_accent_color(path));
    let selected = app.project_path.is_some_and(|current| current == path);

    let status_badge = container(
        text(if selected { i18n.__("当前工作区") } else { i18n.__("最近项目") }).size(11),
    )
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
            text(name.clone())
                .size(14)
                .font(home_emphasis_font())
                .wrapping(text::Wrapping::WordOrGlyph)
                .style(|theme: &Theme| {
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
        .align_y(Alignment::Center)
        .wrap()
        .vertical_spacing(4),
        text(path.to_string()).size(11).wrapping(text::Wrapping::WordOrGlyph).style(
            |theme: &Theme| {
                let color = if is_dark_theme(theme) {
                    theme.palette().text.scale_alpha(0.68)
                } else {
                    theme.extended_palette().secondary.base.text
                };
                text::Style { color: Some(color) }
            }
        ),
    ]
    .spacing(6)
    .width(Length::Fill);

    let open_tag =
        container(text(if selected { i18n.__("继续处理") } else { i18n.__("打开") }).size(11))
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
    .on_press(Message::OpenRecentPressed(path.to_string()))
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

fn recent_project_name(app: &State<'_>, index: usize, path: &str) -> String {
    if let Some(name) = app.recent_projects_edits.get(index).map(|name| name.trim())
        && !name.is_empty() {
            return name.to_owned();
        }

    if let Some(meta) = app.recent_projects_meta.iter().find(|meta| meta.path == path) {
        let name = meta.name.trim();
        if !name.is_empty() {
            return name.to_owned();
        }
    }

    project_name_from_path(desktop_i18n::I18n::new(app.desktop_locale), path)
}

fn project_name_from_path(i18n: desktop_i18n::I18n, path: &str) -> String {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return i18n.__("未命名项目").to_string();
    }
    let without_trailing_separator = trimmed.trim_end_matches(['/', '\\']);
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
