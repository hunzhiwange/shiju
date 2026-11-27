//! Anchored repeat menu and the selected rule's calendar choices.
use super::{schedule, schedule_popup_style};
use crate::message::{Message, SettingsMessage as S};
use crate::state::{CronEditorPicker as Picker, CronEditorPickerAction as Action, CronJobDraft};
use crate::state::{CronScheduleDraft, CronScheduleRule as R};
use chrono::NaiveDate;
use desktop_assets::assets::Icon;
use desktop_i18n::I18n;
use desktop_ui_controls::overlays::BelowOverlay;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_HEIGHT, SETTINGS_CONTROL_TEXT_SIZE, SettingsSelectLabel, icon_svg,
    rounded_action_btn_style,
};
use iced::keyboard::{Key, key::Named};
use iced::widget::{Space, button, column, container, row, scrollable, svg, text};
use iced::{Alignment, Element, Length, Task, Theme};

const ROW_HEIGHT: f32 = 34.0;
const ROW_SPACING: f32 = 2.0;
const MENU_HEIGHT: f32 = 304.0;
const RULES_ID: &str = "cron-repeat-rules";
const MONTHS_ID: &str = "cron-repeat-months";
const DAYS_ID: &str = "cron-repeat-days";

pub(super) fn trigger<'a>(
    label: impl Into<Element<'a, Message>>,
    open: bool,
    on_press: Message,
) -> iced::widget::Button<'a, Message> {
    button(
        row![
            label.into(),
            icon_svg(if open { Icon::ChevronUp } else { Icon::ChevronDown }, 12.0)
                .style(muted_icon),
        ]
        .spacing(8)
        .height(Length::Fill)
        .align_y(Alignment::Center),
    )
    .padding([0, 14])
    .width(Length::Fill)
    .height(SETTINGS_CONTROL_HEIGHT)
    .style(move |theme, status| {
        let mut style = rounded_action_btn_style(theme, status);
        if open {
            style.border.color = theme.palette().text.scale_alpha(0.40);
        }
        style
    })
    .on_press(on_press)
}

pub(super) fn view<'a>(
    i18n: I18n,
    current: &CronScheduleDraft,
    choices: [(R, &'static str); 8],
    selected: usize,
    label: String,
    picker: Option<Picker>,
    editing: bool,
) -> Element<'a, Message> {
    let action = move |action| Message::Settings(S::CronEditorPickerChanged { editing, action });
    let open = matches!(picker, Some(Picker::Repeat { .. }));
    let trigger = trigger(SettingsSelectLabel::new(label), open, action(Action::ToggleRepeat));
    let Some(Picker::Repeat { keyboard_index }) = picker else {
        return trigger.into();
    };

    let keyboard_choices = choices.each_ref().map(|(rule, _)| (*rule).clone());
    let menu = list(choices.into_iter().enumerate().map(|(index, (rule, label))| {
        let active = index == selected;
        let icon = if active {
            Some(if rule.has_repeat_choices() { Icon::ChevronRight } else { Icon::Check })
        } else {
            None
        };
        item(
            label,
            active,
            keyboard_index == Some(index),
            icon,
            action(Action::RepeatSelected(rule)),
        )
    }))
    .id(RULES_ID);
    let details = repeat_choices(i18n, current, editing);
    let width = match &current.rule {
        R::Weekly { .. } | R::Biweekly { .. } => 298,
        R::Monthly { .. } | R::Yearly { .. } => 396,
        _ => 150,
    };
    let mut panels =
        row![panel(menu).width(Length::FillPortion(3))].spacing(2).height(Length::Fill);
    if let Some(details) = details {
        let portion = if width == 298 { 3 } else { 5 };
        panels = panels.push(panel(details).width(Length::FillPortion(portion)));
    }

    BelowOverlay::new(trigger, container(panels).width(width).height(MENU_HEIGHT))
        .show(true)
        .within_window()
        .prefer_above()
        .gap(5.0)
        .on_close(action(Action::Close))
        .on_key_press(move |key| {
            let last = keyboard_choices.len() - 1;
            let index = keyboard_index.unwrap_or(selected).min(last);
            let event = match key {
                Key::Named(Named::ArrowDown) => Action::RepeatFocused((index + 1).min(last)),
                Key::Named(Named::ArrowUp) => Action::RepeatFocused(index.saturating_sub(1)),
                Key::Named(Named::Home) => Action::RepeatFocused(0),
                Key::Named(Named::End) => Action::RepeatFocused(last),
                Key::Named(Named::Enter | Named::Space) => {
                    Action::RepeatSelected(keyboard_choices[index].clone())
                }
                _ => return None,
            };
            Some(action(event))
        })
        .dismiss_on_escape()
        .into()
}

