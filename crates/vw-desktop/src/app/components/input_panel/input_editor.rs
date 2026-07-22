//! 输入编辑器组件模块
//!
//! 本模块负责构建和管理聊天输入界面的文本编辑器组件。它基于 Iced 框架的 `text_editor` 组件，
//! 提供了智能的键盘绑定、自适应高度调整、文件搜索导航以及 @ 提及高亮等功能。
//!
//! # 主要功能
//!
//! - **自适应高度**：根据输入内容行数自动调整编辑器高度（3-8 行）
//! - **智能键盘绑定**：支持普通输入模式和文件搜索模式的不同键盘行为
//! - **文件搜索导航**：在文件搜索激活时，支持上下箭头、Enter、Tab 和 Escape 键导航
//! - **快捷发送**：在普通模式下按 Enter 键发送消息（不按任何修饰键）
//! - **提及高亮**：使用自定义高亮器识别和格式化 @ 提及
//!
//! # 使用示例
//!
//! ```rust,ignore
//! use crate::app::components::input_panel::input_editor::build_input_editor;
//!
//! let (input_element, height) = build_input_editor(&app, &content, false, false);
//! ```

#[cfg(target_arch = "wasm32")]
use iced::widget::mouse_area;
use iced::widget::{button, column, container, text, text_editor};
use iced::{Background, Border, Color, Element, Length, Theme};

use crate::app::components::input_mention_highlighter::{
    MentionHighlighter, Settings as MentionHighlightSettings, mention_format,
};
use crate::app::components::input_panel::styles::editor_style;
use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::widgets::RightClickArea;
use crate::app::{App, Message, message};

/// 输入框最小显示行数
///
/// 即使输入内容为空或只有一行，编辑器也会至少显示 3 行高度，
/// 以提供舒适的输入空间。
pub const INPUT_MIN_LINES: usize = 3;

/// 输入框最大显示行数
///
/// 当输入内容超过此行数时，编辑器高度不再增加，内容将在内部滚动显示。
/// 限制最大高度可以避免输入框占用过多界面空间。
pub const INPUT_MAX_LINES: usize = 8;

/// 单行文本的高度（像素）
///
/// 用于计算编辑器的总高度。该值应与字体大小和行间距相匹配。
pub const INPUT_LINE_HEIGHT: f32 = 20.0;

/// 输入框垂直方向的内边距（像素）
///
/// 应用于编辑器内容区域的顶部和底部内边距，确保文本不会紧贴边缘。
pub const INPUT_VERTICAL_PADDING: f32 = 4.0;
pub const INPUT_HORIZONTAL_PADDING: f32 = 16.0;

#[cfg(target_arch = "wasm32")]
pub(super) fn binding_from_key_press(
    app: &App,
    kp: iced::widget::text_editor::KeyPress,
) -> Option<iced::widget::text_editor::Binding<Message>> {
    if app.show_file_search {
        match kp.key {
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
                    message::ChatMessage::FileSearchNavigateUp,
                )))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
                    message::ChatMessage::FileSearchNavigateDown,
                )))
            }
            // WASM 下保留 Enter 给浏览器 IME，避免中文候选确认被抢走。
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) => {
                Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
                    message::ChatMessage::FileSearchSelectCurrent,
                )))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
                    message::ChatMessage::FileSearchInputChanged(String::new()),
                )))
            }
            _ => iced::widget::text_editor::Binding::from_key_press(kp),
        }
    } else if matches!(&kp.key, iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("v"))
        && (kp.modifiers.control() || kp.modifiers.command())
        && !kp.modifiers.alt()
    {
        Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
            message::ChatMessage::PasteIntoInput,
        )))
    } else if matches!(kp.key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter))
        && (kp.modifiers.control() || kp.modifiers.command())
    {
        Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
            message::ChatMessage::SendPressed,
        )))
    } else {
        iced::widget::text_editor::Binding::from_key_press(kp)
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn binding_from_key_press(
    app: &App,
    kp: iced::widget::text_editor::KeyPress,
) -> Option<iced::widget::text_editor::Binding<Message>> {
    if app.show_file_search {
        match kp.key {
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowUp) => {
                Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
                    message::ChatMessage::FileSearchNavigateUp,
                )))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::ArrowDown) => {
                Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
                    message::ChatMessage::FileSearchNavigateDown,
                )))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter) => {
                if !kp.modifiers.shift()
                    && !kp.modifiers.control()
                    && !kp.modifiers.alt()
                    && !kp.modifiers.command()
                {
                    Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
                        message::ChatMessage::FileSearchSelectCurrent,
                    )))
                } else {
                    iced::widget::text_editor::Binding::from_key_press(kp)
                }
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Tab) => {
                Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
                    message::ChatMessage::FileSearchSelectCurrent,
                )))
            }
            iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) => {
                Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
                    message::ChatMessage::FileSearchInputChanged(String::new()),
                )))
            }
            _ => iced::widget::text_editor::Binding::from_key_press(kp),
        }
    } else if matches!(&kp.key, iced::keyboard::Key::Character(c) if c.eq_ignore_ascii_case("v"))
        && (kp.modifiers.control() || kp.modifiers.command())
        && !kp.modifiers.alt()
    {
        Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
            message::ChatMessage::PasteIntoInput,
        )))
    } else if matches!(kp.key, iced::keyboard::Key::Named(iced::keyboard::key::Named::Enter))
        && !kp.modifiers.shift()
        && !kp.modifiers.control()
        && !kp.modifiers.alt()
        && !kp.modifiers.command()
    {
        Some(iced::widget::text_editor::Binding::Custom(Message::Chat(
            message::ChatMessage::SendPressed,
        )))
    } else {
        iced::widget::text_editor::Binding::from_key_press(kp)
    }
}

