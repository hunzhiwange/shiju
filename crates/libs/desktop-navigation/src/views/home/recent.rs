//! 首页视图模块，负责项目入口、最近项目和常用工具入口的界面组合。
//!
//! 本模块只负责视图组合与样式适配，不持有业务状态，也不扩大外部能力边界。

use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{
    Grid, Image, Space, button, column, container, hover, row, scrollable, text, tooltip,
};
use iced::{Alignment, Background, Border, Color, Element, Length, Pixels, Point, Theme, Vector};

use desktop_assets::assets::{self, Icon};
use desktop_i18n::I18n;
use desktop_ui_controls::overlays::PointBelowOverlay;
use desktop_ui_controls::right_click_area::RightClickArea;
use desktop_ui_controls::system_settings_common::icons::{icon_btn_style, icon_btn_svg_style};
use desktop_ui_controls::system_settings_common::{
    settings_divider, settings_muted_text_style, settings_panel_style, settings_text_input,
    with_settings_modal_escape,
};
use desktop_ui_controls::tooltip::tooltip_dark_style;
use desktop_workspace::project_message::edit_meta::RecentProjectMeta;

use super::common::{
    home_cjk_emphasis_font, home_emphasis_font, icon_svg, is_dark_theme, parse_hex_color,
    project_accent_color, project_avatar,
};

const WORKSPACE_ICON_SIZE: f32 = 94.0;
const APP_ICON_SIZE: f32 = 36.0;
const SHORTCUT_COLUMN_SPACING: f32 = 12.0;
const TWO_COLUMN_MIN_WINDOW_WIDTH: f32 = 588.0;
pub const SEARCH_INPUT_ID: &str = "home-recent-project-search";

#[derive(Debug, Clone)]
pub enum Message {
    #[cfg(not(target_arch = "wasm32"))]
    OpenCliWorkbench,
    #[cfg(not(target_arch = "wasm32"))]
    OpenGitWorkbench,
    OpenThirdPartyAgents,
    OpenAgentSettings,
    OpenCron,
    #[cfg(not(target_arch = "wasm32"))]
    OpenTaskPool,
    #[cfg(not(target_arch = "wasm32"))]
    OpenShijuSkills,
    #[cfg(not(target_arch = "wasm32"))]
    OpenShijuApps,
    #[cfg(not(target_arch = "wasm32"))]
    OpenDigitalEmployees,
    OpenRecentPressed(String),
    SearchToggled,
    SearchChanged(String),
    ProjectMenu(ProjectMenuMessage),
}

#[derive(Debug, Clone)]
pub enum ProjectMenuMessage {
    Open(String, Point),
    Close,
    Action(ProjectMenuAction),
}

#[derive(Debug, Clone)]
pub enum ProjectMenuAction {
    Edit,
    CopyPath,
    Delete,
}

/// 首页实际使用的只读工作区字段，不持有或复制宿主状态。
pub struct State<'a> {
    pub desktop_locale: config_ui::DesktopLocale,
    pub window_width: f32,
    pub recent_projects: &'a [String],
    pub recent_projects_edits: &'a [String],
    pub recent_projects_meta: &'a [RecentProjectMeta],
    pub search_query: Option<&'a str>,
    pub project_menu: Option<&'a (String, Point)>,
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
        column![render_shortcuts(i18n, app), recent].spacing(18).width(Length::Fill);

    let mut body = scrollable(container(section).padding([18, 16]).width(Length::Fill))
        .direction(Direction::Vertical(Scrollbar::new().width(4).scroller_width(4)))
        .height(Length::Fill);
    if app.project_menu.is_some() {
        body = body.on_scroll(|_| Message::ProjectMenu(ProjectMenuMessage::Close));
    }
    body.into()
}

