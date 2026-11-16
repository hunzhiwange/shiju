//! 输入面板内容布局；跨领域子视图由桌面装配。
use iced::widget::{Space, column};
use iced::{Element, Length};

pub struct Content<'a, Message> {
    pub attachment_strip: Element<'a, Message>,
    pub file_refs: Element<'a, Message>,
    pub task_mode_form: Element<'a, Message>,
    pub input: Element<'a, Message>,
    pub divider: Element<'a, Message>,
    pub bottom_bar: Element<'a, Message>,
    pub context_card: Option<Element<'a, Message>>,
    pub todo_panel: Option<Element<'a, Message>>,
    pub queue_panel: Option<Element<'a, Message>>,
}

pub fn content<'a, Message: 'a>(parts: Content<'a, Message>) -> Element<'a, Message> {
    let Content {
        attachment_strip,
        file_refs,
        task_mode_form,
        input,
        divider,
        bottom_bar,
        context_card,
        todo_panel,
        queue_panel,
    } = parts;
    if let Some(todo_panel) = todo_panel {
        // 有待办事项时，将 TODO 面板紧贴输入框上方
        let mut input_block = column![todo_panel].spacing(0);
        if let Some(context_card) = context_card {
            input_block = input_block.push(context_card);
        }
        input_block = input_block.push(input).push(divider).push(bottom_bar);

        column![attachment_strip, file_refs, task_mode_form, input_block].spacing(3)
    } else if let Some(queue_panel) = queue_panel {
        // 队列布局：队列面板 + 标准内容
        let mut queued = column![
            queue_panel,
            Space::new().height(Length::Fixed(4.0)),
            attachment_strip,
            file_refs,
            task_mode_form
        ]
        .spacing(6);
        if let Some(context_card) = context_card {
            queued = queued.push(context_card);
        }
        queued.push(input).push(divider).push(bottom_bar)
    } else {
        // 标准布局：文件引用 + 任务模式表单 + 输入框 + 底部栏
        let mut standard = column![attachment_strip, file_refs, task_mode_form].spacing(6);
        if let Some(context_card) = context_card {
            standard = standard.push(context_card);
        }
        standard.push(input).push(divider).push(bottom_bar)
    }
    .into()
}

/// 输入面板外壳所需的局部快照及跨领域子视图。
pub struct Surface<'a, Message> {
    pub content: Element<'a, Message>,
    pub file_search: Element<'a, Message>,
    pub slash_palette: Element<'a, Message>,
    pub show_file_search: bool,
    pub show_slash_palette: bool,
    pub dragging: bool,
    pub input_drop_hovered: bool,
    pub chrome: desktop_theme::ChromeStyleOverrides,
    pub background: Option<&'a std::path::Path>,
    pub fit: desktop_theme::PanelImageFit,
    pub voice: Option<Element<'a, Message>>,
    pub agent_sessions: Option<Element<'a, Message>>,
    pub quick_messages: Option<Element<'a, Message>>,
}

pub struct SurfaceMessages<Message> {
    pub close_file_search: Message,
    pub close_slash_palette: Message,
    pub drop: Message,
    pub hover: (Message, Message),
    pub native_drop: fn(std::path::PathBuf) -> Message,
}

pub fn surface<'a, Message: Clone + 'a>(
    parts: Surface<'a, Message>,
    messages: SurfaceMessages<Message>,
) -> Element<'a, Message> {
    use desktop_ui_controls::overlays::AboveOverlay;
    use iced::widget::container;
    let content = AboveOverlay::new(parts.content, parts.file_search)
        .show(parts.show_file_search)
        .gap(6.0)
        .on_close(messages.close_file_search);
    let content = AboveOverlay::new(content, parts.slash_palette)
        .show(parts.show_slash_palette)
        .gap(6.0)
        .on_close(messages.close_slash_palette);
    let drop_area = super::drop_area::DropArea::new(
        content.into(),
        messages.drop,
        Some(messages.hover),
        parts.dragging,
    )
    .on_native_drop(messages.native_drop);
    let hovered = parts.input_drop_hovered && parts.dragging;
    let chrome = parts.chrome;
    let card = container(Element::new(drop_area))
        .style(move |theme| super::surface_styles::input_card_style(theme, hovered, Some(&chrome)))
        .padding(0)
        .width(Length::Fill);
    let card = desktop_theme::with_panel_background(card.into(), parts.background, parts.fit);
    let mut panel = column![].spacing(6).width(Length::Fill);
    if let Some(voice) = parts.voice {
        panel = panel.push(voice);
    }
    if let Some(entry) = parts.agent_sessions {
        panel = panel.push(entry);
    }
    if let Some(quick_messages) = parts.quick_messages {
        panel = panel.push(quick_messages);
    }
    desktop_chat_view::panel::conversation_reading_column(
        panel.push(card),
        iced::Padding { top: 0.0, right: 2.0, bottom: 0.0, left: 2.0 },
    )
    .into()
}