fn repeat_choices<'a>(
    i18n: I18n,
    draft: &CronScheduleDraft,
    editing: bool,
) -> Option<Element<'a, Message>> {
    let select = |rule| {
        let mut next = draft.clone();
        next.rule = rule;
        schedule::changed(editing, next)
    };
    let details = match &draft.rule {
        R::Weekly { weekdays, time } | R::Biweekly { weekdays, time, .. } => {
            let options =
                schedule::weekday_labels(i18n).into_iter().enumerate().map(|(index, label)| {
                    let day = index as u32 + 1;
                    let active = weekdays.contains(&day);
                    let updated = toggled(weekdays, day);
                    let rule = match &draft.rule {
                        R::Biweekly { anchor, .. } => {
                            R::Biweekly { anchor: *anchor, weekdays: updated, time: *time }
                        }
                        _ => R::Weekly { weekdays: updated, time: *time },
                    };
                    item(label, active, false, active.then_some(Icon::Check), select(rule))
                });
            list(options).into()
        }
        R::Monthly { days, time } => {
            let mut grid = column![].spacing(ROW_SPACING);
            for start in (1..=31u32).step_by(7) {
                let week = row((start..start + 7).map(|day| {
                    if day > 31 {
                        return Space::new().width(Length::Fill).into();
                    }
                    let rule = R::Monthly { days: toggled(days, day), time: *time };
                    cell(day.to_string(), days.contains(&day), select(rule))
                }))
                .spacing(ROW_SPACING);
                grid = grid.push(week);
            }
            scroll(grid).into()
        }
        R::Yearly { month, day, time } => {
            let months = month_labels(i18n).into_iter().enumerate().map(|(index, label)| {
                let value = index as u32 + 1;
                let new_day = (1..=(*day).min(31))
                    .rev()
                    .find(|day| NaiveDate::from_ymd_opt(2000, value, *day).is_some())
                    .unwrap_or(1);
                let rule = R::Yearly { month: value, day: new_day, time: *time };
                cell(label, *month == value, select(rule))
            });
            let days = (1..=31)
                .filter(|value| NaiveDate::from_ymd_opt(2000, *month, *value).is_some())
                .map(|value| {
                    let rule = R::Yearly { month: *month, day: value, time: *time };
                    cell(value.to_string(), *day == value, select(rule))
                });
            row![list(months).id(MONTHS_ID), list(days).id(DAYS_ID)].spacing(6).into()
        }
        _ => return None,
    };
    Some(details)
}

fn month_labels(i18n: I18n) -> [&'static str; 12] {
    [
        "一月",
        "二月",
        "三月",
        "四月",
        "五月",
        "六月",
        "七月",
        "八月",
        "九月",
        "十月",
        "十一月",
        "十二月",
    ]
    .map(|month| i18n.__(month))
}

fn toggled(values: &[u32], value: u32) -> Vec<u32> {
    let mut values = values.to_vec();
    if values.contains(&value) {
        values.retain(|day| *day != value);
    } else {
        values.push(value);
        values.sort_unstable();
    }
    values
}

pub(crate) fn reveal_year(draft: &CronJobDraft) -> Task<crate::update::messages::Message> {
    let R::Yearly { month, day, .. } = draft.effective_schedule().rule else {
        return Task::none();
    };
    Task::batch([
        scroll_to_index(MONTHS_ID, month.saturating_sub(1) as usize),
        scroll_to_index(DAYS_ID, day.saturating_sub(1) as usize),
    ])
}

pub(crate) fn reveal_rule(index: usize) -> Task<crate::update::messages::Message> {
    scroll_to_index(RULES_ID, index)
}

fn scroll_to_index(id: &'static str, index: usize) -> Task<crate::update::messages::Message> {
    use iced::widget::operation::{AbsoluteOffset, scroll_to};
    let y = index.saturating_sub(3) as f32 * (ROW_HEIGHT + ROW_SPACING);
    scroll_to(id, AbsoluteOffset { x: None, y: Some(y) })
}

fn panel<'a>(content: impl Into<Element<'a, Message>>) -> iced::widget::Container<'a, Message> {
    container(content).height(Length::Fill).padding(8).style(schedule_popup_style)
}

fn scroll<'a>(items: iced::widget::Column<'a, Message>) -> iced::widget::Scrollable<'a, Message> {
    scrollable(container(items).padding([0, 7]).width(Length::Fill))
        .direction(scrollable::Direction::Vertical(
            scrollable::Scrollbar::new().width(4).scroller_width(4),
        ))
        .width(Length::Fill)
        .height(Length::Fill)
}

fn list<'a>(
    items: impl IntoIterator<Item = Element<'a, Message>>,
) -> iced::widget::Scrollable<'a, Message> {
    scroll(column(items).spacing(ROW_SPACING))
}

fn item<'a>(
    label: &'static str,
    selected: bool,
    focused: bool,
    icon: Option<Icon>,
    message: Message,
) -> Element<'a, Message> {
    let label = text(label).size(SETTINGS_CONTROL_TEXT_SIZE).width(Length::Fill);
    let mut content = row![label].spacing(5).align_y(Alignment::Center);
    if let Some(icon) = icon {
        content = content.push(icon_svg(icon, 13.0).style(move |theme, status| {
            if matches!(icon, Icon::Check) {
                svg::Style { color: Some(theme.palette().success) }
            } else {
                muted_icon(theme, status)
            }
        }));
    }
    option(content, selected, focused, message).padding([0, 8]).into()
}

fn cell<'a>(label: impl Into<String>, selected: bool, message: Message) -> Element<'a, Message> {
    option(
        text(label.into()).size(SETTINGS_CONTROL_TEXT_SIZE).width(Length::Fill).center(),
        selected,
        false,
        message,
    )
    .padding(0)
    .into()
}

fn option<'a>(
    content: impl Into<Element<'a, Message>>,
    selected: bool,
    focused: bool,
    message: Message,
) -> iced::widget::Button<'a, Message> {
    button(container(content).center_y(Length::Fill))
        .height(ROW_HEIGHT)
        .width(Length::Fill)
        .clip(true)
        .style(move |theme, status| {
            let mut style = rounded_action_btn_style(theme, status);
            style.border.width = if focused { 2.0 } else { 0.0 };
            style.border.color = theme.palette().primary;
            style.border.radius = 8.0.into();
            if status == button::Status::Active {
                style.background = selected.then(|| theme.palette().text.scale_alpha(0.10).into());
            }
            style
        })
        .on_press(message)
}

fn muted_icon(theme: &Theme, _status: svg::Status) -> svg::Style {
    svg::Style { color: Some(theme.palette().text.scale_alpha(0.65)) }
}
