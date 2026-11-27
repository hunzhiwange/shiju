use crate::app::components::system_settings_common::{
    icon_svg, rounded_action_btn_style, settings_divider, settings_muted_text_style,
    settings_toggle,
};
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_task_board::board::{task_status_marker, task_title};
use desktop_task_board::work_views::{
    BuiltinWorkView, SIDEBAR_TASK_PAGE_SIZE, SidebarWorkView, WorkSidebarMenu, WorkToolbarMenu,
};
use desktop_task_model::Task;
use desktop_ui_controls::{
    overlays::{PointBelowOverlay, SideOverlay},
    right_click_area::RightClickArea,
    single_line_label::SingleLineLabel,
    system_settings_common::{icons::icon_btn_svg_style, settings_panel_style},
};
use iced::widget::{Space, button, column, container, mouse_area, row, rule, scrollable, text};
use iced::{Alignment, Element, Font, Length, Padding};

const VIEWS: [(BuiltinWorkView, Icon); 4] = [
    (BuiltinWorkView::MyTasks, Icon::PersonCircle),
    (BuiltinWorkView::AgentExecuting, Icon::Robot),
    (BuiltinWorkView::Subscribed, Icon::Bell),
    (BuiltinWorkView::AllTasks, Icon::ClipboardCheck),
];

pub(super) fn view(app: &App, width: f32) -> Element<'_, Message> {
    let project = app.navigation_tabs.project_path.as_deref().unwrap_or("");
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let workflow = board.task_board_settings.workflow().or_else(|| {
        desktop_task_model::task_workflow_by_id(desktop_task_model::DEFAULT_TASK_WORKFLOW_ID)
    });
    let personal_ids = board.personal_tasks.get(project).map(Vec::as_slice).unwrap_or_default();
    let mut content = column![view_section(app, i18n)].spacing(4);
    for (view, icon) in VIEWS {
        let key = SidebarWorkView::Builtin(view);
        if board.hidden_work_sidebar_views.contains(&key) {
            continue;
        }
        let tasks = board.task_board_tasks.iter().filter(|task| {
            workflow.as_ref().is_some_and(|workflow| {
                view.matches(task, workflow, personal_ids, board.work_task_subscribed(&task.id))
            })
        });
        content = content.push(view_group(
            app,
            key,
            i18n.__(view.label()),
            icon,
            board.work_view == view && board.selected_work_view.is_none(),
            TaskBoardMessage::SelectWorkView(view),
            tasks,
        ));
    }
    let mut saved = board
        .saved_work_views
        .iter()
        .filter(|view| {
            !board.hidden_work_sidebar_views.contains(&SidebarWorkView::Saved(view.id.clone()))
        })
        .peekable();
    if saved.peek().is_some() {
        content = content.push(section(i18n.__("保存的视图")));
        for view in saved {
            let tasks = board.task_board_tasks.iter().filter(|task| {
                workflow.as_ref().is_some_and(|workflow| {
                    let context = board.work_filter_context(&task.id, super::configured_work_agent(app, task));
                    view.matches(task, workflow, context)
                })
            });
            content = content.push(view_group(
                app,
                SidebarWorkView::Saved(view.id.clone()),
                &view.name,
                Icon::ListUl,
                board.selected_work_view.as_ref() == Some(&view.id),
                TaskBoardMessage::OpenSavedWorkView(view.id.clone()),
                tasks,
            ));
        }
    }
    if !app.workspace_state.recent_projects.is_empty() {
        content = content.push(section(i18n.__("项目")));
        let labels = desktop_workspace::projects::recent_project_labels(
            &app.workspace_state.recent_projects,
            &app.workspace_state.recent_projects_meta,
        );
        for (path, label) in app.workspace_state.recent_projects.iter().zip(labels) {
            let current = path == project;
            content = content.push(
                button(row_label(Icon::FolderClosed, label, 13.0))
                    .padding([7, 9])
                    .style(move |theme, status| {
                        super::style::navigation_button(theme, status, current)
                    })
                    .width(Length::Fill)
                    .on_press(TaskBoardMessage::OpenWorkProject(path.clone()).into()),
            );
        }
    }
    let heading = container(
        text(i18n.__("工作"))
            .size(18)
            .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT }),
    )
    .padding([13, 16]);
    let scroll = scrollable(container(content).width(Length::Fill).padding(Padding {
        top: 8.0,
        bottom: 16.0,
        left: 7.0,
        right: 7.0,
    }))
    .direction(scrollable::Direction::Vertical(
        scrollable::Scrollbar::new().width(4).scroller_width(4),
    ))
    .width(Length::Fill)
    .height(Length::Fill);
    row![
        container(column![heading, settings_divider(), scroll])
            .width(width)
            .height(Length::Fill)
            .style(super::style::sidebar_surface),
        rule::vertical(1),
    ]
    .height(Length::Fill)
    .into()
}