/// 构建输入编辑器组件
///
/// 创建一个配置完整的文本编辑器组件，包含自适应高度、智能键盘绑定、
/// 样式设置和提及高亮功能。
///
/// # 参数
///
/// * `app` - 应用状态引用，用于访问输入编辑器 ID 和文件搜索状态
/// * `input_editor` - 文本编辑器内容引用，包含当前输入的文本
/// * `requesting` - 是否正在请求中（用于样式变化，如显示加载状态）
/// * `_task_mode_enabled` - 任务模式是否启用（当前未使用，保留用于未来扩展）
///
/// # 返回值
///
/// 返回一个元组 `(Element<'a, Message>, f32)`：
/// - `Element<'a, Message>` - 构建好的编辑器 UI 元素
/// - `f32` - 编辑器的计算高度（像素）
///
/// # 键盘行为
///
/// ## 文件搜索模式（`show_file_search` 为 true）
///
/// - `↑` / `↓`：在搜索结果中上下导航
/// - `Enter` / `Tab`：选择当前高亮的文件
/// - `Escape`：关闭文件搜索
/// - 其他按键：使用默认编辑器行为
///
/// ## 普通输入模式
///
/// - `Enter`（无修饰键）：发送消息
/// - `Enter` + 任意修饰键（Shift/Ctrl/Alt/Cmd）：换行
/// - 其他按键：使用默认编辑器行为
///
/// # 示例
///
/// ```rust,ignore
/// // 在组件视图中构建输入编辑器
/// let (input_element, editor_height) = build_input_editor(
///     &self.app,
///     &self.input_editor,
///     self.is_requesting,
///     self.task_mode_enabled,
/// );
///
/// // 使用返回的元素和高度
/// column![input_element].height(Length::Fixed(editor_height))
/// ```
pub fn build_input_editor<'a>(
    app: &'a App,
    input_editor: &'a text_editor::Content,
    requesting: bool,
    _task_mode_enabled: bool,
) -> (Element<'a, Message>, f32) {
    // 根据文本行数计算编辑器行数，限制在 [INPUT_MIN_LINES, INPUT_MAX_LINES] 范围内
    let input_line_count =
        input_editor.text().split('\n').count().clamp(INPUT_MIN_LINES, INPUT_MAX_LINES) as f32;

    // 计算编辑器总高度：行数 * 单行高度 + 上下内边距
    let editor_height = input_line_count * INPUT_LINE_HEIGHT + (INPUT_VERTICAL_PADDING * 2.0);

    // 构建文本编辑器组件
    let editor = text_editor(input_editor)
        .id(app.input_editor_id.clone())
        .placeholder("随便问点什么...")
        .on_action(|a| Message::Chat(message::ChatMessage::InputEditorAction(a)))
        .key_binding(move |kp| binding_from_key_press(app, kp))
        .size(14.0)
        // 设置编辑器内边距：上下压缩，左右留出更舒适的输入空间。
        .padding(iced::Padding {
            top: INPUT_VERTICAL_PADDING,
            right: INPUT_HORIZONTAL_PADDING,
            bottom: INPUT_VERTICAL_PADDING,
            left: INPUT_HORIZONTAL_PADDING,
        })
        .height(Length::Fixed(editor_height))
        // 应用编辑器样式：根据请求状态调整外观
        .style(move |theme, status| editor_style(theme, status, requesting))
        // 启用 @ 提及高亮功能
        .highlight_with::<MentionHighlighter>(MentionHighlightSettings, mention_format);

    // 将编辑器包装在容器中，宽度填充父容器
    let input: Element<'a, Message> = RightClickArea::new(
        editor.into(),
        Box::new(|point| {
            Message::Chat(message::ChatMessage::OpenInputContextMenu { x: point.x, y: point.y })
        }),
    )
    .into();

    #[cfg(target_arch = "wasm32")]
    let input: Element<'a, Message> =
        mouse_area(input).on_press(Message::Chat(message::ChatMessage::WasmImeFocus)).into();

    let input = if let Some((x, y)) = app.input_context_menu_pos {
        if app.input_context_menu_open {
            PointBelowOverlay::new(input, input_context_menu())
                .show(true)
                .anchor(iced::Point::new(x, y))
                .gap(0.0)
                .on_close(Message::Chat(message::ChatMessage::CloseInputContextMenu))
                .into()
        } else {
            input
        }
    } else {
        input
    };

    let input = container(input).width(Length::Fill).into();

    (input, editor_height)
}

