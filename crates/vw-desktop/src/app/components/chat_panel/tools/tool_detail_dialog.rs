//! 工具详情弹窗。
//!
//! 本模块构建工具输入输出详情的编辑器面板，并通过滚轮拦截避免弹窗滚动影响底层聊天列表。

use crate::app::components::system_settings_common::{
    settings_close_button, settings_modal_card, settings_modal_overlay, settings_muted_text_style,
    settings_panel_style,
};
/// 重新导出 use crate::app::components::text_editor_context_menu::{，让上层模块通过稳定路径访问。
use crate::app::components::text_editor_context_menu::{
    TextEditorContextMenuMessages, TextEditorContextMenuState, wrap_with_context_menu,
};
/// 重新导出 use crate::app::{App, Message, message}，让上层模块通过稳定路径访问。
use crate::app::{App, Message, message};
/// 重新导出 use iced::advanced::layout，让上层模块通过稳定路径访问。
use iced::advanced::layout;
/// 重新导出 use iced::advanced::overlay，让上层模块通过稳定路径访问。
use iced::advanced::overlay;
/// 重新导出 use iced::advanced::renderer，让上层模块通过稳定路径访问。
use iced::advanced::renderer;
/// 重新导出 use iced::advanced::widget::{Operation, Tree}，让上层模块通过稳定路径访问。
use iced::advanced::widget::{Operation, Tree};
/// 重新导出 use iced::advanced::{Clipboard, Layout, Shell, Widget}，让上层模块通过稳定路径访问。
use iced::advanced::{Clipboard, Layout, Shell, Widget};
/// 重新导出 use iced::mouse，让上层模块通过稳定路径访问。
use iced::mouse;
/// 重新导出 use iced::widget::slider::Rail，让上层模块通过稳定路径访问。
use iced::widget::slider::Rail;
/// 重新导出 use iced::widget::{，让上层模块通过稳定路径访问。
use iced::widget::{Space, column, container, responsive, row, text, text_editor, vertical_slider};
/// 重新导出 use iced::{，让上层模块通过稳定路径访问。
use iced::{Border, Color, Element, Event, Length, Rectangle, Size, Theme, Vector};

/// 处理 tool detail dialog view 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// `None` 表示输入缺少必要字段、当前状态不匹配，或该视图片段不需要展示。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub fn tool_detail_dialog_view<'a>(app: &'a App) -> Option<Element<'a, Message>> {
    let dialog = app.tool_detail_dialog.as_ref()?;
    let close_message = Message::Chat(message::ChatMessage::CloseToolDetail);

    let title = text(&dialog.title).size(14).style(|theme: &Theme| iced::widget::text::Style {
        // color 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        color: Some(theme.extended_palette().background.base.text.scale_alpha(0.95)),
    });

    let panel = settings_modal_card(
        column![
            row![
                column![
                    title,
                    text("查看工具输出详情。").size(12).style(settings_muted_text_style),
                ]
                .spacing(4)
                .width(Length::Fill),
                settings_close_button(close_message.clone()),
            ]
            .align_y(iced::Alignment::Start)
            .spacing(8),
            responsive(move |size| build_editor_panel(app, size)).height(Length::Fill),
        ]
        .spacing(14),
    )
    .width(Length::Fixed(760.0))
    .height(Length::Fixed(560.0));

    Some(settings_modal_overlay(None, close_message, panel))
}