fn view_group<'a>(
    app: &'a App,
    key: SidebarWorkView,
    label: &str,
    icon: Icon,
    selected: bool,
    select: TaskBoardMessage,
    tasks: impl IntoIterator<Item = &'a Task>,
) -> Element<'a, Message> {
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let board = &app.task_board;
    let mut tasks = tasks.into_iter().collect::<Vec<_>>();
    let expanded = board.work_sidebar_view_expanded(&key);
    let hovered = board.work_sidebar_hovered_view.as_ref() == Some(&key);
    let menu_open = board.work_sidebar_menu.as_ref().is_some_and(|menu| {
        menu.view == key && menu.task_id.is_none()
    });
    let show_controls = hovered || menu_open;
    let leading_icon = if expanded {
        Icon::ChevronDown
    } else if show_controls {
        Icon::ChevronRight
    } else {
        icon
    };
    let toggle = iced::widget::tooltip(
        view_row_icon(leading_icon, TaskBoardMessage::ToggleWorkSidebarView(key.clone())),
        container(text(i18n.__(if expanded { "收起" } else { "展开" })).size(12))
            .padding([4, 7])
            .style(desktop_ui_controls::tooltip::tooltip_dark_style),
        iced::widget::tooltip::Position::Top,
    );
    let select = button(SingleLineLabel::new(label.to_owned(), 14.0))
        .padding([7, 2])
        .style(move |theme, status| {
            let mut style = super::style::navigation_button(theme, status, selected);
            if status != button::Status::Pressed {
                style.background = None;
            }
            style
        })
        .width(Length::Fill)
        .on_press(select.into());
    let trailing: Element<'_, Message> = if show_controls {
        let open =
            WorkSidebarMenu { view: key.clone(), task_id: None, position: app.cursor_position };
        view_row_icon(
            Icon::DotsThreeHorizontal,
            TaskBoardMessage::ShowWorkSidebarMenu(Some(open)),
        )
    } else if board.work_sidebar_show_counts && !tasks.is_empty() {
        container(text(tasks.len().to_string()).size(12).style(settings_muted_text_style))
            .center_x(28)
            .into()
    } else {
        Space::new().width(28).into()
    };
    let header = mouse_area(
        container(row![toggle, select, trailing].spacing(3).align_y(Alignment::Center))
            .padding([2, 4])
            .width(Length::Fill)
            .style(move |theme| super::style::sidebar_view_surface(theme, selected, show_controls)),
    )
    .on_enter(TaskBoardMessage::HoverWorkSidebarView(key.clone(), true).into())
    .on_exit(TaskBoardMessage::HoverWorkSidebarView(key.clone(), false).into());
    let header = with_context(app, key.clone(), None, header.into(), || {
        let remove = menu_action(
            Icon::EyeSlash,
            i18n.__("从侧边栏移除"),
            TaskBoardMessage::SetSidebarWorkViewVisible(key.clone(), false),
        );
        container(remove).width(176).padding(5).style(settings_panel_style).into()
    });

    if !expanded {
        return header;
    }

    tasks.sort_by_key(|task| std::cmp::Reverse((task.updated_at_ms, task.created_at_ms)));
    let mut items = column![].spacing(1).width(Length::Fill);
    if tasks.is_empty() {
        let empty = if board.task_board_loading { "正在加载任务…" } else { "无任务" };
        items = items.push(
            container(text(i18n.__(empty)).size(12).style(settings_muted_text_style))
                .padding([6, 8]),
        );
    }

    let limit = board.work_sidebar_task_limits.get(&key).copied().unwrap_or(SIDEBAR_TASK_PAGE_SIZE);
    let remaining = tasks.len().saturating_sub(limit);
    let project = app.navigation_tabs.project_path.as_deref().unwrap_or("");
    for task in tasks.into_iter().take(limit) {
        let current = board.task_board_editing_task_id.as_ref() == Some(&task.id);
        let item = button(
            row![task_status_marker(task.status), SingleLineLabel::new(task_title(task), 12.0)]
                .spacing(8)
                .align_y(Alignment::Center),
        )
        .padding([5, 8])
        .style(move |theme, status| super::style::navigation_button(theme, status, current))
        .width(Length::Fill)
        .on_press(
            TaskBoardMessage::OpenTaskDetail {
                project_path: project.to_owned(),
                task_id: task.id.clone(),
            }
            .into(),
        );
        items = items.push(with_context(app, key.clone(), Some(&task.id), item.into(), || {
            super::modals::build_context_menu(i18n, task.clone())
        }));
    }
    if remaining > 0 {
        let label = format!("{}  {remaining}", i18n.__("加载更多"));
        items = items.push(menu_action(
            Icon::ChevronDown,
            label,
            TaskBoardMessage::LoadMoreSidebarWorkTasks(key),
        ));
    }
    let branch = row![
        rule::vertical(1).style(|theme: &iced::Theme| rule::Style {
            color: theme.palette().text.scale_alpha(0.14),
            ..rule::default(theme)
        }),
        items,
    ]
    .height(Length::Shrink)
    .spacing(7)
    .padding(Padding::ZERO.left(17));
    column![header, branch].spacing(2).into()
}

