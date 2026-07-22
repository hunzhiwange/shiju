//! 编辑器工具栏组件模块
//!
//! 本模块提供了编辑器工具栏相关的UI组件构建功能，包括：
//! - 图标按钮组件（带工具提示）
//! - SVG图标渲染
//! - 完整的编辑器工具栏视图
//!
//! # 主要功能
//!
//! - **保存功能**：支持保存文件，并根据文件修改状态显示不同样式
//! - **撤销/重做**：提供撤销和重做操作的快捷按钮
//! - **搜索/替换**：打开搜索和替换面板的快捷入口
//!
//! # 使用示例
//!
//! ```ignore
//! use crate::app::components::editor_toolbar;
//! use crate::app::{App, Message};
//!
//! // 在视图层创建工具栏
//! let toolbar = editor_toolbar::view(&app, Some(Message::Save), true);
//! ```

use crate::app::assets::{self, Icon};
use crate::app::{App, Message};
use iced::widget::svg::Svg;
use iced::widget::tooltip::{Position as TooltipPosition, Tooltip};
use iced::widget::{button, container, row, text};
use iced::{Background, Border, Color, Element, Length};

/// 创建固定尺寸的SVG图标组件
///
/// 该函数用于创建一个16x16像素的SVG图标，适用于工具栏按钮等场景。
///
/// # 参数
///
/// - `icon`: 图标枚举类型，指定要渲染的图标
///
/// # 返回值
///
/// 返回配置好的`Svg`组件实例，尺寸固定为16x16像素
///
/// # 示例
///
/// ```ignore
/// use crate::app::assets::Icon;
///
/// let svg_icon = icon_svg(Icon::Save);
/// ```
pub fn icon_svg(icon: Icon) -> Svg<'static> {
    Svg::new(assets::get_icon(icon)).width(Length::Fixed(16.0)).height(Length::Fixed(16.0))
}

/// 计算工具栏图标按钮的样式
pub(crate) fn icon_button_style(
    theme: &iced::Theme,
    status: iced::widget::button::Status,
    is_highlighted: bool,
) -> iced::widget::button::Style {
    let p = theme.palette().primary;
    let highlight_bg = Color::from_rgba(p.r, p.g, p.b, 0.12);
    let highlight_hover_bg = Color::from_rgba(p.r, p.g, p.b, 0.18);
    let bg = if is_highlighted {
        match status {
            iced::widget::button::Status::Hovered => highlight_hover_bg,
            iced::widget::button::Status::Pressed => highlight_hover_bg,
            _ => highlight_bg,
        }
    } else {
        match status {
            iced::widget::button::Status::Hovered => Color::from_rgba8(242, 242, 242, 1.0),
            iced::widget::button::Status::Pressed => Color::from_rgba8(236, 236, 236, 1.0),
            _ => Color::from_rgba8(250, 250, 250, 1.0),
        }
    };
    let border_color = if is_highlighted { p } else { Color::from_rgba8(225, 225, 225, 1.0) };
    iced::widget::button::Style {
        background: Some(Background::Color(bg)),
        border: Border { width: 1.0, color: border_color, radius: 8.0.into() },
        ..Default::default()
    }
}

/// 创建工具栏提示容器样式
pub(crate) fn tooltip_content_style() -> iced::widget::container::Style {
    iced::widget::container::Style {
        text_color: None,
        background: Some(Background::Color(Color::from_rgba8(245, 245, 245, 1.0))),
        border: Border {
            width: 1.0,
            color: Color::from_rgba8(210, 210, 210, 1.0),
            radius: 8.0.into(),
        },
        shadow: iced::Shadow::default(),
        snap: false,
    }
}

