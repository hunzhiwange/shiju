use crate::app::components::system_settings_common::{
    icon_svg, primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_divider, settings_modal_card, settings_muted_text_style, settings_panel_style,
    settings_segment_button_style, settings_text_input, settings_toggle,
};
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardViewMode;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_task_board::work_views::{
    LaneGrouping, SavedWorkView, WorkDisplay, WorkSort, WorkToolbarMenu,
};
use desktop_ui_controls::{
    overlays::BelowOverlay, system_settings_common::icons::icon_btn_svg_style,
};
use iced::widget::{Column, Row, Svg, button, column, container, row, text};
use iced::{Alignment, Element, Font, Length};

pub(in crate::app) const WORK_VIEW_NAME_INPUT_ID: &str = "task-board-work-view-name";

const MODES: [(TaskBoardViewMode, &str, Icon); 4] = [
    (TaskBoardViewMode::Kanban, "看板", Icon::Columns),
    (TaskBoardViewMode::List, "列表", Icon::ListUl),
    (TaskBoardViewMode::Table, "表格", Icon::Grid1x2),
    (TaskBoardViewMode::Swimlane, "泳道", Icon::AlignMiddle),
];

fn toolbar_icon(icon: Icon, size: f32) -> Svg<'static> {
    icon_svg(icon, size).style(icon_btn_svg_style)
}

pub(super) fn view(app: &App, compact: bool) -> Element<'_, Message> {
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let saved_view = board
        .saved_work_views
        .iter()
        .find(|view| board.selected_work_view.as_ref() == Some(&view.id));
    let display = toolbar_menu(
        board.work_toolbar_menu,
        WorkToolbarMenu::Display,
        row![toolbar_icon(Icon::Sliders, 14.0), text(i18n.__("显示设置")).size(13),]
            .spacing(7)
            .align_y(Alignment::Center),
        || options(app, i18n),
    );
    let actions = toolbar_menu(
        board.work_toolbar_menu,
        WorkToolbarMenu::Actions,
        row![text(i18n.__("操作")).size(13), toolbar_icon(Icon::ChevronDown, 12.0)]
            .spacing(6)
            .align_y(Alignment::Center),
        || super::control::build_control_buttons(app),
    );
    let title = view_title(app, i18n, saved_view);
    let mut toolbar_actions = row![].spacing(6).align_y(Alignment::Center);
    if board.work_editor_open {
        toolbar_actions = toolbar_actions.push(
            button(text(i18n.__("关闭编辑")).size(13))
                .padding([6, 10])
                .style(|theme, status| settings_segment_button_style(theme, status, true))
                .on_press(TaskBoardMessage::OpenWorkViewEditor.into()),
        );
    }
    let actions = toolbar_actions.push(display).push(actions);
    let modes = MODES.into_iter().map(|(mode, label, icon)| {
        let selected = mode == board.task_board_view_mode;
        button(
            row![toolbar_icon(icon, 14.0), text(i18n.__(label)).size(13)]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .style(move |theme, status| super::style::navigation_button(theme, status, selected))
        .on_press(TaskBoardMessage::SelectWorkDisplayMode(mode).into())
        .into()
    });
    let mut toolbar = if compact {
        column![title, super::work_sidebar::compact_views(app, i18n), actions.wrap()]
    } else if board.work_view_name_editing {
        column![title, actions.wrap()]
    } else {
        column![row![title, actions].spacing(8).align_y(Alignment::Center)]
    }
    .spacing(12);
    if !board.work_editor_open {
        let add = filter_menu(app, i18n);
        toolbar = toolbar.push(super::work_filter_bar::view(app, i18n, saved_view, add));
    }
    let toolbar = toolbar.push(row(modes).spacing(4).wrap());
    let mut content =
        column![container(toolbar).padding([12, 18]).width(Length::Fill), settings_divider(),]
            .width(Length::Fill)
            .height(Length::Fill);
    if board.work_editor_open {
        content = content.push(container(super::work_view_editor::view(app)).padding([10, 14]));
    }
    content
        .push(
            container(super::board::build_work_display(app))
                .padding([8, 7])
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .into()
}

fn filter_menu(app: &App, i18n: I18n) -> Element<'_, Message> {
    let current = app.task_board.work_toolbar_menu;
    let open = matches!(current, Some(WorkToolbarMenu::Filter | WorkToolbarMenu::AdvancedFilter));
    let anchor = button(
        row![toolbar_icon(Icon::Plus, 14.0), text(i18n.__("添加筛选")).size(13)]
            .spacing(5)
            .align_y(Alignment::Center),
    )
    .padding([6, 9])
    .style(move |theme, status| {
        if open {
            settings_segment_button_style(theme, status, true)
        } else {
            rounded_action_btn_style(theme, status)
        }
    })
    .on_press(TaskBoardMessage::ShowWorkToolbarMenu((!open).then_some(WorkToolbarMenu::Filter)).into());
    let popup = match current {
        Some(WorkToolbarMenu::Filter) => super::work_filter_picker::view(app),
        Some(WorkToolbarMenu::AdvancedFilter) => super::work_filter_advanced::view(app, i18n),
        _ => return anchor.into(),
    };
    BelowOverlay::new(anchor, popup)
        .show(true)
        .within_window()
        .gap(5.0)
        .dismiss_on_escape()
        .on_close(TaskBoardMessage::ShowWorkToolbarMenu(None).into())
        .into()
}

