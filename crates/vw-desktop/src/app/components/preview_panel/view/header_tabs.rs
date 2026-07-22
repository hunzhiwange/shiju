//! 预览面板标签页头部组件
//!
//! 本模块实现了预览面板顶部的标签页条（header tabs）组件，用于显示和管理多个打开的预览标签页。
//!
//! # 主要功能
//!
//! - **标签页渲染**：动态渲染所有打开的预览标签页，包括文件图标和标题
//! - **标签页交互**：支持点击切换、关闭标签页、右键菜单、拖拽等交互
//! - **视觉反馈**：提供选中状态、悬停状态、按下状态等视觉反馈
//! - **自适应主题**：根据明暗主题自动调整标签页的颜色和样式
//! - **可滚动容器**：当标签页过多时支持水平滚动
//!
//! # 使用场景
//!
//! 该组件通常作为预览面板的头部使用，显示当前打开的所有预览文件标签，
//! 用户可以通过点击标签切换预览内容，通过关闭按钮或右键菜单关闭标签。

use crate::app::assets::Icon;
use crate::app::components::overlays::point_below::PointBelowOverlay;
use crate::app::components::widgets::RightClickArea;
use crate::app::{App, Message, message};
use iced::widget::scrollable::{Direction, Scrollbar};
use iced::widget::{button, container, row, scrollable, text};
use iced::{Background, Color, Element, Length, Point};

use super::super::styles::{
    file_icon_for, file_tab_icon_svg, should_show_preview_tabs_scrollbar, small_icon_svg,
    truncate_title,
};
use super::super::tabs::build_preview_tab_menu;
use super::super::widgets::DraggableArea;

pub(super) fn header_tabs_height(scrollbar_visible: bool) -> f32 {
    if scrollbar_visible { 36.0 } else { 30.0 }
}

pub(super) fn tab_is_selected(active_path: Option<&str>, tab_path: &str) -> bool {
    active_path == Some(tab_path)
}

pub(super) fn close_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let p = theme.palette().primary;
    let hover_bg = Color::from_rgba(p.r, p.g, p.b, 0.10);
    let pressed_bg = Color::from_rgba(p.r, p.g, p.b, 0.18);
    let bg = match status {
        iced::widget::button::Status::Hovered => Some(Background::Color(hover_bg)),
        iced::widget::button::Status::Pressed => Some(Background::Color(pressed_bg)),
        _ => None,
    };

    iced::widget::button::Style {
        background: bg,
        text_color: theme.palette().text,
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 4.0.into() },
        ..Default::default()
    }
}

pub(super) fn tab_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
    selected: bool,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();
    let base = palette.background.base.color;
    let luma = 0.2126 * base.r + 0.7152 * base.g + 0.0722 * base.b;
    let is_dark = luma < 0.5;
    let primary = palette.primary.base.color;
    let hover_bg = if is_dark {
        palette.background.strong.color.scale_alpha(0.8)
    } else {
        Color::from_rgba(primary.r, primary.g, primary.b, 0.10)
    };
    let selected_bg = if is_dark {
        palette.background.strong.color.scale_alpha(0.95)
    } else {
        Color::from_rgba(primary.r, primary.g, primary.b, 0.18)
    };
    let base_bg = if selected { Some(Background::Color(selected_bg)) } else { None };
    let bg = match status {
        iced::widget::button::Status::Hovered => Some(Background::Color(hover_bg)),
        iced::widget::button::Status::Pressed => Some(Background::Color(selected_bg)),
        _ => base_bg,
    };

    iced::widget::button::Style {
        background: bg,
        text_color: theme.palette().text,
        border: iced::Border { width: 0.0, color: Color::TRANSPARENT, radius: 4.0.into() },
        ..Default::default()
    }
}

pub(super) fn tab_drag_start_message(path: &str) -> Message {
    Message::Project(message::ProjectMessage::FileTreeDragStart(path.to_string(), None))
}

pub(super) fn tab_drag_end_message() -> Message {
    Message::Project(message::ProjectMessage::FileTreeDragEnd)
}

pub(super) fn tab_close_message(path: &str) -> Message {
    Message::Preview(message::PreviewMessage::Close(path.to_string()))
}

pub(super) fn tab_select_message(path: &str) -> Message {
    Message::Preview(message::PreviewMessage::Select(path.to_string()))
}

pub(super) fn tab_right_click_message(path: &str, pos: Point) -> Message {
    Message::Preview(message::PreviewMessage::TabRightClicked(path.to_string(), pos.x, pos.y))
}

pub(super) fn tab_context_menu_matches(menu_path: Option<&str>, tab_path: &str) -> bool {
    menu_path == Some(tab_path)
}

pub(super) fn tab_menu_anchor(menu_pos: Option<Point>) -> Point {
    menu_pos.unwrap_or(Point::ORIGIN)
}