/// 创建带工具提示的图标按钮组件
///
/// 该函数创建一个带有悬停提示功能的图标按钮，支持高亮状态显示。
/// 按钮会根据`is_highlighted`参数和鼠标交互状态动态改变样式。
///
/// # 参数
///
/// - `icon`: 按钮显示的图标类型
/// - `tip`: 鼠标悬停时显示的提示文本
/// - `position`: 工具提示的显示位置（顶部、底部、左侧、右侧）
/// - `on`: 按钮点击时触发的消息
/// - `is_highlighted`: 是否高亮显示按钮（用于标识激活或修改状态）
///
/// # 返回值
///
/// 返回包含按钮和工具提示的`Element`组件
///
/// # 样式规则
///
/// - **高亮状态**：使用主题色的半透明背景，边框使用主题色
/// - **普通状态**：使用浅灰色背景和边框
/// - **悬停/按下**：根据状态调整背景颜色的明暗度
///
/// # 示例
///
/// ```ignore
/// use crate::app::assets::Icon;
/// use iced::widget::tooltip::Position;
///
/// let save_button = icon_button(
///     Icon::Save,
///     "保存文件",
///     Position::Bottom,
///     Message::Save,
///     true, // 文件已修改，显示高亮
/// );
/// ```
pub fn icon_button<'a>(
    icon: Icon,
    tip: &'a str,
    position: TooltipPosition,
    on: Message,
    is_highlighted: bool,
) -> Element<'a, Message> {
    let btn = button(icon_svg(icon))
        .on_press(on)
        .padding(6)
        .width(Length::Fixed(32.0))
        .height(Length::Fixed(32.0))
        .style(move |theme, status| icon_button_style(theme, status, is_highlighted));

    // 创建工具提示内容容器样式
    let tip_content =
        container(text(tip.to_string())).padding([6, 10]).style(|_theme| tooltip_content_style());

    // 组合按钮和工具提示，设置10像素的间距
    Tooltip::new(btn, tip_content, position).gap(10).into()
}

/// 构建编辑器工具栏视图
///
/// 该函数根据应用状态创建完整的编辑器工具栏，包含保存、撤销、重做、搜索和替换等功能按钮。
/// 工具栏会根据当前屏幕状态自动调整显示内容。
///
/// # 参数
///
/// - `app`: 应用状态引用，用于获取当前屏幕信息
/// - `on_save`: 可选的保存消息，`None`表示不显示保存按钮
/// - `is_dirty`: 文件是否已修改，影响保存按钮的显示样式
///
/// # 返回值
///
/// 返回工具栏的`Element`组件，包含所有配置好的工具按钮
///
/// # 工具栏内容
///
/// - **保存按钮**：仅当`on_save`参数为`Some`时显示，文件修改时高亮显示
/// - **撤销按钮**：执行撤销操作
/// - **重做按钮**：执行重做操作
/// - **搜索按钮**：打开搜索面板
/// - **替换按钮**：打开替换面板
///
/// # 屏幕适配
///
/// - **项目屏幕**：返回空工具栏（仅包含间距布局）
/// - **其他屏幕**：显示完整工具栏
///
/// # 示例
///
/// ```ignore
/// use crate::app::{App, Message};
///
/// // 创建工具栏，显示保存按钮并标记文件已修改
/// let toolbar = view(&app, Some(Message::Save), true);
///
/// // 创建工具栏，不显示保存按钮
/// let toolbar_no_save = view(&app, None, false);
/// ```
pub fn view<'a>(app: &App, on_save: Option<Message>, is_dirty: bool) -> Element<'a, Message> {
    // 如果是项目屏幕，返回空工具栏
    if matches!(app.screen, crate::app::Screen::Project) {
        return row![].spacing(6).into();
    }

    // 初始化工具栏容器，设置按钮间距为6像素
    let mut tools = row![].spacing(6);

    // 添加保存按钮（如果提供了保存消息）
    if let Some(save_msg) = on_save {
        let save_btn = icon_button(
            Icon::Save,
            if is_dirty { "保存文件 (已修改)" } else { "保存文件" },
            TooltipPosition::Bottom,
            save_msg,
            is_dirty, // 文件修改时高亮显示保存按钮
        );
        tools = tools.push(save_btn);
    }

    // 创建撤销按钮
    let undo_btn = icon_button(
        Icon::ArrowCounterClockwise,
        "撤销",
        TooltipPosition::Bottom,
        Message::Editor(crate::app::message::editor::EditorMessage::Undo),
        false,
    );

    // 创建重做按钮
    let redo_btn = icon_button(
        Icon::ArrowClockwise,
        "重做",
        TooltipPosition::Bottom,
        Message::Editor(crate::app::message::editor::EditorMessage::Redo),
        false,
    );

    // 创建搜索按钮
    let search_btn = icon_button(
        Icon::Search,
        "搜索",
        TooltipPosition::Bottom,
        Message::Editor(crate::app::message::editor::EditorMessage::OpenSearch),
        false,
    );

    // 创建替换按钮
    let replace_btn = icon_button(
        Icon::ArrowRepeat,
        "替换",
        TooltipPosition::Bottom,
        Message::Editor(crate::app::message::editor::EditorMessage::OpenReplace),
        false,
    );

    // 将所有按钮添加到工具栏
    tools = tools.push(undo_btn).push(redo_btn).push(search_btn).push(replace_btn);

    tools.into()
}
