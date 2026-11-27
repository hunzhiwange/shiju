use crate::app::Message;
use crate::app::components::system_settings_common::{
    icon_svg, primary_action_btn_style, settings_divider, settings_muted_text_style,
};
use crate::app::i18n::I18n;
use crate::app::message::TaskBoardMessage;
use chrono::Local;
use desktop_assets::assets::Icon;
use desktop_settings_cron::system_settings_cron::{CalendarSelection, calendar_month};
use desktop_task_board::state::State;
use desktop_task_board::work_views::{WorkDateCalendar, WorkDateField, WorkDateRange};
use desktop_ui_controls::system_settings_common::icons::icon_btn_svg_style;
use iced::widget::{Space, button, column, row, text};
use iced::{Alignment, Element, Length};

pub(super) fn range_label(i18n: I18n, range: WorkDateRange) -> String {
    match range {
        WorkDateRange::Custom { start, end } if start == end => start.to_string(),
        WorkDateRange::Custom { start, end } => format!("{start} – {end}"),
        _ => i18n.__(range.label()).to_owned(),
    }
}

pub(super) fn choices(board: &State, i18n: I18n) -> Element<'static, Message> {
    if let Some(calendar) = board.work_date_calendar {
        return calendar_view(i18n, calendar);
    }

    let fields = [WorkDateField::Created, WorkDateField::Updated].map(|field| {
        let selected = board.work_date_field == field;
        choice(
            i18n.__(field.label()).to_owned(),
            selected.then_some(Icon::Check),
            selected,
            TaskBoardMessage::SetWorkFilterDateField(field),
        )
    });
    let mut content = column![text(i18n.__("字段")).size(12).style(settings_muted_text_style)]
        .extend(fields)
        .push(settings_divider());
    for range in WorkDateRange::PRESETS {
        let selected = board.work_date_range == Some(range);
        content = content.push(choice(
            range_label(i18n, range),
            selected.then_some(Icon::Check),
            selected,
            TaskBoardMessage::SetWorkFilterDateRange(Some(range)),
        ));
    }
    let custom =
        board.work_date_range.filter(|range| matches!(range, WorkDateRange::Custom { .. }));
    content = content.push(choice(
        custom.map_or_else(
            || i18n.__("自定义日期或范围").to_owned(),
            |range| range_label(i18n, range),
        ),
        Some(Icon::ChevronRight),
        custom.is_some(),
        TaskBoardMessage::ShowWorkFilterCalendar(true),
    ));
    if board.work_date_range.is_some() {
        content = content.push(settings_divider()).push(choice(
            i18n.__("清除日期筛选").to_owned(),
            None,
            false,
            TaskBoardMessage::SetWorkFilterDateRange(None),
        ));
    }
    content.spacing(4).into()
}

fn calendar_view(i18n: I18n, calendar: WorkDateCalendar) -> Element<'static, Message> {
    let grid = calendar_month(
        i18n,
        calendar.month,
        CalendarSelection {
            today: Local::now().date_naive(),
            start: calendar.start,
            end: calendar.end,
        },
        |offset| TaskBoardMessage::ShiftWorkFilterCalendar(offset).into(),
        |date| TaskBoardMessage::PickWorkFilterDate(date).into(),
    );
    let back = button(
        row![icon_svg(Icon::ChevronLeft, 12.0), text(i18n.__("返回")).size(12)]
            .spacing(5)
            .align_y(Alignment::Center),
    )
    .padding([5, 7])
    .style(|theme, status| super::style::navigation_button(theme, status, false))
    .on_press(TaskBoardMessage::ShowWorkFilterCalendar(false).into());
    let apply = button(text(i18n.__("应用")).size(13))
        .padding([5, 11])
        .style(primary_action_btn_style)
        .on_press_maybe(calendar.start.map(|_| TaskBoardMessage::ApplyWorkFilterCalendar.into()));
    column![back, grid, settings_divider(), row![Space::new().width(Length::Fill), apply]]
        .spacing(7)
        .into()
}

fn choice(
    label: String,
    icon: Option<Icon>,
    selected: bool,
    message: TaskBoardMessage,
) -> Element<'static, Message> {
    let mut content = row![text(label).size(13).width(Length::Fill)].align_y(Alignment::Center);
    if let Some(icon) = icon {
        content = content.push(icon_svg(icon, 13.0).style(icon_btn_svg_style));
    }
    button(content)
        .width(Length::Fill)
        .padding([6, 7])
        .style(move |theme, status| super::style::navigation_button(theme, status, selected))
        .on_press(message.into())
        .into()
}
