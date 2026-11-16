use iced::widget::{column, container, mouse_area, responsive, row, scrollable, stack};
use iced::{Background, Color, Element, Length, Theme};
use desktop_ui_controls::system_settings_common::settings_modal_overlay;
#[derive(Clone, Copy)]
pub struct PageView { pub task_board_create_modal_open: bool, pub task_board_worktree_pixel_office: bool, pub kanban: bool, pub viewing_logs: bool, pub task_board_settings_modal_open: bool }
pub struct PageSlots<'a, M> { pub header: Element<'a, M>, pub board: Option<Element<'a, M>>, pub drag: Element<'a, M>, pub create_panel: Option<Element<'a, M>>, pub detail_panel: Option<Element<'a, M>>, pub settings: Option<Element<'a, M>>, pub context_closed: M, pub close_logs: M, pub close_settings: M }
/// 渲染对应界面。
///
/// # 参数
/// - `app`: 当前视图构建所需的状态、配置或消息。
///
/// # 返回
/// 返回可直接嵌入父级视图的 Iced 控件或样式闭包。
///
/// # 错误
/// 此函数不返回 `Result`；不可用状态会通过空视图、禁用控件或回退文案表达。
pub fn view<'a, M: Clone + 'static>(app: PageView, slots: impl Fn(Option<Length>) -> Element<'a, M> + 'a) -> Element<'a, M> {
    if app.task_board_create_modal_open {
        responsive(move |size| {
            let width = if size.width < 800.0 {
                Length::Fill
            } else {
                Length::Fixed((size.width / 3.0).clamp(320.0, 600.0))
            };
            slots(Some(width))
        })
        .into()
    } else {
        slots(None)
    }
}

pub fn build_view<'a, M: Clone + 'static>(app: PageView, create_width: Option<Length>, slots: PageSlots<'a, M>) -> Element<'a, M> {
    let header = slots.header;
    let content = if app.task_board_worktree_pixel_office {
        column![header].spacing(0).width(Length::Fill).height(Length::Fill)
    } else {
        let kanban = slots.board.expect("visible board slot");
        column![header, iced::widget::Space::new().height(12.0), kanban]
            .spacing(0)
            .width(Length::Fill)
            .height(Length::Fill)
    };

    let main_surface: Element<'_, M> = if app.task_board_worktree_pixel_office
        && app.kanban
    {
        scrollable(content)
            .direction(iced::widget::scrollable::Direction::Vertical(
                iced::widget::scrollable::Scrollbar::new().width(4).scroller_width(4),
            ))
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    } else {
        content.into()
    };

    let main_mouse_area = iced::widget::mouse_area(main_surface)
        .on_press(slots.context_closed);

    let base = container(main_mouse_area)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(16)
        .style(task_board_container_style);

    let drag_preview_layer = slots.drag;

    let main_content: Element<'_, M> = if let Some(width) = create_width {
        let panel = slots.create_panel.expect("visible create_panel slot");
        if width == Length::Fill {
            panel
        } else {
            row![
                container(stack![base, drag_preview_layer])
                    .width(Length::Fill)
                    .height(Length::Fill),
                container(panel).width(width),
            ]
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
        }
    } else if app.viewing_logs {
        let detail_panel = slots.detail_panel.expect("visible detail_panel slot");
        let main_content = stack![base, drag_preview_layer];
        let dismiss_layer = mouse_area(
            container(iced::widget::Space::new().width(Length::Fill).height(Length::Fill))
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .on_press(slots.close_logs);
        let detail_layer = row![iced::widget::Space::new().width(Length::Fill), detail_panel]
            .padding(16)
            .width(Length::Fill)
            .height(Length::Fill);
        stack![main_content, dismiss_layer, detail_layer].into()
    } else {
        stack![base, drag_preview_layer].into()
    };

    if app.task_board_settings_modal_open {
        return settings_modal_overlay(
            Some(main_content),
            slots.close_settings,
            container(slots.settings.expect("visible settings slot")).padding(12),
        );
    }

    main_content
}

fn task_board_container_style(theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(
            if theme.palette().background.r
                + theme.palette().background.g
                + theme.palette().background.b
                < 1.5
            {
                theme.extended_palette().background.base.color.scale_alpha(0.94)
            } else {
                Color::from_rgba8(246, 248, 252, 0.98)
            },
        )),
        ..Default::default()
    }
}

/// Lazy content factories keep responsive page assembly in the domain.
pub struct PageBuilders<'a, M> {
    pub header: Box<dyn Fn() -> Element<'a, M> + 'a>,
    pub board: Box<dyn Fn() -> Element<'a, M> + 'a>,
    pub drag: Box<dyn Fn() -> Element<'a, M> + 'a>,
    pub panel: Box<dyn Fn(bool) -> Element<'a, M> + 'a>,
    pub settings: Box<dyn Fn() -> Element<'a, M> + 'a>,
    pub context_closed: M,
    pub close_logs: M,
    pub close_settings: M,
}
pub fn assemble<'a, M: Clone + 'static>(input: PageView, builders: PageBuilders<'a, M>) -> Element<'a, M> {
    view(input, move |width| build_view(input, width, PageSlots {
        header: (builders.header)(),
        board: (!input.task_board_worktree_pixel_office).then(|| (builders.board)()),
        drag: (builders.drag)(),
        create_panel: width.is_some().then(|| (builders.panel)(false)),
        detail_panel: (width.is_none() && input.viewing_logs).then(|| (builders.panel)(true)),
        settings: input.task_board_settings_modal_open.then(|| (builders.settings)()),
        context_closed: builders.context_closed.clone(),
        close_logs: builders.close_logs.clone(),
        close_settings: builders.close_settings.clone(),
    }))
}
