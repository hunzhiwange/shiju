//! 文本编辑器组件的上下文菜单或滚动面板控件。
//!
//! 本模块主要负责把应用状态转换为桌面端可渲染的 Iced 控件，并把用户操作映射回上层消息。
//! 注释聚焦调用边界、返回值和错误传播方式，便于后续维护设置页与工具栏行为时快速定位职责。

use crate::app::components::overlays::PointBelowOverlay;
use crate::app::components::widgets::RightClickArea;
use iced::widget::{Space, button, column, container, operation, row, text, text_editor};
use iced::{Alignment, Background, Border, Color, Element, Length, Point, Task, Theme};
use std::sync::Arc;

#[derive(Debug, Clone, Copy, Default)]
/// `TextEditorContextMenuState` 结构体，用于表达本模块对该领域对象的建模。
///
/// 该定义保持在当前模块职责内，调用方应通过显式字段、变体或别名理解其语义。
pub struct TextEditorContextMenuState {
    pub open: bool,
    pub position: Option<(f32, f32)>,
}

#[derive(Debug, Clone)]
/// `TextEditorContextMenuMessages` 结构体，用于表达本模块对该领域对象的建模。
///
/// 该定义保持在当前模块职责内，调用方应通过显式字段、变体或别名理解其语义。
pub struct TextEditorContextMenuMessages<Message> {
    pub close: Message,
    pub undo: Option<Message>,
    pub redo: Option<Message>,
    pub copy: Message,
    pub cut: Message,
    pub paste: Message,
    pub delete: Message,
}

#[derive(Debug, Clone)]
pub struct CopyTextContextMenuMessages<Message> {
    pub close: Message,
    pub copy: Message,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// `SelectionActionOutcome` 枚举，用于表达本模块对该领域对象的建模。
///
/// 该定义保持在当前模块职责内，调用方应通过显式字段、变体或别名理解其语义。
pub enum SelectionActionOutcome {
    None,
    Copied,
    Cut,
    Deleted,
}

/// 构建或处理 `wrap_with_context_menu` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn wrap_with_context_menu<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    menu_state: TextEditorContextMenuState,
    on_open: impl Fn(Point) -> Message + 'a,
    messages: TextEditorContextMenuMessages<Message>,
) -> Element<'a, Message> {
    let content: Element<'a, Message> =
        RightClickArea::new(content.into(), Box::new(on_open)).preserve_on_right_click().into();
    let anchor = menu_state.position.unwrap_or_default();
    let show = menu_state.open && menu_state.position.is_some();
    let close_message = messages.close.clone();

    PointBelowOverlay::new(content, context_menu(messages))
        .show(show)
        .anchor(Point::new(anchor.0, anchor.1))
        .gap(0.0)
        .on_close(close_message)
        .into()
}

/// 为只读文本区域提供仅包含“复制”的右键菜单。
pub fn wrap_with_copy_context_menu<'a, Message: Clone + 'a>(
    content: impl Into<Element<'a, Message>>,
    menu_state: TextEditorContextMenuState,
    on_open: impl Fn(Point) -> Message + 'a,
    messages: CopyTextContextMenuMessages<Message>,
) -> Element<'a, Message> {
    let content: Element<'a, Message> =
        RightClickArea::new(content.into(), Box::new(on_open)).preserve_on_right_click().into();
    let anchor = menu_state.position.unwrap_or_default();
    let show = menu_state.open && menu_state.position.is_some();
    let close_message = messages.close.clone();

    PointBelowOverlay::new(content, copy_context_menu(messages.copy))
        .show(show)
        .anchor(Point::new(anchor.0, anchor.1))
        .gap(0.0)
        .on_close(close_message)
        .into()
}

/// 构建或处理 `open_menu` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn open_menu(state: &mut TextEditorContextMenuState, point: Point) {
    state.open = true;
    state.position = Some((point.x, point.y));
}