pub(super) fn input_context_menu<'a>() -> Element<'a, Message> {
    let menu_item = |label: &'static str, message: Message| {
        button(
            container(text(label).size(12))
                .width(Length::Fill)
                .padding([5, 9])
                .align_x(iced::alignment::Horizontal::Left),
        )
        .width(Length::Fill)
        .padding(0)
        .style(input_context_menu_button_style)
        .on_press(message)
    };

    container(
        column![
            menu_item("复制", Message::Chat(message::ChatMessage::CopyInputSelection)),
            menu_item("剪切", Message::Chat(message::ChatMessage::CutInputSelection)),
            menu_item("粘贴", Message::Chat(message::ChatMessage::PasteIntoInput)),
            menu_item("全选", Message::Chat(message::ChatMessage::SelectAllInput)),
        ]
        .spacing(0)
        .width(Length::Fixed(134.0)),
    )
    .padding([3, 3])
    .style(input_context_menu_style)
    .into()
}

pub(super) fn input_context_menu_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
) -> iced::widget::button::Style {
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    let hovered_bg = if is_dark {
        Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.05)
    } else {
        Color::from_rgba8(0xEA, 0xED, 0xF1, 0.85)
    };
    let pressed_bg = if is_dark {
        Color::from_rgba8(0xFF, 0xFF, 0xFF, 0.09)
    } else {
        Color::from_rgba8(0xE2, 0xE6, 0xEB, 0.95)
    };

    iced::widget::button::Style {
        background: match status {
            iced::widget::button::Status::Pressed => Some(Background::Color(pressed_bg)),
            iced::widget::button::Status::Hovered => Some(Background::Color(hovered_bg)),
            _ => None,
        },
        border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 5.0.into() },
        text_color: theme.palette().text,
        ..Default::default()
    }
}

pub(super) fn input_context_menu_style(theme: &Theme) -> iced::widget::container::Style {
    let is_dark =
        theme.palette().background.r + theme.palette().background.g + theme.palette().background.b
            < 1.5;
    let background = if is_dark {
        Color::from_rgba8(0x22, 0x22, 0x24, 0.98)
    } else {
        Color::from_rgba8(0xF3, 0xF4, 0xF6, 0.98)
    };
    let border = if is_dark {
        Color::from_rgba8(0x3A, 0x3A, 0x3F, 0.95)
    } else {
        Color::from_rgba8(0xDD, 0xE1, 0xE6, 1.0)
    };

    iced::widget::container::Style {
        background: Some(Background::Color(background)),
        border: Border { width: 1.0, color: border, radius: 8.0.into() },
        shadow: iced::Shadow {
            color: Color::BLACK.scale_alpha(if is_dark { 0.20 } else { 0.08 }),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    }
}