fn with_context<'a>(
    app: &App,
    view: SidebarWorkView,
    task_id: Option<&str>,
    content: Element<'a, Message>,
    menu: impl FnOnce() -> Element<'a, Message>,
) -> Element<'a, Message> {
    let position = app
        .task_board
        .work_sidebar_menu
        .as_ref()
        .filter(|open| open.view == view && open.task_id.as_deref() == task_id)
        .map(|open| open.position);
    let task_id = task_id.map(str::to_owned);
    let content: Element<'_, _> = RightClickArea::new(
        content,
        Box::new(move |position| {
            TaskBoardMessage::ShowWorkSidebarMenu(Some(WorkSidebarMenu {
                view: view.clone(),
                task_id: task_id.clone(),
                position,
            }))
            .into()
        }),
    )
    .use_global_position()
    .into();
    if let Some(position) = position {
        PointBelowOverlay::new(content, menu())
            .show(true)
            .global_anchor(position)
            .gap(3.0)
            .capture_keyboard()
            .close_on_target_click(true)
            .on_close(TaskBoardMessage::ShowWorkSidebarMenu(None).into())
            .into()
    } else {
        content
    }
}

fn view_section(app: &App, i18n: I18n) -> Element<'_, Message> {
    let settings = if app.task_board.work_sidebar_views_hovered
        || app.task_board.work_toolbar_menu == Some(WorkToolbarMenu::SidebarViews)
    {
        manage_views_button(app, i18n)
    } else {
        Space::new().width(26).height(26).into()
    };
    let heading = row![
        section(i18n.__("视图")),
        Space::new().width(Length::Fill),
        settings,
        icon_button(Icon::Plus, TaskBoardMessage::CreateWorkView),
    ]
    .spacing(3)
    .align_y(Alignment::Center);
    mouse_area(heading)
        .on_enter(TaskBoardMessage::HoverWorkSidebarViews(true).into())
        .on_exit(TaskBoardMessage::HoverWorkSidebarViews(false).into())
        .into()
}

fn manage_views_button(app: &App, i18n: I18n) -> Element<'_, Message> {
    let board = &app.task_board;
    let open = board.work_toolbar_menu == Some(WorkToolbarMenu::SidebarViews);
    let trigger = icon_button(
        Icon::Sliders,
        TaskBoardMessage::ShowWorkToolbarMenu((!open).then_some(WorkToolbarMenu::SidebarViews)),
    );
    if !open {
        return trigger;
    }
    let switch = |name: &str, key: SidebarWorkView| {
        let visible = !board.hidden_work_sidebar_views.contains(&key);
        switch_row(name, visible, move |visible| {
            TaskBoardMessage::SetSidebarWorkViewVisible(key.clone(), visible).into()
        })
    };
    let builtins = VIEWS
        .into_iter()
        .map(|(view, _)| switch(i18n.__(view.label()), SidebarWorkView::Builtin(view)));
    let mut options = column![text(i18n.__("内置视图")).size(14)].spacing(7).extend(builtins);
    if !board.saved_work_views.is_empty() {
        options =
            options.push(settings_divider()).push(text(i18n.__("保存的视图")).size(14)).extend(
                board
                    .saved_work_views
                    .iter()
                    .map(|view| switch(&view.name, SidebarWorkView::Saved(view.id.clone()))),
            );
    }
    options = options.push(settings_divider()).push(switch_row(
        i18n.__("数量"),
        board.work_sidebar_show_counts,
        |visible| TaskBoardMessage::ShowWorkSidebarCounts(visible).into(),
    ));
    let menu = container(
        scrollable(container(options).width(Length::Fill).padding([0, 7]))
            .height(Length::Shrink)
            .direction(scrollable::Direction::Vertical(
                scrollable::Scrollbar::new().width(4).scroller_width(4),
            )),
    )
    .width(252)
    .max_height(380)
    .padding([12, 8])
    .style(settings_panel_style);
    SideOverlay::new(trigger, menu)
        .show(true)
        .gap(6.0)
        .align_y_start(true)
        .on_close(TaskBoardMessage::ShowWorkToolbarMenu(None).into())
        .into()
}