/// 构建或处理 `close_menu` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn close_menu(state: &mut TextEditorContextMenuState) {
    state.open = false;
    state.position = None;
}

/// 构建或处理 `selection_copy_task` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn selection_copy_task<Message: Send + 'static>(
    editor: &text_editor::Content,
    editor_id: &iced::widget::Id,
) -> (SelectionActionOutcome, Task<Message>) {
    let selected = editor.selection().unwrap_or_default();

    if selected.is_empty() {
        (SelectionActionOutcome::None, focus_editor_task(editor_id))
    } else {
        (
            SelectionActionOutcome::Copied,
            Task::batch(vec![iced::clipboard::write(selected), focus_editor_task(editor_id)]),
        )
    }
}

/// 构建或处理 `selection_cut_task` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn selection_cut_task<Message: Send + 'static>(
    editor: &mut text_editor::Content,
    editor_id: &iced::widget::Id,
) -> (SelectionActionOutcome, Task<Message>) {
    let selected = editor.selection().unwrap_or_default();

    if selected.is_empty() {
        (SelectionActionOutcome::None, focus_editor_task(editor_id))
    } else {
        editor.perform(text_editor::Action::Edit(text_editor::Edit::Backspace));
        (
            SelectionActionOutcome::Cut,
            Task::batch(vec![iced::clipboard::write(selected), focus_editor_task(editor_id)]),
        )
    }
}

/// 构建或处理 `selection_delete_task` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn selection_delete_task<Message: Send + 'static>(
    editor: &mut text_editor::Content,
    editor_id: &iced::widget::Id,
) -> (SelectionActionOutcome, Task<Message>) {
    let selected = editor.selection().unwrap_or_default();

    if selected.is_empty() {
        (SelectionActionOutcome::None, focus_editor_task(editor_id))
    } else {
        editor.perform(text_editor::Action::Edit(text_editor::Edit::Backspace));
        (SelectionActionOutcome::Deleted, focus_editor_task(editor_id))
    }
}

/// 构建或处理 `paste_task` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn paste_task<Message: Send + 'static>(
    editor_id: &iced::widget::Id,
    on_paste: impl Fn(String) -> Message + Send + 'static,
) -> Task<Message> {
    Task::batch(vec![
        iced::clipboard::read().map(move |content| on_paste(content.unwrap_or_default())),
        focus_editor_task(editor_id),
    ])
}

/// 构建或处理 `focus_editor_task` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn focus_editor_task<Message: Send + 'static>(editor_id: &iced::widget::Id) -> Task<Message> {
    operation::focus(editor_id.clone())
}

/// 构建或处理 `paste_action` 对应的界面片段与交互数据。
///
/// # 参数
///
/// 参数来自调用方持有的应用状态、配置快照或控件输入，用于保持渲染结果与当前状态同步。
///
/// # 返回值
///
/// 返回本函数生成的状态、样式或辅助值，供同一流程继续组合使用。
///
/// # 错误处理
///
/// 本函数不直接返回错误；无法交互或缺省状态会在控件状态中显式表达。
pub fn paste_action(content: String) -> text_editor::Action {
    text_editor::Action::Edit(text_editor::Edit::Paste(Arc::new(content)))
}

pub(super) fn context_menu<'a, Message: Clone + 'a>(
    messages: TextEditorContextMenuMessages<Message>,
) -> Element<'a, Message> {
    let menu_button = |label: &'static str, shortcut: &'static str, message: Message| {
        button(
            row![
                text(label).size(12),
                Space::new().width(Length::Fill),
                text(shortcut).size(11).style(shortcut_text_style),
            ]
            .align_y(Alignment::Center),
        )
            .width(Length::Fill)
            .style(menu_button_style)
            .on_press(message)
    };
    let mut actions = column![
        menu_button("复制选择", copy_shortcut(), messages.copy),
        menu_button("剪切", cut_shortcut(), messages.cut),
        menu_button("粘贴", paste_shortcut(), messages.paste),
        menu_button("删除", "Del", messages.delete),
    ]
    .spacing(4)
    .width(Length::Fixed(176.0));
    if let Some(message) = messages.undo {
        actions = actions.push(menu_button("撤销", undo_shortcut(), message));
    }
    if let Some(message) = messages.redo {
        actions = actions.push(menu_button("重做", redo_shortcut(), message));
    }

    container(actions)
    .padding(6)
    .style(popover_style)
    .into()
}