/// 构建 editor panel 对应的 Iced 界面片段或中间数据。
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
pub(super) fn build_editor_panel<'a>(app: &'a App, size: Size) -> Element<'a, Message> {
    let Some(dialog) = app.tool_detail_dialog.as_ref() else {
        return container(Space::new().width(Length::Fill).height(Length::Fill)).into();
    };

    let scroll_state = tool_detail_scroll_state(
        size,
        app.current_line_height,
        dialog.editor.line_count(),
        dialog.scroll_top_line,
    );
    let viewport_height = scroll_state.viewport_height;
    let max_scroll = scroll_state.max_scroll;
    let scroll_top_line = scroll_state.scroll_top_line;

    let editor = text_editor(&dialog.editor)
        .id(dialog.editor_id.clone())
        .on_action(|action| Message::Chat(message::ChatMessage::ToolDetailEditorAction(action)))
        .size(14.0)
        .padding([8, 10])
        .line_height(crate::app::components::chat_panel::tool_text_support::chat_text_line_height())
        .height(Length::Fill)
        .style(|theme: &Theme, _status| {
            let palette = theme.extended_palette();

            text_editor::Style {
                // background 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                background: palette.background.base.color.into(),
                // border 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                border: Border { width: 0.0, color: Color::TRANSPARENT, radius: 0.0.into() },
                // value 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                value: theme.palette().text,
                // selection 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                selection: theme.palette().primary.scale_alpha(0.30),
                // placeholder 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                placeholder: theme.palette().text.scale_alpha(0.55),
            }
        });

    let editor = wheel_interceptor(editor, move |delta| {
        // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        Message::Chat(message::ChatMessage::ToolDetailEditorWheelScrolled {
            delta,
            viewport_height,
        })
    });

    let editor = wrap_with_context_menu(
        editor,
        TextEditorContextMenuState {
            // open 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            open: dialog.context_menu_open,
            // position 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            position: dialog.context_menu_pos,
        },
        |point| {
            // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            Message::Chat(message::ChatMessage::ToolDetailOpenContextMenu {
                // x 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                x: point.x,
                // y 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                y: point.y,
            })
        },
        TextEditorContextMenuMessages {
            undo: None,
            redo: None,
            // close 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            close: Message::Chat(message::ChatMessage::ToolDetailCloseContextMenu),
            // copy 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            copy: Message::Chat(message::ChatMessage::ToolDetailContextMenuCopy),
            // cut 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            cut: Message::Chat(message::ChatMessage::ToolDetailContextMenuCut),
            // paste 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            paste: Message::Chat(message::ChatMessage::ToolDetailContextMenuPaste),
            // delete 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            delete: Message::Chat(message::ChatMessage::ToolDetailContextMenuDelete),
        },
    );

    let mut body = row![container(editor).width(Length::Fill).height(Length::Fill)];

    if max_scroll > 0.0 {
        let slider =
            vertical_slider(0.0..=max_scroll, max_scroll - scroll_top_line, move |value| {
                // Message 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                Message::Chat(message::ChatMessage::ToolDetailScrollbarChanged {
                    // top_line 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
                    top_line: max_scroll - value,
                    viewport_height,
                })
            })
            .step(1.0)
            .width(10)
            .height(Length::Fill)
            .style(tool_detail_scrollbar_style);

        body = body.push(container(slider).width(Length::Fixed(10.0)).height(Length::Fill));
    }

    container(body.spacing(8).height(Length::Fill))
        .padding(12)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(tool_detail_panel_style)
        .into()
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ToolDetailScrollState {
    pub viewport_height: f32,
    pub max_scroll: f32,
    pub scroll_top_line: f32,
}

pub(super) fn tool_detail_scroll_state(
    size: Size,
    line_height: f32,
    line_count: usize,
    scroll_top_line: f32,
) -> ToolDetailScrollState {
    let viewport_height = (size.height - 24.0).max(1.0);
    let line_height = line_height.max(1.0);
    let total_lines = line_count.max(1) as f32;
    let visible_lines = (viewport_height / line_height).floor().max(1.0);
    let max_scroll = (total_lines - visible_lines).max(0.0);
    let scroll_top_line = scroll_top_line.clamp(0.0, max_scroll);

    ToolDetailScrollState { viewport_height, max_scroll, scroll_top_line }
}

pub(super) fn tool_detail_scrollbar_style(
    theme: &Theme,
    status: iced::widget::vertical_slider::Status,
) -> iced::widget::vertical_slider::Style {
    let palette = theme.extended_palette();
    let thumb = match status {
        iced::widget::vertical_slider::Status::Active => {
            palette.background.strong.color.scale_alpha(0.85)
        }
        iced::widget::vertical_slider::Status::Hovered => theme.palette().primary.scale_alpha(0.75),
        iced::widget::vertical_slider::Status::Dragged => theme.palette().primary,
    };

    iced::widget::vertical_slider::Style {
        rail: Rail {
            backgrounds: (
                palette.background.weak.color.scale_alpha(0.30).into(),
                palette.background.weak.color.scale_alpha(0.30).into(),
            ),
            width: 4.0,
            border: Border { radius: 999.0.into(), width: 0.0, color: Color::TRANSPARENT },
        },
        handle: iced::widget::vertical_slider::Handle {
            shape: iced::widget::vertical_slider::HandleShape::Rectangle {
                width: 8,
                border_radius: 999.0.into(),
            },
            background: thumb.into(),
            border_width: 0.0,
            border_color: Color::TRANSPARENT,
        },
    }
}

pub(super) fn tool_detail_panel_style(theme: &Theme) -> iced::widget::container::Style {
    let mut style = settings_panel_style(theme);
    style.border.radius = 18.0.into();
    style
}

