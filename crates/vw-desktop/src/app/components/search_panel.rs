//! 搜索面板组件模块
//!
//! 本模块提供全局搜索功能的 UI 组件，用于在应用程序中快速查找和导航到：
//! - 文件
//! - 项目
//! - 历史会话
//!
//! # 功能特性
//!
//! - 实时搜索：根据用户输入的搜索文本即时过滤结果
//! - 不区分大小写的匹配
//! - 分类展示：将搜索结果按类型分组显示
//! - 结果限制：每种类型的搜索结果最多显示 8 条，避免列表过长
//!
//! # 使用方式
//!
//! ```ignore
//! use crate::app::components::search_panel;
//!
//! let search_ui = search_panel::view(&app);
//! ```

use iced::widget::{Space, button, column, container, mouse_area, opaque, row, scrollable, stack};
use iced::widget::{text, text_input};
use iced::{Background, Color, Element, Length, Theme};

use crate::app::{App, Message, message};

const SEARCH_PANEL_SCROLLBAR_WIDTH: f32 = 4.0;
const SEARCH_PANEL_SCROLLBAR_GUTTER: f32 = 12.0;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SessionSearchMatch {
    pub id: String,
    pub title: String,
    pub preview: Option<String>,
    pub directory: String,
}

pub(super) fn result_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let background = match status {
        iced::widget::button::Status::Hovered => Some(palette.background.weak.color),
        iced::widget::button::Status::Pressed => Some(palette.background.strong.color),
        _ => Some(palette.background.base.color),
    };

    iced::widget::button::Style {
        background: background.map(Background::Color),
        text_color: palette.background.base.text,
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 6.0.into() },
        ..Default::default()
    }
}

pub(super) fn search_input_style(
    theme: &Theme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
    let palette = theme.extended_palette();
    let is_focused = matches!(status, iced::widget::text_input::Status::Focused { .. });
    let is_hovered = matches!(status, iced::widget::text_input::Status::Hovered)
        || matches!(status, iced::widget::text_input::Status::Focused { is_hovered: true });
    let border_color = if is_focused {
        palette.primary.base.color
    } else if is_hovered {
        palette.background.strong.color
    } else {
        palette.background.weak.color
    };

    iced::widget::text_input::Style {
        background: Background::Color(palette.background.base.color),
        border: iced::Border { width: 1.0, color: border_color, radius: 8.0.into() },
        icon: palette.background.base.text.scale_alpha(0.65),
        placeholder: palette.background.strong.text.scale_alpha(0.70),
        value: palette.background.base.text,
        selection: palette.primary.base.color.scale_alpha(0.30),
    }
}

pub(super) fn matching_sessions(app: &App, query: &str) -> Vec<SessionSearchMatch> {
    let needle = query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }

    app.sessions
        .iter()
        .filter(|session| {
            let preview =
                app.session_previews.get(&session.id).map(String::as_str).unwrap_or_default();
            session.title.to_ascii_lowercase().contains(&needle)
                || preview.to_ascii_lowercase().contains(&needle)
                || session.directory.to_ascii_lowercase().contains(&needle)
                || session.id.to_ascii_lowercase().contains(&needle)
        })
        .take(8)
        .map(|session| SessionSearchMatch {
            id: session.id.clone(),
            title: session.title.clone(),
            preview: app.session_previews.get(&session.id).cloned(),
            directory: session.directory.clone(),
        })
        .collect()
}

pub fn overlay(app: &App) -> Element<'_, Message> {
    let close = Message::Search(message::SearchMessage::Toggle(false));
    let input = text_input("搜索文件 / 项目 / 历史会话", &app.search_text)
        .on_input(|value| Message::Search(message::SearchMessage::InputChanged(value)))
        .padding([8, 10])
        .size(14)
        .style(search_input_style);

    let close_btn = button(text("关闭").size(12)).on_press(close.clone()).padding([8, 12]).style(
        |theme: &Theme, status| {
            let palette = theme.extended_palette();
            let background = match status {
                iced::widget::button::Status::Hovered => palette.background.weak.color,
                iced::widget::button::Status::Pressed => palette.background.strong.color,
                _ => palette.background.base.color,
            };
            iced::widget::button::Style {
                background: Some(Background::Color(background)),
                text_color: palette.background.base.text,
                border: iced::Border {
                    width: 1.0,
                    color: palette.background.strong.color,
                    radius: 8.0.into(),
                },
                ..Default::default()
            }
        },
    );

    let card = container(
        column![row![input, close_btn].spacing(8).align_y(iced::Alignment::Center), view(app)]
            .spacing(10),
    )
    .width(Length::Fixed(680.0))
    .height(Length::Fixed(460.0))
    .padding(14)
    .style(|theme: &Theme| {
        let palette = theme.extended_palette();
        container::Style {
            background: Some(Background::Color(palette.background.base.color)),
            text_color: Some(palette.background.base.text),
            border: iced::Border {
                width: 1.0,
                color: palette.background.strong.color,
                radius: 12.0.into(),
            },
            ..Default::default()
        }
    });

    let scrim = opaque(
        mouse_area(container(Space::new().width(Length::Fill).height(Length::Fill)).style(|_| {
            container::Style {
                background: Some(Background::Color(Color::from_rgba(0.0, 0.0, 0.0, 0.35))),
                ..Default::default()
            }
        }))
        .on_press(close),
    );

    let modal: Element<'_, Message> = container(mouse_area(card).on_press(Message::None))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .padding(iced::Padding::default().top(54).right(0).bottom(0).left(0))
        .into();

    stack![scrim, modal].into()
}