fn copy_context_menu<'a, Message: Clone + 'a>(copy: Message) -> Element<'a, Message> {
    container(
        button(row![
            text("复制").size(12),
            Space::new().width(Length::Fill),
            text(copy_shortcut()).size(11).style(shortcut_text_style),
        ])
            .width(Length::Fixed(144.0))
            .style(menu_button_style)
            .on_press(copy),
    )
    .padding(6)
    .style(popover_style)
    .into()
}

#[cfg(target_os = "macos")]
fn copy_shortcut() -> &'static str {
    "⌘C"
}

#[cfg(not(target_os = "macos"))]
fn copy_shortcut() -> &'static str {
    "Ctrl+C"
}

#[cfg(target_os = "macos")]
fn cut_shortcut() -> &'static str {
    "⌘X"
}

#[cfg(not(target_os = "macos"))]
fn cut_shortcut() -> &'static str {
    "Ctrl+X"
}

#[cfg(target_os = "macos")]
fn paste_shortcut() -> &'static str {
    "⌘V"
}

#[cfg(not(target_os = "macos"))]
fn paste_shortcut() -> &'static str {
    "Ctrl+V"
}

#[cfg(target_os = "macos")]
fn undo_shortcut() -> &'static str {
    "⌘Z"
}

#[cfg(not(target_os = "macos"))]
fn undo_shortcut() -> &'static str {
    "Ctrl+Z"
}

#[cfg(target_os = "macos")]
fn redo_shortcut() -> &'static str {
    "⇧⌘Z"
}

#[cfg(not(target_os = "macos"))]
fn redo_shortcut() -> &'static str {
    "Ctrl+Y"
}

fn shortcut_text_style(theme: &Theme) -> iced::widget::text::Style {
    iced::widget::text::Style { color: Some(theme.palette().text.scale_alpha(0.55)) }
}

pub(super) fn popover_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let background =
        if is_dark { palette.background.weak.color } else { Color::from_rgb8(0xF3, 0xF4, 0xF6) };
    let border_color = if is_dark {
        palette.background.strong.color
    } else {
        Color::from_rgba8(0x00, 0x00, 0x00, 0.10)
    };

    iced::widget::container::Style {
        background: Some(Background::Color(background)),
        border: Border { radius: 8.0.into(), width: 1.0, color: border_color },
        ..Default::default()
    }
}

pub(super) fn menu_button_style(theme: &Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let is_dark = is_dark_theme(theme);
    let hover = if is_dark {
        palette.background.strong.color.scale_alpha(0.65)
    } else {
        Color::from_rgba8(0x00, 0x00, 0x00, 0.06)
    };
    let pressed = if is_dark {
        theme.palette().primary.scale_alpha(0.28)
    } else {
        theme.palette().primary.scale_alpha(0.14)
    };

    let background = match status {
        button::Status::Hovered => Some(Background::Color(hover)),
        button::Status::Pressed => Some(Background::Color(pressed)),
        _ => None,
    };

    button::Style {
        background,
        text_color: theme.palette().text,
        border: Border { radius: 6.0.into(), width: 0.0, color: Color::TRANSPARENT },
        ..Default::default()
    }
}

pub(super) fn is_dark_theme(theme: &Theme) -> bool {
    let background = theme.palette().background;
    background.r + background.g + background.b < 1.5
}
