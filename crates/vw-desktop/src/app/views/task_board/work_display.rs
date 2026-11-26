use crate::app::components::system_settings_common::{
    icon_svg, rounded_action_btn_style, settings_close_button, settings_modal_card,
    settings_muted_text_style, settings_segment_button_style, settings_text_input, settings_toggle,
};
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use crate::app::state::TaskBoardViewMode;
use crate::app::{App, Message};
use desktop_assets::assets::Icon;
use desktop_task_board::work_views::{
    BuiltinWorkView, LaneGrouping, WorkDisplay, WorkSort, WorkViewFilter,
};
use desktop_ui_controls::{
    overlays::BelowOverlay, system_settings_common::icons::icon_btn_svg_style,
};
use iced::widget::{Column, Row, Svg, button, column, container, row, text};
use iced::{Alignment, Element, Font, Length};

const MODES: [(TaskBoardViewMode, &str, Icon); 4] = [
    (TaskBoardViewMode::Kanban, "看板", Icon::Columns),
    (TaskBoardViewMode::List, "列表", Icon::ListUl),
    (TaskBoardViewMode::Table, "表格", Icon::Grid1x2),
    (TaskBoardViewMode::Swimlane, "泳道", Icon::AlignMiddle),
];

fn toolbar_icon(icon: Icon, size: f32) -> Svg<'static> {
    icon_svg(icon, size).style(icon_btn_svg_style)
}

pub(super) fn view(app: &App) -> Element<'_, Message> {
    let board = &app.task_board;
    let i18n = I18n::new(app.general_settings.desktop_locale);
    let label = match board.work_view {
        BuiltinWorkView::MyTasks => "我的任务",
        BuiltinWorkView::AgentExecuting => "我的智能体在做",
        BuiltinWorkView::AllTasks => "全部任务",
    };
    let title = board
        .saved_work_views
        .iter()
        .find(|view| board.selected_work_view.as_ref() == Some(&view.id))
        .map_or(i18n.__(label), |view| view.name.as_str());
    let filter_active = board.work_editor_open || board.work_filter != WorkViewFilter::default();
    let display_button = button(
        row![toolbar_icon(Icon::Sliders, 14.0), text(i18n.__("显示设置")).size(13),]
            .spacing(7)
            .align_y(Alignment::Center),
    )
    .padding([6, 10])
    .style(rounded_action_btn_style)
    .on_press(TaskBoardMessage::ShowWorkDisplay(!board.work_display_open).into());
    let display: Element<'_, Message> = if board.work_display_open {
        BelowOverlay::new(display_button, options(app, i18n))
            .show(true)
            .align_right(true)
            .gap(5.0)
            .dismiss_on_escape()
            .on_close(TaskBoardMessage::ShowWorkDisplay(false).into())
            .into()
    } else {
        display_button.into()
    };
    let filter = button(
        row![toolbar_icon(Icon::Funnel, 14.0), text(i18n.__("筛选")).size(13),]
            .spacing(5)
            .align_y(Alignment::Center),
    )
    .padding([6, 10])
    .style(move |theme, status| settings_segment_button_style(theme, status, filter_active))
    .on_press(TaskBoardMessage::OpenWorkViewEditor.into());
    let header = row![
        container(
            text(title)
                .size(17)
                .font(Font { weight: iced::font::Weight::Semibold, ..Font::DEFAULT })
                .wrapping(text::Wrapping::None)
        )
        .width(Length::Fill)
        .clip(true),
        filter,
        display,
    ]
    .spacing(8)
    .align_y(Alignment::Center);
    let search = settings_text_input(i18n.__("搜索任务名称或编号…"), &board.work_search)
        .width(Length::Fill)
        .on_input(|value| TaskBoardMessage::SearchWorkTasks(value).into());
    let search_bar = row![toolbar_icon(Icon::Search, 14.0), container(search).max_width(310),]
        .spacing(8)
        .align_y(Alignment::Center);
    let modes = MODES.into_iter().map(|(mode, label, icon)| {
        let selected = mode == board.task_board_view_mode;
        button(
            row![toolbar_icon(icon, 14.0), text(i18n.__(label)).size(13)]
                .spacing(6)
                .align_y(Alignment::Center),
        )
        .padding([6, 10])
        .style(move |theme, status| settings_segment_button_style(theme, status, selected))
        .on_press(TaskBoardMessage::SelectWorkDisplayMode(mode).into())
        .into()
    });
    let mut content = column![
        container(column![header, row(modes).spacing(4).wrap(), search_bar].spacing(8))
            .padding([0, 7])
    ]
    .width(Length::Fill)
    .height(Length::Fill)
    .spacing(10);
    if board.work_editor_open {
        content = content.push(super::work_view_editor::view(app));
    }
    content.push(super::board::build_work_display(app)).into()
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
            settings_close_button(TaskBoardMessage::ShowWorkDisplay(false).into()),
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
        settings = settings
            .push(
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