/// 处理 wheel interceptor 对应的局部职责。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
pub(super) fn wheel_interceptor<'a, Message, ThemeT, RendererT>(
    content: impl Into<Element<'a, Message, ThemeT, RendererT>>,
    // on_scroll 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    on_scroll: impl Fn(mouse::ScrollDelta) -> Message + 'a,
) -> Element<'a, Message, ThemeT, RendererT>
where
    Message: 'a,
    ThemeT: 'a,
    RendererT: iced::advanced::Renderer + 'a,
{
    // Element 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    Element::new(WheelInterceptor { content: content.into(), on_scroll: Box::new(on_scroll) })
}

/// WheelInterceptor 保存 tool_detail_dialog 模块需要跨函数传递的状态。
///
/// 字段保持贴近调用方的真实数据，避免在 UI 边界处隐藏额外转换。
pub(super) struct WheelInterceptor<'a, Message, ThemeT, RendererT> {
    // content 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) content: Element<'a, Message, ThemeT, RendererT>,
    // on_scroll 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
    pub(super) on_scroll: Box<dyn Fn(mouse::ScrollDelta) -> Message + 'a>,
}

impl<Message, ThemeT, RendererT> Widget<Message, ThemeT, RendererT>
    for WheelInterceptor<'_, Message, ThemeT, RendererT>
where
    RendererT: iced::advanced::Renderer,
{
    /// 处理 children 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回集合保持输入顺序或界面展示顺序，空集合表示没有可展示项。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn children(&self) -> Vec<Tree> {
        vec![Tree::new(&self.content)]
    }

    /// 处理 diff 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn diff(&self, tree: &mut Tree) {
        tree.diff_children(std::slice::from_ref(&self.content));
    }

    /// 处理 size 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 返回值保持当前模块的领域语义，供相邻视图或测试继续使用。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn size(&self) -> Size<Length> {
        self.content.as_widget().size()
    }

    /// 处理 layout 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn layout(
        &mut self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &mut Tree,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // limits 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        limits: &layout::Limits,
    ) -> layout::Node {
        self.content.as_widget_mut().layout(&mut tree.children[0], renderer, limits)
    }

    /// 处理 operate 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn operate(
        &mut self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &mut Tree,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // operation 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        operation: &mut dyn Operation,
    ) {
        self.content.as_widget_mut().operate(&mut tree.children[0], layout, renderer, operation);
    }

    /// 处理 update 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn update(
        &mut self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &mut Tree,
        // event 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        event: &Event,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // clipboard 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        clipboard: &mut dyn Clipboard,
        // shell 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        shell: &mut Shell<'_, Message>,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        viewport: &Rectangle,
    ) {
        if cursor.is_over(layout.bounds())
            && let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event
        {
            shell.publish((self.on_scroll)(*delta));
            shell.capture_event();
            return;
        }

        self.content.as_widget_mut().update(
            &mut tree.children[0],
            event,
            layout,
            cursor,
            renderer,
            clipboard,
            shell,
            viewport,
        );
    }

    /// 处理 draw 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn draw(
        &self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &Tree,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &mut RendererT,
        // theme 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        theme: &ThemeT,
        // style 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        style: &renderer::Style,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        viewport: &Rectangle,
    ) {
        self.content.as_widget().draw(
            &tree.children[0],
            renderer,
            theme,
            style,
            layout,
            cursor,
            viewport,
        );
    }

    /// 处理 mouse interaction 对应的局部职责。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn mouse_interaction(
        &self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &Tree,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'_>,
        // cursor 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        cursor: mouse::Cursor,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        viewport: &Rectangle,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
    ) -> mouse::Interaction {
        self.content.as_widget().mouse_interaction(
            &tree.children[0],
            layout,
            cursor,
            viewport,
            renderer,
        )
    }

    /// 构建或定位 overlay，用于把浮层稳定附着到目标控件。
    ///
    /// # 参数
    ///
    /// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
    ///
    /// # 返回值
    ///
    /// 无返回值时，函数通过发布消息或更新局部状态完成交互。
    ///
    /// # 错误处理
    ///
    /// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
    fn overlay<'b>(
        &'b mut self,
        // tree 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        tree: &'b mut Tree,
        // layout 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        layout: Layout<'b>,
        // renderer 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        renderer: &RendererT,
        // viewport 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        viewport: &Rectangle,
        // translation 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
        translation: Vector,
    ) -> Option<overlay::Element<'b, Message, ThemeT, RendererT>> {
        self.content.as_widget_mut().overlay(
            &mut tree.children[0],
            layout,
            renderer,
            viewport,
            translation,
        )
    }
}
