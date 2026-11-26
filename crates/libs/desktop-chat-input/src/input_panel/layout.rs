//! 输入面板内容布局；跨领域子视图由桌面装配。
use iced::widget::{Container, column, container, row};
use iced::{Alignment, Element, Length, Padding};

pub const COMPOSER_TOOLBAR_PADDING: Padding =
    Padding { top: 3.0, right: 12.0, bottom: 6.0, left: 12.0 };

pub fn composer_toolbar<Message>(bar: iced::widget::Row<'_, Message>) -> iced::widget::Row<'_, Message> {
    bar.spacing(4).align_y(iced::Alignment::Center).padding(COMPOSER_TOOLBAR_PADDING)
}

pub fn composer_card<'a, Message: 'a>(
    content: impl Into<Element<'a, Message>>,
    hovered: bool,
    chrome: Option<desktop_theme::ChromeStyleOverrides>,
) -> Container<'a, Message> {
    container(content)
        .style(move |theme| super::surface_styles::input_card_style(theme, hovered, chrome.as_ref()))
        .padding(0)
        .width(Length::Fill)
}

pub struct Content<'a, Message> {
    pub attachment_strip: Element<'a, Message>,
    pub file_refs: Element<'a, Message>,
    pub task_mode_form: Element<'a, Message>,
    pub input: Element<'a, Message>,
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
        input_block = input_block.push(input).push(bottom_bar);

        column![attachment_strip, file_refs, task_mode_form, input_block].spacing(3)
    } else if let Some(queue_panel) = queue_panel {
        // 队列布局：队列面板 + 标准内容
        let mut queued = column![
            queue_panel,
            attachment_strip,
            file_refs,
            task_mode_form
        ]
        .spacing(6);
        if let Some(context_card) = context_card {
            queued = queued.push(context_card);
        }
        queued.push(column![input, bottom_bar])
    } else {
        // 标准布局：文件引用 + 任务模式表单 + 输入框 + 底部栏
        let mut standard = column![attachment_strip, file_refs, task_mode_form].spacing(6);
        if let Some(context_card) = context_card {
            standard = standard.push(context_card);
        }
        standard.push(column![input, bottom_bar])
    }
    .into()
}

/// 输入面板外壳所需的局部快照及跨领域子视图。
pub struct Surface<'a, Message> {
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
    pub quick_messages: Element<'a, Message>,
    pub companion: Option<Element<'a, Message>>,
    pub context: Element<'a, Message>,
}

pub struct SurfaceMessages<Message> {
    pub close_file_search: Message,
    pub close_slash_palette: Message,
    pub drop: Message,
    pub hover: (Message, Message),
    pub native_drop: fn(std::path::PathBuf) -> Message,
}

pub fn view<'a, Message: Clone + 'a>(
    content_parts: Content<'a, Message>,
    parts: Surface<'a, Message>,
    messages: SurfaceMessages<Message>,
) -> Element<'a, Message> {
    use desktop_ui_controls::overlays::AboveOverlay;
    let content = container(content(content_parts)).padding(Padding { top: 4.0, ..Padding::ZERO });
    let content = AboveOverlay::new(content, parts.file_search)
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
    let body_chrome = chrome.clone();
    let context_chrome = chrome.clone();
    let body = container(Element::new(drop_area))
        .width(Length::Fill)
        .style(move |theme| super::surface_styles::composer_body_style(theme, hovered, &body_chrome));
    let body = desktop_theme::with_panel_background(body.into(), parts.background, parts.fit);
    let context = container(parts.context)
        .width(Length::Fill)
        .style(move |theme| super::surface_styles::composer_context_style(theme, &context_chrome));
    let composer = container(column![body, context])
        .width(Length::Fill)
        .padding(super::surface_styles::COMPOSER_RIM)
        .style(move |theme| super::surface_styles::chat_composer_style(theme, hovered, &chrome));

    let companion_visible = parts.companion.is_some();
    let shortcuts = container(parts.quick_messages).width(Length::Fill).padding(Padding {
        bottom: if companion_visible { 6.0 } else { 0.0 },
        ..Padding::ZERO
    });
    let mut header = row![shortcuts]
        .spacing(12)
        .padding([0, 8])
        .align_y(Alignment::End);
    if let Some(companion) = parts.companion {
        header = header.push(companion);
    }

    let mut panel = column![].spacing(8).width(Length::Fill);
    if let Some(voice) = parts.voice {
        panel = panel.push(voice);
    }
    if let Some(entry) = parts.agent_sessions {
        panel = panel.push(entry);
    }
    panel = panel.push(
        column![header, composer]
            .spacing(if companion_visible { 0 } else { 6 })
            .width(Length::Fill),
    );
    desktop_chat_view::panel::conversation_reading_column(
        panel,
        desktop_chat_view::panel::CONVERSATION_GUTTER,
    )
    .into()
}

