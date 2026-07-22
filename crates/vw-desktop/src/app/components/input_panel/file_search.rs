//! 输入面板局部组件。
//!
//! 本模块负责输入区附件、文件搜索或图标展示相关的可复用逻辑。

use iced::widget::svg;
/// 重新导出 use iced::widget::{button, column, container, row, scrollable, text}，让上层模块通过稳定路径访问。
use iced::widget::{button, column, container, row, scrollable, text};
/// 重新导出 use iced::{Background, Border, Color, Element, Length, Theme}，让上层模块通过稳定路径访问。
use iced::{Background, Border, Color, Element, Length, Theme};

/// 重新导出 use crate::app::assets::{self, Icon}，让上层模块通过稳定路径访问。
use crate::app::assets::{self, Icon};
/// 重新导出 use crate::app::message::chat::input::ranked_file_search_entries，让上层模块通过稳定路径访问。
use crate::app::message::chat::input::ranked_file_search_entries;
/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};

pub(super) fn highlighted_file_search_query(query: &str) -> String {
    format!("@{}", query.trim().replace('\\', "/"))
}

pub(super) fn display_file_search_path(path: &str, project_root: Option<&str>) -> String {
    if let Some(project_root) = project_root {
        std::path::Path::new(path)
            .strip_prefix(project_root)
            .ok()
            .and_then(|path| path.to_str())
            .unwrap_or(path)
            .replace('\\', "/")
    } else {
        path.replace('\\', "/")
    }
}

pub(super) fn empty_file_search_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.extended_palette().secondary.base.text) }
}

pub(super) fn file_search_icon_style(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style { color: Some(theme.palette().text.scale_alpha(0.65)) }
}

pub(super) fn file_search_entry_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let hovered = matches!(status, iced::widget::button::Status::Hovered);
    let background = if hovered || selected {
        Some(Background::Color(palette.background.weak.color))
    } else {
        None
    };

    iced::widget::button::Style {
        background,
        border: Border { radius: 6.0.into(), ..Default::default() },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

pub(super) fn file_search_query_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.palette().primary) }
}

pub(super) fn file_search_overlay_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.base.color)),
        border: Border { width: 1.0, color: palette.background.strong.color, radius: 12.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(0.12),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 12.0,
        },
        ..Default::default()
    }
}

/// 构建或定位 file search overlay，用于把浮层稳定附着到目标控件。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 返回值是 Iced `Element`，调用方继续组合到当前界面树中。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn file_search_overlay(app: &App) -> Element<'_, Message> {
    let files = ranked_file_search_entries(app);
    let highlighted_query = highlighted_file_search_query(&app.file_search_query);

    let mut list = column![].spacing(4);

    if files.is_empty() {
        list = list.push(text("No matching files").size(13).style(empty_file_search_text_style));
    } else {
        for (i, entry) in files.iter().take(20).enumerate() {
            let path = &entry.path;
            let display_path = display_file_search_path(path, app.project_path.as_deref());

            let icon = if entry.is_dir { Icon::FolderOpen } else { Icon::FileText };
            let icon_svg = svg::Svg::<iced::Theme>::new(assets::get_icon(icon))
                .width(Length::Fixed(14.0))
                .height(Length::Fixed(14.0))
                .style(file_search_icon_style);

            let row_content = row![icon_svg, text(display_path).size(13)]
                .spacing(8)
                .align_y(iced::alignment::Vertical::Center);

            let selected = i == app.file_search_selected_index;
            let btn = button(row_content)
                .width(Length::Fill)
                .padding([6, 10])
                .style(move |theme: &Theme, status| {
                    file_search_entry_style(theme, status, selected)
                })
                .on_press(Message::Chat(message::ChatMessage::FileSearchSelect(path.clone())));

            list = list.push(btn);
        }
    }

    container(
        column![
            container(text(highlighted_query).size(12).style(file_search_query_text_style))
                .padding([2, 2]),
            scrollable(list).id(app.file_search_scroll_id.clone()).height(Length::Fixed(320.0))
        ]
        .spacing(6),
    )
    .padding([6, 8])
    .width(Length::Fixed(320.0))
    .style(file_search_overlay_style)
    .into()
}