fn switch_row(
    label: &str,
    checked: bool,
    on_toggle: impl Fn(bool) -> Message + 'static,
) -> Element<'static, Message> {
    row![
        SingleLineLabel::new(label.to_owned(), 13.0),
        settings_toggle(checked).width(Length::Shrink).on_toggle(on_toggle),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

pub(super) fn compact_views(app: &App, i18n: I18n) -> Element<'_, Message> {
    let board = &app.task_board;
    let builtins = VIEWS.into_iter().filter_map(|(view, icon)| {
        (!board.hidden_work_sidebar_views.contains(&SidebarWorkView::Builtin(view))).then(|| {
            let selected = board.work_view == view && board.selected_work_view.is_none();
            compact_view(icon, i18n.__(view.label()), selected, TaskBoardMessage::SelectWorkView(view))
        })
    });
    let saved = board.saved_work_views.iter().filter_map(|view| {
        (!board.hidden_work_sidebar_views.contains(&SidebarWorkView::Saved(view.id.clone()))).then(
            || {
                compact_view(
                    Icon::ListUl,
                    &view.name,
                    board.selected_work_view.as_ref() == Some(&view.id),
                    TaskBoardMessage::OpenSavedWorkView(view.id.clone()),
                )
            },
        )
    });
    row(builtins.chain(saved).chain([
        manage_views_button(app, i18n),
        icon_button(Icon::Plus, TaskBoardMessage::CreateWorkView),
    ]))
    .spacing(4)
    .wrap()
    .into()
}

fn compact_view(icon: Icon, label: &str, selected: bool, message: TaskBoardMessage) -> Element<'_, Message> {
    button(row![
        icon_svg(icon, 13.0).style(icon_btn_svg_style),
        text(label).size(12),
    ].spacing(5).align_y(Alignment::Center))
        .padding([6, 8])
        .style(move |theme, status| super::style::navigation_button(theme, status, selected))
        .on_press(message.into())
        .into()
}

fn view_row_icon(icon: Icon, message: TaskBoardMessage) -> Element<'static, Message> {
    button(container(icon_svg(icon, 15.0).style(icon_btn_svg_style)).center(Length::Fill))
        .width(28)
        .height(28)
        .padding(0)
        .style(|theme, status| super::style::navigation_button(theme, status, false))
        .on_press(message.into())
        .into()
}

fn icon_button(icon: Icon, message: TaskBoardMessage) -> Element<'static, Message> {
    button(icon_svg(icon, 14.0).style(icon_btn_svg_style))
        .padding(6)
        .style(|theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            style.border = style.border.rounded(6);
            style
        })
        .on_press(message.into())
        .into()
}

fn menu_action(
    icon: Icon,
    label: impl Into<String>,
    message: TaskBoardMessage,
) -> Element<'static, Message> {
    button(row_label(icon, label, 12.0))
        .padding([6, 8])
        .style(|theme, status| super::style::navigation_button(theme, status, false))
        .width(Length::Fill)
        .on_press(message.into())
        .into()
}

fn section(label: &str) -> Element<'_, Message> {
    container(text(label).size(11).style(settings_muted_text_style)).padding([5, 9]).into()
}

fn row_label<'a>(icon: Icon, label: impl Into<String>, size: f32) -> Element<'a, Message> {
    row![icon_svg(icon, 15.0).style(icon_btn_svg_style), SingleLineLabel::new(label.into(), size),]
        .spacing(8)
        .align_y(Alignment::Center)
        .into()
}
