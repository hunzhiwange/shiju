use iced::widget::{column, container, row, scrollable, stack};
use iced::{Element, Length, Theme};
use desktop_ui_controls::system_settings_common::{
    panels::settings_right_drawer_overlay, settings_modal_overlay,
};
#[derive(Clone, Copy)]
pub struct PageView {
    pub task_board_worktree_pixel_office: bool,
    pub kanban: bool,
    pub viewing_logs: bool,
    pub task_board_settings_modal_open: bool,
}

struct PageSlots<'a, M> {
    navigation: Option<Element<'a, M>>,
    header: Element<'a, M>,
    board: Option<Element<'a, M>>,
    drag: Element<'a, M>,
    detail_panel: Option<Element<'a, M>>,
    settings: Option<Element<'a, M>>,
    context_closed: M,
    close_logs: M,
    close_settings: M,
}

fn build_view<'a, M: Clone + 'static>(app: PageView, slots: PageSlots<'a, M>) -> Element<'a, M> {
    let header = slots.header;
    let content = if app.task_board_worktree_pixel_office {
        column![header].spacing(0).width(Length::Fill).height(Length::Fill)
    } else {
        let kanban = slots.board.expect("visible board slot");
        column![header, kanban]
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

    let main_surface = row(slots.navigation.into_iter().chain([main_surface]))
        .width(Length::Fill)
        .height(Length::Fill);
    let main_mouse_area = iced::widget::mouse_area(main_surface)
        .on_press(slots.context_closed);

    let base = container(main_mouse_area)
        .width(Length::Fill)
        .height(Length::Fill)
        .style(task_board_container_style);

    let base = stack![base, slots.drag].into();
    let main_content = if app.viewing_logs {
        let detail_panel = slots.detail_panel.expect("visible detail_panel slot");
        settings_right_drawer_overlay(
            base,
            Some((slots.close_logs, container(detail_panel).padding(16).into())),
        )
    } else {
        base
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
        background: Some(theme.palette().background.into()),
        ..Default::default()
    }
}

/// Lazy content factories keep page assembly in the domain.
pub struct PageBuilders<'a, M> {
    pub navigation: Option<Element<'a, M>>,
    pub header: Box<dyn Fn() -> Element<'a, M> + 'a>,
    pub board: Box<dyn Fn() -> Element<'a, M> + 'a>,
    pub drag: Box<dyn Fn() -> Element<'a, M> + 'a>,
    pub detail: Box<dyn Fn() -> Element<'a, M> + 'a>,
    pub settings: Box<dyn Fn() -> Element<'a, M> + 'a>,
    pub context_closed: M,
    pub close_logs: M,
    pub close_settings: M,
}
pub fn assemble<'a, M: Clone + 'static>(input: PageView, builders: PageBuilders<'a, M>) -> Element<'a, M> {
    build_view(input, PageSlots {
        navigation: builders.navigation,
        header: (builders.header)(),
        board: (!input.task_board_worktree_pixel_office).then(|| (builders.board)()),
        drag: (builders.drag)(),
        detail_panel: input.viewing_logs.then(|| (builders.detail)()),
        settings: input.task_board_settings_modal_open.then(|| (builders.settings)()),
        context_closed: builders.context_closed,
        close_logs: builders.close_logs,
        close_settings: builders.close_settings,
    })
}