fn view_title<'a>(
    app: &'a App,
    i18n: I18n,
    saved_view: Option<&'a SavedWorkView>,
) -> Element<'a, Message> {
    let board = &app.task_board;
    let Some(view) = saved_view else {
        return container(
            text(i18n.__(board.work_view.label()))
                .size(24)
                .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT })
                .wrapping(text::Wrapping::None),
        )
        .width(Length::Fill)
        .clip(true)
        .into();
    };
    if board.work_view_name_editing {
        let can_save = !board.work_view_name.trim().is_empty();
        let save = Message::from(TaskBoardMessage::SaveWorkViewName);
        let name = settings_text_input(i18n.__("视图名称"), &board.work_view_name)
            .id(WORK_VIEW_NAME_INPUT_ID)
            .width(Length::Fill)
            .on_input(|name| TaskBoardMessage::WorkViewFieldChanged(0, name).into())
            .on_submit_maybe(can_save.then(|| save.clone()));
        return row![
            toolbar_icon(Icon::ListUl, 16.0),
            container(name).max_width(340),
            button(text(i18n.__("保存")).size(13))
                .padding([6, 12])
                .style(primary_action_btn_style)
                .on_press_maybe(can_save.then_some(save)),
            settings_close_button(TaskBoardMessage::CancelWorkViewName.into()),
        ]
        .spacing(7)
        .align_y(Alignment::Center)
        .into();
    }
    let title = toolbar_menu(
        board.work_toolbar_menu,
        WorkToolbarMenu::ViewActions,
        row![
            toolbar_icon(Icon::ListUl, 16.0),
            desktop_ui_controls::single_line_label::SingleLineLabel::new(view.name.clone(), 20.0)
                .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT })
                .shrink(),
            toolbar_icon(Icon::ChevronDown, 12.0),
        ]
        .spacing(7)
        .align_y(Alignment::Center),
        || view_menu(i18n),
    );
    container(title).width(Length::Fill).into()
}

fn view_menu(i18n: I18n) -> Element<'static, Message> {
    let items = [
        (Icon::Sliders, "编辑视图", TaskBoardMessage::EditWorkView),
        (Icon::Pencil, "重命名", TaskBoardMessage::EditWorkViewName),
        (Icon::Copy, "创建副本", TaskBoardMessage::DuplicateWorkView),
    ];
    container(
        column(items.into_iter().map(|(icon, label, action)| {
            super::work_filter_picker::menu_row(icon, i18n.__(label).to_owned(), action, false)
        }))
        .spacing(3),
    )
    .width(196)
    .padding(6)
    .style(settings_panel_style)
    .into()
}