/// 构建搜索面板视图
///
/// 根据应用程序当前状态中的搜索文本，生成包含匹配结果的 UI 元素。
/// 搜索范围包括：
/// - 当前项目的文件列表
/// - 最近的打开项目
/// - 历史会话记录
///
/// # 参数
///
/// - `app`: 应用程序状态的不可变引用，包含搜索文本和各类数据源
///
/// # 返回值
///
/// 返回一个 `Element` 类型的 UI 组件，包含：
/// - 空容器：当搜索文本为空或仅包含空白字符时
/// - 可滚动的结果列表：包含按类型分组的搜索结果按钮
///
/// # 搜索逻辑
///
/// 1. 首先对搜索文本进行 trim 处理，去除首尾空白
/// 2. 如果搜索文本为空，返回空容器
/// 3. 对每种数据源进行不区分大小写的子串匹配
/// 4. 每种类型的匹配结果限制为 8 条
///
/// # 示例
///
/// ```ignore
/// // 在主视图更新中调用
/// fn update_view(app: &App) -> Element<Message> {
///     // 当搜索面板激活时
///     if app.show_search_panel {
///         search_panel::view(app)
///     } else {
///         // 其他视图...
///     }
/// }
/// ```
pub fn view(app: &App) -> Element<'_, Message> {
    // 获取并清理搜索文本
    let q = app.search_text.trim();
    let q_lower = q.to_ascii_lowercase();

    // 如果搜索文本为空，返回一个尺寸收缩的空容器
    // 这样可以避免在用户尚未输入任何内容时显示空白面板
    if q.is_empty() {
        return container(Space::new().height(Length::Fill))
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    // 创建主要内容列，设置子元素间距为 8 像素
    let mut content = column![].spacing(8);

    // ========== 文件搜索部分 ==========
    let files = app.cached_search_panel_file_results();

    // 如果有匹配的文件，添加到结果列表
    if !files.is_empty() {
        // 添加"文件"分组标题
        content = content.push(text("文件"));

        // 为每个匹配的文件创建按钮
        for f in files {
            let btn = button(text(f.clone()))
                .on_press(Message::Search(message::SearchMessage::SelectFile(f.clone())))
                .style(result_button_style);
            content = content.push(btn);
        }
    }

    // ========== 项目搜索部分 ==========
    // 在最近打开的项目中搜索
    let projects = app
        .recent_projects
        .iter()
        .filter(|p| p.to_ascii_lowercase().contains(&q_lower))
        .take(8)
        .cloned()
        .collect::<Vec<_>>();

    // 如果有匹配的项目，添加到结果列表
    if !projects.is_empty() {
        // 添加"项目"分组标题
        content = content.push(text("项目"));

        // 为每个匹配的项目创建按钮
        for p in projects {
            let btn = button(text(p.clone()))
                .on_press(Message::Search(message::SearchMessage::SelectProject(p)))
                .style(result_button_style);
            content = content.push(btn);
        }
    }

    // ========== 会话搜索部分 ==========
    // 在历史会话中搜索
    let sessions = matching_sessions(app, q);

    // 如果有匹配的会话，添加到结果列表
    if !sessions.is_empty() {
        // 添加"历史会话"分组标题
        content = content.push(text("历史会话"));

        // 为每个匹配的会话创建按钮
        for session in sessions {
            let mut label = column![text(session.title.clone())].spacing(2);
            if let Some(preview) = session.preview.filter(|value| !value.trim().is_empty()) {
                label = label.push(text(preview).size(12).style(|theme: &Theme| {
                    iced::widget::text::Style {
                        color: Some(theme.extended_palette().secondary.base.text),
                    }
                }));
            }
            if !session.directory.trim().is_empty() {
                label =
                    label.push(text(session.directory.clone()).size(11).style(|theme: &Theme| {
                        iced::widget::text::Style {
                            color: Some(theme.palette().text.scale_alpha(0.55)),
                        }
                    }));
            }

            let btn = button(label)
                .width(Length::Fill)
                .on_press(Message::Search(message::SearchMessage::SelectSession(session.id)))
                .style(result_button_style);
            content = content.push(btn);
        }
    }

    // 将内容包装在可滚动容器中返回
    scrollable(container(content).width(Length::Fill).padding(iced::Padding::default().right(4.0)))
        .direction(iced::widget::scrollable::Direction::Vertical(
            iced::widget::scrollable::Scrollbar::new()
                .width(SEARCH_PANEL_SCROLLBAR_WIDTH)
                .scroller_width(SEARCH_PANEL_SCROLLBAR_WIDTH)
                .spacing(SEARCH_PANEL_SCROLLBAR_GUTTER - SEARCH_PANEL_SCROLLBAR_WIDTH),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
