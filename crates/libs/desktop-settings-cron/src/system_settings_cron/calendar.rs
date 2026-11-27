//! Calendar grids shared by the anchored time and validity pickers.
use crate::state::shift_month;
use chrono::{Datelike, Days, NaiveDate};
use desktop_i18n::I18n;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::system_settings_common::{
    rounded_action_btn_style, settings_muted_text_style,
};
use iced::widget::{Space, button, column, responsive, row, text};
use iced::{Alignment, Element, Length};

pub(super) const POPOVER_WIDTH: u32 = 560;

#[derive(Clone, Copy)]
pub struct Selection {
    pub today: NaiveDate,
    pub start: Option<NaiveDate>,
    pub end: Option<NaiveDate>,
}

pub(super) fn anchored<'a, M: Clone + 'a>(
    trigger: Element<'a, M>,
    popup: Option<Element<'a, M>>,
    close: M,
) -> Element<'a, M> {
    let open = popup.is_some();
    BelowOverlay::new(trigger, popup.unwrap_or_else(|| Space::new().into()))
        .show(open)
        .within_window()
        .prefer_above()
        .align_right(true)
        .gap(5.0)
        .on_close(close)
        .dismiss_on_escape()
        .into()
}

pub(super) fn days(month: NaiveDate, full_grid: bool) -> Vec<Option<NaiveDate>> {
    let leading = month.weekday().num_days_from_monday();
    let last = shift_month(month, 1).and_then(|next| next.pred_opt()).unwrap_or(NaiveDate::MAX);
    let count = if full_grid { 42 } else { (leading + last.day()).div_ceil(7) * 7 };
    (0..count)
        .map(|index| {
            if index < leading {
                month.checked_sub_days(Days::new((leading - index).into()))
            } else {
                month.checked_add_days(Days::new((index - leading).into()))
            }
        })
        .collect()
}

pub(super) fn pair<'a, M: Clone + 'a>(
    i18n: I18n,
    month: NaiveDate,
    selection: Selection,
    navigate: impl Fn(i32) -> M + 'a,
    pick: impl Fn(NaiveDate) -> M + 'a,
) -> Element<'a, M> {
    responsive(move |size| {
        let left = month_grid(i18n, month, selection, true, &navigate, &pick);
        if size.width >= 440.0
            && let Some(next) = shift_month(month, 1)
        {
            let right = month_grid(i18n, next, selection, true, &navigate, &pick);
            return row![left, right].spacing(22).width(Length::Fill).into();
        }
        left.into()
    })
    .height(Length::Shrink)
    .into()
}

pub fn calendar_month<'a, M: Clone + 'a>(
    i18n: I18n,
    month: NaiveDate,
    selection: Selection,
    navigate: impl Fn(i32) -> M,
    pick: impl Fn(NaiveDate) -> M,
) -> Element<'a, M> {
    month_grid(i18n, month, selection, false, &navigate, &pick).into()
}

fn month_grid<'a, M: Clone + 'a>(
    i18n: I18n,
    month: NaiveDate,
    selection: Selection,
    dim_past: bool,
    navigate: &impl Fn(i32) -> M,
    pick: &impl Fn(NaiveDate) -> M,
) -> iced::widget::Column<'a, M> {
    let arrow = |label, offset| {
        button(text(label).size(18).width(Length::Fill).height(Length::Fill).center())
            .width(23)
            .height(29)
            .padding(0)
            .style(|theme, status| {
                let mut style = rounded_action_btn_style(theme, status);
                style.border.width = 0.0;
                style.text_color = theme.palette().text.scale_alpha(0.65);
                if status == button::Status::Active {
                    style.background = None;
                }
                style
            })
            .on_press(navigate(offset))
    };
    let heading = desktop_i18n::__!(i18n, "%d年%u月", month.year(), month.month())
        .expect("valid month title");
    let header = row![
        arrow("«", -12),
        arrow("‹", -1),
        text(heading).size(14).width(Length::Fill).center(),
        arrow("›", 1),
        arrow("»", 12),
    ]
    .spacing(2)
    .width(Length::Fill)
    .align_y(Alignment::Center);
    let weekdays = row(["一", "二", "三", "四", "五", "六", "日"].map(|day| {
        text(i18n.__(day))
            .size(12)
            .style(settings_muted_text_style)
            .width(Length::Fill)
            .center()
            .into()
    }))
    .spacing(2);
    let mut grid = column![header, weekdays].spacing(5).width(Length::Fill);
    for week in days(month, true).chunks(7) {
        let cells = week.iter().map(|date| match *date {
            Some(date) => day_cell(month, date, selection, dim_past, || pick(date)),
            None => Space::new().width(Length::Fill).height(29).into(),
        });
        grid = grid.push(row(cells).spacing(2));
    }
    grid
}

fn day_cell<'a, M: Clone + 'a>(
    month: NaiveDate,
    date: NaiveDate,
    selection: Selection,
    dim_past: bool,
    pick: impl FnOnce() -> M,
) -> Element<'a, M> {
    let in_month = date.month() == month.month();
    let selected = in_month && (selection.start == Some(date) || selection.end == Some(date));
    let today = in_month && selection.today == date;
    let in_range = in_month
        && matches!(
            (selection.start, selection.end),
            (Some(start), Some(end)) if start < date && date < end
        );
    button(text(date.day().to_string()).size(13).width(Length::Fill).height(Length::Fill).center())
        .width(Length::Fill)
        .height(29)
        .padding(0)
        .style(move |theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            let palette = theme.palette();
            style.border.radius = 6.0.into();
            style.border.width = if selected || today { 1.0 } else { 0.0 };
            style.border.color =
                if selected { palette.primary } else { palette.text.scale_alpha(0.48) };
            if selected {
                style.background = Some(palette.primary.scale_alpha(0.18).into());
            } else if in_range {
                style.background = Some(palette.primary.scale_alpha(0.10).into());
            } else if matches!(status, button::Status::Active | button::Status::Disabled) {
                style.background = None;
            }
            style.text_color = if !in_month {
                palette.text.scale_alpha(0.22)
            } else if dim_past && date < selection.today && !selected && !in_range {
                palette.text.scale_alpha(0.48)
            } else {
                palette.text
            };
            style
        })
        .on_press_maybe(in_month.then(pick))
        .into()
}