fn toolbar_menu<'a>(
    current: Option<WorkToolbarMenu>,
    menu: WorkToolbarMenu,
    label: Row<'a, Message>,
    content: impl FnOnce() -> Element<'a, Message>,
) -> Element<'a, Message> {
    let open = current == Some(menu);
    let anchor = button(label)
        .padding([6, 10])
        .style(move |theme, status| {
            if open {
                settings_segment_button_style(theme, status, true)
            } else {
                rounded_action_btn_style(theme, status)
            }
        })
        .on_press(TaskBoardMessage::ShowWorkToolbarMenu((!open).then_some(menu)).into());
    if open {
        BelowOverlay::new(anchor, content())
            .show(true)
            .align_right(matches!(menu, WorkToolbarMenu::Display | WorkToolbarMenu::Actions))
            .gap(5.0)
            .dismiss_on_escape()
            .on_close(TaskBoardMessage::ShowWorkToolbarMenu(None).into())
            .into()
    } else {
        anchor.into()
    }
}

fn section<'a>(label: &'a str, controls: Row<'a, Message>) -> Row<'a, Message> {
    row![
        text(label).size(13).style(settings_muted_text_style).width(Length::Fill),
        controls.spacing(4),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
}

fn segment<'a>(label: &'a str, selected: bool, message: TaskBoardMessage) -> Element<'a, Message> {
    button(text(label).size(12))
        .padding([5, 8])
        .style(move |theme, status| settings_segment_button_style(theme, status, selected))
        .on_press(message.into())
        .into()
}

fn options(app: &App, i18n: I18n) -> Element<'_, Message> {
    let board = &app.task_board;
    let mode = board.task_board_view_mode;
    let display = board.work_display;
    let sort = [
        (WorkSort::Created, "创建时间"),
        (WorkSort::Updated, "更新时间"),
        (WorkSort::Priority, "优先级"),
    ]
    .into_iter()
    .map(|(sort, label)| {
        segment(
            i18n.__(label),
            sort == display.sort,
            TaskBoardMessage::ChangeWorkDisplay(WorkDisplay { sort, ..display }),
        )
    });
    let mut settings: Column<'_, Message> = column![
        row![
            text(i18n.__("显示设置")).size(15).width(Length::Fill),
            settings_close_button(TaskBoardMessage::ShowWorkToolbarMenu(None).into()),
        ]
        .align_y(Alignment::Center),
        section(i18n.__("排序"), row(sort)),
        section(
            i18n.__("顺序"),
            row![
                segment(
                    i18n.__("升序"),
                    display.ascending,
                    TaskBoardMessage::ChangeWorkDisplay(WorkDisplay { ascending: true, ..display })
                ),
                segment(
                    i18n.__("降序"),
                    !display.ascending,
                    TaskBoardMessage::ChangeWorkDisplay(WorkDisplay {
                        ascending: false,
                        ..display
                    })
                ),
            ]
        ),
    ]
    .spacing(12);

    if mode == TaskBoardViewMode::Swimlane {
        let grouping = [(LaneGrouping::Assignee, "负责人"), (LaneGrouping::Agent, "执行智能体")]
            .into_iter()
            .map(|(lane_grouping, label)| {
                segment(
                    i18n.__(label),
                    display.lane_grouping == lane_grouping,
                    TaskBoardMessage::ChangeWorkDisplay(WorkDisplay { lane_grouping, ..display }),
                )
            });
        settings = settings.push(section(i18n.__("泳道分组"), row(grouping)));
    }
    if mode == TaskBoardViewMode::Table {
        settings = settings.push(
            settings_toggle(display.table_group_status)
                .label(i18n.__("按状态分组"))
                .text_size(13)
                .on_toggle(move |table_group_status| {
                    TaskBoardMessage::ChangeWorkDisplay(WorkDisplay {
                        table_group_status,
                        ..display
                    })
                    .into()
                }),
        );
    }
    if mode != TaskBoardViewMode::Table || display.table_group_status {
        settings = settings.push(
            settings_toggle(display.hide_empty_statuses)
                .label(i18n.__("隐藏空状态"))
                .text_size(13)
                .on_toggle(move |hide_empty_statuses| {
                    TaskBoardMessage::ChangeWorkDisplay(WorkDisplay {
                        hide_empty_statuses,
                        ..display
                    })
                    .into()
                }),
        );
    }
    settings_modal_card(settings).width(350).padding(12).into()
}
