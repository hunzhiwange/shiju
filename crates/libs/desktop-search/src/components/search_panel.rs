use iced::widget::{
    Id, Space, button, column, container, mouse_area, opaque, row, scrollable, stack,
};
use iced::widget::{text, text_input};
use iced::{Background, Color, Element, Length, Theme};

use crate::message::SearchMessage;
type Message = SearchMessage;

pub struct ViewData<'a> {
    pub search_text: &'a str,
    pub files: &'a [String],
    pub recent_projects: &'a [String],
    pub sessions: Vec<SessionSearchMatch>,
    pub labels: [&'static str; 5],
}

const SEARCH_PANEL_SCROLLBAR_WIDTH: f32 = 4.0;
const SEARCH_PANEL_SCROLLBAR_GUTTER: f32 = 12.0;

pub fn input_id() -> Id {
    Id::new("global_search_input")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionSearchMatch {
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

pub fn matching_sessions(sessions: &[session_info::Info], previews: &std::collections::HashMap<String, String>, title: impl Fn(&session_info::Info) -> String, query: &str) -> Vec<SessionSearchMatch> {
    let needle = query.trim().to_ascii_lowercase();
    if needle.is_empty() {
        return Vec::new();
    }

    sessions
        .iter()
        .filter(|session| session.is_user_navigable())
        .filter(|session| {
            let preview =
                previews.get(&session.id).map(String::as_str).unwrap_or_default();
            session.title.to_ascii_lowercase().contains(&needle)
                || title(session)
                    .to_ascii_lowercase()
                    .contains(&needle)
                || preview.to_ascii_lowercase().contains(&needle)
                || session.directory.to_ascii_lowercase().contains(&needle)
                || session.id.to_ascii_lowercase().contains(&needle)
        })
        .take(8)
        .map(|session| SessionSearchMatch {
            id: session.id.clone(),
            title: title(session),
            preview: previews.get(&session.id).cloned(),
            directory: session.directory.clone(),
        })
        .collect()
}

pub fn overlay<'a>(app: ViewData<'a>) -> Element<'a, Message> {
    let close = Message::Toggle(false);
    let input = text_input(
        app.labels[0],
        app.search_text,
    )
    .id(input_id())
    .on_input(|value| Message::InputChanged(value))
    .padding([8, 10])
    .size(14)
    .style(search_input_style);

    let close_btn =
        button(text(app.labels[1]).size(12))
            .on_press(close.clone())
            .padding([8, 12])
            .style(|theme: &Theme, status| {
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
            });

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

    let modal: Element<'_, Message> = container(mouse_area(card).on_press(Message::Ignore))
        .width(Length::Fill)
        .height(Length::Fill)
        .center_x(Length::Fill)
        .padding(iced::Padding::default().top(54).right(0).bottom(0).left(0))
        .into();

    stack![scrim, modal].into()
}

/// 构建搜索结果列表。
pub fn view<'a>(app: ViewData<'a>) -> Element<'a, Message> {
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
    let files = app.files;

    // 如果有匹配的文件，添加到结果列表
    if !files.is_empty() {
        // 添加"文件"分组标题
        content = content.push(text(app.labels[2]));

        // 为每个匹配的文件创建按钮
        for f in files {
            let btn = button(text(f.clone()))
                .on_press(Message::SelectFile(f.clone()))
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
        content = content.push(text(app.labels[3]));

        // 为每个匹配的项目创建按钮
        for p in projects {
            let btn = button(text(p.clone()))
                .on_press(Message::SelectProject(p))
                .style(result_button_style);
            content = content.push(btn);
        }
    }

    // ========== 会话搜索部分 ==========
    // 在历史会话中搜索
    let sessions = app.sessions;

    // 如果有匹配的会话，添加到结果列表
    if !sessions.is_empty() {
        // 添加"历史会话"分组标题
        content =
            content.push(text(app.labels[4]));

        // 为每个匹配的会话创建按钮
        for session in sessions {
            let mut label = column![text(session.title.clone())]
            .spacing(2);
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
                .on_press(Message::SelectSession(session.id))
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
