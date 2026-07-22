//! 任务看板员工模式视图。

use iced::widget::{column, container, row};
use iced::{Element, Length};

use crate::app::{App, Message};

pub(crate) mod assignment;
mod feed;
mod lanes;
mod roster;
mod styles;

const COMPACT_WIDTH: f32 = 960.0;
const DRAWER_WIDTH: f32 = 1240.0;
const LANE_WIDTH: f32 = 236.0;
const ROSTER_WIDTH: f32 = 286.0;

pub(super) fn build_staff_mode_view(app: &App) -> Element<'_, Message> {
    let window_width = app.window_size.0;
    let projection = &app.task_board_staff_projection;
    let load_error = app.task_board_staff_config_load_error.as_deref();

    if window_width < COMPACT_WIDTH {
        let lanes = lanes::build_lanes_panel(
            projection,
            &app.task_board_staff_config,
            load_error,
            Length::Fill,
            Length::Fixed(220.0),
        );
        let feed = feed::build_feed_panel(
            projection,
            &app.task_board_staff_command_queue,
            &app.task_board_staff_events,
            app.task_board_staff_feed_tab,
            app.task_board_staff_filter_member_id.as_deref(),
            &app.task_board_staff_task_status_filter,
            &app.task_board_staff_command_input,
            matrix_bridge_snapshot(app),
            Length::Fill,
        );
        let roster = build_staff_side_panel(app, Length::Fill, Length::Fixed(250.0));
        return container(column![lanes, feed, roster].spacing(10))
            .width(Length::Fill)
            .height(Length::Fill)
            .style(styles::root_style)
            .into();
    }

    if window_width < DRAWER_WIDTH {
        let lanes = lanes::build_lanes_panel(
            projection,
            &app.task_board_staff_config,
            load_error,
            Length::Fixed(LANE_WIDTH),
            Length::Fixed(360.0),
        );
        let feed = feed::build_feed_panel(
            projection,
            &app.task_board_staff_command_queue,
            &app.task_board_staff_events,
            app.task_board_staff_feed_tab,
            app.task_board_staff_filter_member_id.as_deref(),
            &app.task_board_staff_task_status_filter,
            &app.task_board_staff_command_input,
            matrix_bridge_snapshot(app),
            Length::Fixed(360.0),
        );
        let roster = build_staff_side_panel(app, Length::Fill, Length::Fixed(250.0));
        return container(
            column![
                row![lanes, feed].spacing(10).width(Length::Fill).height(Length::Fixed(360.0)),
                roster
            ]
            .spacing(10),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .style(styles::root_style)
        .into();
    }

    let lanes = lanes::build_lanes_panel(
        projection,
        &app.task_board_staff_config,
        load_error,
        Length::Fixed(LANE_WIDTH),
        Length::Fill,
    );
    let feed = feed::build_feed_panel(
        projection,
        &app.task_board_staff_command_queue,
        &app.task_board_staff_events,
        app.task_board_staff_feed_tab,
        app.task_board_staff_filter_member_id.as_deref(),
        &app.task_board_staff_task_status_filter,
        &app.task_board_staff_command_input,
        matrix_bridge_snapshot(app),
        Length::Fill,
    );
    let roster = build_staff_side_panel(app, Length::Fixed(ROSTER_WIDTH), Length::Fill);

    container(row![lanes, feed, roster].spacing(10))
        .width(Length::Fill)
        .height(Length::Fill)
        .style(styles::root_style)
        .into()
}

fn matrix_bridge_snapshot(app: &App) -> feed::StaffMatrixBridgeSnapshot {
    feed::StaffMatrixBridgeSnapshot {
        enabled: app.channels_settings.matrix.is_some(),
        cursor: app.task_board_matrix_event_cursor,
        processed_count: app.task_board_processed_matrix_event_ids.len(),
    }
}

fn build_staff_side_panel(app: &App, width: Length, height: Length) -> Element<'_, Message> {
    if app.task_board_staff_assignment_open {
        assignment::build_assignment_panel(app, width, height)
    } else {
        roster::build_roster_panel(
            &app.task_board_staff_projection,
            app.task_board_staff_filter_member_id.as_deref(),
            width,
            height,
        )
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "assignment_tests.rs"]
mod assignment_tests;