fn render_shortcuts(i18n: I18n, app: &State<'_>) -> Element<'static, Message> {
    let window_width = app.window_width;
    let columns = if cfg!(not(target_arch = "wasm32")) && window_width >= 960.0 {
        4
    } else if window_width >= TWO_COLUMN_MIN_WINDOW_WIDTH {
        2
    } else {
        1
    };
    let logo = Image::new(assets::get_image(Icon::Logo))
        .width(WORKSPACE_ICON_SIZE)
        .height(WORKSPACE_ICON_SIZE);
    let recent_project = app.recent_projects.first();
    let workspace = Grid::new()
        .columns(columns)
        .spacing(SHORTCUT_COLUMN_SPACING)
        .height(Length::Shrink)
        .push(workspace_card(
            logo.into(),
            i18n.__("识矩引擎"),
            i18n.__("打开最近项目"),
            recent_project.cloned().map(Message::OpenRecentPressed),
        ));

    #[cfg(not(target_arch = "wasm32"))]
    let workspace = workspace
        .push(workspace_card(
            shortcut_icon(Icon::Terminal, iced::color!(0x158ea5), WORKSPACE_ICON_SIZE, 52.0),
            i18n.__("我的工作台"),
            i18n.__("使用终端与智能体"),
            Message::OpenCliWorkbench,
        ))
        .push(workspace_card(
            shortcut_icon(Icon::GitBranch, iced::color!(0x2b8877), WORKSPACE_ICON_SIZE, 52.0),
            i18n.__("Git 工作台"),
            i18n.__("查看变更和分支"),
            Message::OpenGitWorkbench,
        ));

    let workspace = workspace.push(workspace_card(
        shortcut_icon(Icon::Robot, iced::color!(0x7568d8), WORKSPACE_ICON_SIZE, 52.0),
        i18n.__("第三方智能体"),
        i18n.__("安装和管理智能体"),
        Message::OpenThirdPartyAgents,
    ));

    let tasks = column![compact_shortcut_card(
        shortcut_icon(Icon::Clock, iced::color!(0xc26420), APP_ICON_SIZE, 20.0),
        i18n.__("定时任务"),
        Message::OpenCron,
    )]
    .spacing(10)
    .width(Length::Fill);
    #[cfg(not(target_arch = "wasm32"))]
    let tasks = tasks.push(compact_shortcut_card(
        shortcut_icon(Icon::ClipboardCheck, iced::color!(0x3585d6), APP_ICON_SIZE, 20.0),
        i18n.__("任务池"),
        Message::OpenTaskPool,
    ));

    let apps = Grid::new()
        .columns(columns)
        .spacing(SHORTCUT_COLUMN_SPACING)
        .height(Length::Shrink)
        .push(workspace_card(
            shortcut_icon(Icon::Robot, iced::color!(0x545acc), WORKSPACE_ICON_SIZE, 52.0),
            i18n.__("智能体"),
            i18n.__("配置智能体"),
            Message::OpenAgentSettings,
        ))
        .push(tasks);

    #[cfg(not(target_arch = "wasm32"))]
    let apps = apps
        .push(workspace_card(
            shortcut_icon(Icon::Star, iced::color!(0xab780a), WORKSPACE_ICON_SIZE, 52.0),
            i18n.__("识矩技能"),
            i18n.__("浏览和安装技能"),
            Message::OpenShijuSkills,
        ))
        .push(
            column![
                compact_shortcut_card(
                    shortcut_icon(Icon::Box, iced::color!(0x845ad5), APP_ICON_SIZE, 20.0),
                    i18n.__("识矩应用"),
                    Message::OpenShijuApps,
                ),
                compact_shortcut_card(
                    shortcut_icon(Icon::Robot, iced::color!(0xc54b84), APP_ICON_SIZE, 20.0),
                    i18n.__("数字员工"),
                    Message::OpenDigitalEmployees,
                ),
            ]
            .spacing(10)
            .width(Length::Fill),
        );

    column![workspace, apps].spacing(8).width(Length::Fill).into()
}

fn shortcut_icon(
    icon: Icon,
    accent: Color,
    diameter: f32,
    icon_size: f32,
) -> Element<'static, Message> {
    container(icon_svg(icon, icon_size).style(move |_theme: &Theme, _| iced::widget::svg::Style {
        color: Some(accent),
    }))
    .center(diameter)
    .style(move |_theme: &Theme| iced::widget::container::Style {
        background: Some(accent.scale_alpha(0.14).into()),
        border: Border { radius: (diameter / 2.0).into(), ..Default::default() },
        ..Default::default()
    })
    .into()
}

fn workspace_card(
    icon: Element<'static, Message>,
    title: &'static str,
    description: &'static str,
    action: impl Into<Option<Message>>,
) -> Element<'static, Message> {
    let details = column![
        text(title)
            .size(24)
            .line_height(Pixels(28.0))
            .wrapping(text::Wrapping::WordOrGlyph)
            .font(home_cjk_emphasis_font()),
        text(description).size(16).line_height(Pixels(20.0)).style(settings_muted_text_style),
    ]
    .spacing(3)
    .width(Length::Fill);
    shortcut_panel(row![icon, details].spacing(10).align_y(Alignment::Center), [14, 12], action.into())
}

fn compact_shortcut_card(
    icon: Element<'static, Message>,
    title: &'static str,
    action: Message,
) -> Element<'static, Message> {
    let content = row![
        icon,
        text(title)
            .size(18)
            .line_height(Pixels(22.0))
            .font(home_cjk_emphasis_font())
            .width(Length::Fill),
    ]
    .spacing(7)
    .align_y(Alignment::Center);
    shortcut_panel(content, [10, 10], Some(action))
}