/// 局部控件布局；跨域子视图由宿主投影后传入。
pub struct Controls<'a, Message> {
    pub agent_tab_btn: Element<'a, Message>,
    pub skills_tab_btn: Element<'a, Message>,
    pub workflow_btn: Element<'a, Message>,
    pub model_btn: Element<'a, Message>,
    pub usage_btn: Element<'a, Message>,
    pub attach_btn: Element<'a, Message>,
    pub voice_btn: Option<Element<'a, Message>>,
    pub send: Element<'a, Message>,
    pub tools_tab_btn: Option<Element<'a, Message>>,
    pub acp_btn: Option<Element<'a, Message>>,
    pub pool_btn: Option<Element<'a, Message>>,
    pub cancel: Option<Element<'a, Message>>,
    pub mobile_model_content: Element<'a, Message>,
}
pub fn controls<'a, Message: Clone + 'a>(
    i18n: desktop_i18n::I18n,
    parts: Controls<'a, Message>,
    mobile: bool,
    expanded: bool,
    task_mode_enabled: bool,
    toggle_model: Message,
    close_popovers: Message,
) -> Element<'a, Message> {
    use desktop_ui_controls::overlays::AboveOverlay;
    use iced::widget::container;
    let Controls {
        agent_tab_btn,
        skills_tab_btn,
        workflow_btn,
        model_btn,
        usage_btn,
        attach_btn,
        voice_btn,
        send,
        tools_tab_btn,
        acp_btn,
        pool_btn,
        cancel,
        mobile_model_content,
    } = parts;
    if mobile {
        // 复用已有弹层状态与消息，布局切换不产生第二份草稿或请求状态。
        let mut options = iced::widget::row![agent_tab_btn, skills_tab_btn]
            .spacing(8)
            .align_y(iced::Alignment::Center);
        if let Some(control) = tools_tab_btn {
            options = options.push(control);
        }
        if let Some(control) = acp_btn {
            options = options.push(control);
        }
        options = options.push(workflow_btn).push(usage_btn);
        let menu =
            container(column![options.wrap(), mobile_model_content].spacing(8).width(Length::Fill))
                .padding(7)
                .width(Length::Fixed(334.0))
                .style(super::surface_styles::popover_style);
        let icon = super::icons::icon_svg(desktop_assets::assets::Icon::Sliders, 14.0)
            .style(|theme: &iced::Theme, _| iced::widget::svg::Style {
                color: Some(theme.palette().text),
            });
        let toggle = super::panel::bottom_bar_round_toggle(icon.into(), i18n.__("模型与工具"), toggle_model);
        let toggle: Element<'_, Message> = AboveOverlay::new(toggle, menu)
            .show(expanded)
            .block_background_events(true)
            .flip_if_clipped()
            .gap(6.0)
            .on_close(close_popovers)
            .into();
        let attachment = if let Some(voice_btn) = voice_btn {
            row![attach_btn, voice_btn].spacing(3).align_y(Alignment::Center).into()
        } else {
            attach_btn
        };
        super::panel::mobile_controls(toggle, attachment, pool_btn, cancel, send)
    } else {
        super::send_controls::bottom_bar(
            Some(agent_tab_btn),
            tools_tab_btn,
            Some(skills_tab_btn),
            acp_btn,
            workflow_btn,
            model_btn,
            usage_btn,
            attach_btn,
            voice_btn,
            pool_btn,
            cancel,
            Some(send),
            task_mode_enabled,
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Supplement { Todo, Queue, None }
pub fn supplement(has_todos: bool, todos_at_input: bool, has_queue: bool, task_mode: bool) -> Supplement {
    if has_todos && todos_at_input { Supplement::Todo }
    else if has_queue && !task_mode { Supplement::Queue }
    else { Supplement::None }
}