/// 构建预览面板的标签页头部组件
///
/// 该函数根据当前应用状态构建预览面板顶部的标签页条，包含所有打开的预览标签页。
/// 每个标签页支持以下功能：
/// - 点击切换到对应的预览内容
/// - 关闭按钮（X 图标）用于关闭标签页
/// - 右键菜单提供更多操作选项
/// - 拖拽功能支持将文件拖拽到其他位置
/// - 选中状态的视觉区分
///
/// # 参数
///
/// * `app` - 应用状态的不可变引用，包含预览标签页列表、当前激活的预览路径等信息
///
/// # 返回值
///
/// 返回一个 `Element<Message>` 类型的 UI 元素，包含所有标签页的可滚动容器
///
/// # 示例
///
/// ```rust,ignore
/// // 在预览面板的视图中构建标签页头部
/// let header = build_header_tabs(&app);
/// // 返回的 Element 可以直接用于 iced 的 UI 布局中
/// ```
///
/// # 视觉行为说明
///
/// - 标签页高度根据是否显示滚动条自适应：有滚动条时 36px，无滚动条时 30px
/// - 标签页宽度根据内容自动收缩（Shrink）
/// - 标签页间距为 4px
/// - 选中标签页有背景色高亮
/// - 悬停和按下状态有不同的背景色反馈
/// - 标签页标题超过 28 个字符时会被截断
pub fn build_header_tabs(app: &App) -> Element<'_, Message> {
    // 判断是否需要显示滚动条，影响标签页容器的高度
    let tabs_scrollbar_visible = should_show_preview_tabs_scrollbar(app);
    // 根据滚动条可见性调整标签页头部高度：有滚动条时更高（38px），无滚动条时较矮（30px）
    let tabs_header_height = header_tabs_height(tabs_scrollbar_visible);

    // 初始化标签页行容器，设置间距和固定高度
    let mut tabs: iced::widget::Row<'_, Message, iced::Theme, iced::Renderer> =
        row![].spacing(4).height(Length::Fixed(28.0));

    // 遍历所有预览标签页，为每个标签页构建 UI 组件
    for t in &app.preview_tabs {
        // 判断当前标签页是否被选中（与激活的预览路径比较）
        let selected = tab_is_selected(app.active_preview_path.as_deref(), &t.path);

        // 拖拽开始消息：在用户开始拖拽标签页时发送
        let drag_start = tab_drag_start_message(&t.path);
        // 拖拽结束消息：在用户释放拖拽时发送
        let drag_end = tab_drag_end_message();

        // 克隆路径用于右键菜单回调（闭包捕获需要所有权）
        let path_for_menu = t.path.clone();
        // 克隆路径用于菜单显示状态检查
        let path_for_menu_check = t.path.clone();

        // 构建关闭按钮（X 图标）
        let close = {
            button(small_icon_svg(Icon::X))
                .on_press(tab_close_message(&t.path))
                .padding(2)
                .style(close_button_style)
        };

        // 截断过长的标题，最多保留 28 个字符
        let title = truncate_title(&t.title, 40);

        // 构建标签页内容：文件图标 + 标题 + 关闭按钮
        let tab_content = row![
            // 文件图标容器，固定宽度 16px
            container(file_tab_icon_svg(file_icon_for(&t.title))).width(Length::Fixed(16.0)),
            // 标题文本容器，宽度自适应收缩
            container(text(title).size(13))
                .width(Length::Fill)
                .align_y(iced::alignment::Vertical::Center),
            // 关闭按钮容器，宽度自适应收缩
            container(close).width(Length::Shrink),
        ]
        .spacing(6)
        .align_y(iced::Alignment::Center)
        .width(Length::Fill);

        // 构建标签页按钮，包含内容容器
        let tab_button: Element<'_, Message> = button(
            container(tab_content).padding([4, 6]).height(Length::Fixed(28.0)).width(Length::Fill),
        )
        // 点击标签页时切换到对应的预览
        .on_press(tab_select_message(&t.path))
        .style(move |theme: &iced::Theme, status| tab_button_style(theme, status, selected))
        .padding(0)
        .width(Length::Fill)
        .into();

        // 将标签页按钮包装为可拖拽区域
        let draggable_tab = Element::new(DraggableArea::new(tab_button, drag_start, drag_end));

        // 将可拖拽标签页包装为支持右键点击的区域
        let right_click = Element::new(RightClickArea::new(
            draggable_tab,
            Box::new(move |pos| {
                // 右键点击时发送消息，携带文件路径和点击位置
                tab_right_click_message(&path_for_menu, pos)
            }),
        ));

        // 检查是否需要为当前标签页显示右键菜单
        let tab_item =
            if tab_context_menu_matches(app.preview_tab_menu_path.as_deref(), &path_for_menu_check)
            {
                // 如果当前标签页是菜单目标，则在标签页下方显示菜单叠加层
                PointBelowOverlay::new(right_click, build_preview_tab_menu(&path_for_menu_check))
                    .show(true)
                    .anchor(tab_menu_anchor(app.preview_tab_menu_pos))
                    .on_close(Message::Preview(message::PreviewMessage::TabMenuClose))
                    .capture_outside_click(false)
                    .into()
            } else {
                // 否则只显示标签页本身
                right_click
            };

        // 将构建好的标签页项添加到标签页行容器中
        tabs = tabs.push(tab_item);
    }

    // 创建滚动条配置，宽度为 4px
    let scrollbar = Scrollbar::new().width(4).scroller_width(4);

    // 将标签页行包装在可滚动容器中并返回
    scrollable(container(tabs).padding([0, 0]).width(Length::Fill))
        .id(app.preview_tabs_scroll_id.clone())
        .direction(Direction::Horizontal(scrollbar))
        .height(Length::Fixed(tabs_header_height))
        .into()
}