fn shortcut_panel(
    content: impl Into<Element<'static, Message>>,
    padding: [u16; 2],
    action: Option<Message>,
) -> Element<'static, Message> {
    let card = button(content)
        .width(Length::Fill)
        .padding(padding)
        .on_press_maybe(action)
        .style(|theme: &Theme, status| {
            let panel = settings_panel_style(theme);
            let mut border = panel.border;
            match status {
                button::Status::Hovered => border.color = theme.palette().primary.scale_alpha(0.45),
                button::Status::Pressed => border.color = theme.palette().primary,
                _ => {}
            }
            button::Style {
                background: panel.background,
                text_color: theme.palette().text,
                border,
                shadow: panel.shadow,
                snap: panel.snap,
            }
        });
    let highlight =
        container(Space::new()).width(Length::Fill).height(Length::Fill).style(|theme: &Theme| {
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
    let query = app.search_query.unwrap_or_default().trim().to_lowercase();
    let projects: Vec<_> = app
        .recent_projects
        .iter()
        .enumerate()
        .filter_map(|(index, path)| {
            let name = recent_project_name(app, index, path);
            (query.is_empty()
                || name.to_lowercase().contains(&query)
                || path.to_lowercase().contains(&query))
            .then_some((path, name))
        })
        .collect();
    let recent_count = container(
        text(i18n.format("%u 个项目", &[projects.len().into()]).expect("valid project count"))
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

    let recent_list: Element<'_, Message> = if projects.is_empty() {
        container(text(i18n.__("未找到匹配的项目")).size(14).style(settings_muted_text_style))
            .center_x(Length::Fill)
            .padding([24, 7])
            .into()
    } else {
        let columns = if app.window_width >= TWO_COLUMN_MIN_WINDOW_WIDTH && projects.len() > 1 {
            2
        } else {
            1
        };
        Grid::with_children(
            projects.into_iter().map(|(path, name)| recent_project_row(app, path, name)),
        )
        .columns(columns)
        .spacing(10)
        .height(Length::Shrink)
        .into()
    };

    let search_label = i18n.__("搜索项目");
    let (search_icon, search_tooltip) = if app.search_query.is_some() {
        (Icon::X, i18n.__("收起搜索"))
    } else {
        (Icon::Search, search_label)
    };
    let search_button = button(icon_svg(search_icon, 14.0).style(icon_btn_svg_style))
        .padding([4, 6])
        .style(icon_btn_style)
        .on_press(Message::SearchToggled);
    let search_button =
        tooltip(search_button, text(search_tooltip).size(12), tooltip::Position::Bottom)
            .gap(5)
            .padding(8)
            .style(tooltip_dark_style);
    let heading = row![
        column![
            row![
                text(i18n.__("最近项目")).size(15).font(home_cjk_emphasis_font()),
                search_button,
            ]
            .spacing(6)
            .align_y(Alignment::Center),
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
    .align_y(Alignment::Center);
    let mut section = column![heading].spacing(14).width(Length::Fill);
    if let Some(query) = app.search_query {
        let input = settings_text_input(search_label, query)
            .id(SEARCH_INPUT_ID)
            .on_input(Message::SearchChanged);
        section = section.push(with_settings_modal_escape(
            container(input).max_width(320).into(),
            Message::SearchToggled,
        ));
    }
    section.push(recent_list).into()
}

fn recent_project_row<'a>(app: &State<'_>, path: &str, name: String) -> Element<'a, Message> {
    let i18n = desktop_i18n::I18n::new(app.desktop_locale);
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

    let card = button(
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
    });

    let project_path = path.to_owned();
    let card: Element<'a, Message> = RightClickArea::new(
        card.into(),
        Box::new(move |point| {
            Message::ProjectMenu(ProjectMenuMessage::Open(project_path.clone(), point))
        }),
    )
    .into();
    if let Some((menu_path, point)) = app.project_menu
        && menu_path == path
    {
        PointBelowOverlay::new(card, project_menu(i18n, path))
            .show(true)
            .anchor(*point)
            .close_on_target_click(true)
            .capture_keyboard()
            .on_close(Message::ProjectMenu(ProjectMenuMessage::Close))
            .into()
    } else {
        card
    }
}

fn project_menu(i18n: I18n, path: &str) -> Element<'static, Message> {
    let action = |icon, label, message| {
        button(
            row![icon_svg(icon, 15.0).style(icon_btn_svg_style), text(label).size(13)]
                .spacing(9)
                .align_y(Alignment::Center),
        )
        .on_press(Message::ProjectMenu(ProjectMenuMessage::Action(message)))
        .padding([8, 9])
        .width(Length::Fill)
        .style(icon_btn_style)
    };
    let mut items = column![
        action(Icon::Pencil, i18n.__("编辑项目"), ProjectMenuAction::Edit),
        action(Icon::Copy, i18n.__("复制路径"), ProjectMenuAction::CopyPath),
    ]
    .spacing(3);
    if !desktop_workspace::projects::is_builtin_workspace_project(path) {
        items = items.push(settings_divider()).push(action(
            Icon::Trash,
            i18n.__("删除"),
            ProjectMenuAction::Delete,
        ));
    }
    container(items)
        .width(168)
        .padding(5)
        .style(|theme| iced::widget::container::Style {
            background: Some(theme.palette().background.into()),
            ..settings_panel_style(theme)
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
