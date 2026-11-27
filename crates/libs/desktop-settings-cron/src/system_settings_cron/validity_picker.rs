//! Anchored editor for optional, inclusive calendar boundaries.
use super::{calendar, repeat_picker, schedule_popup_style, secondary_button};
use crate::message::{Message, SettingsMessage};
use crate::state::{
    CronEditorPicker, CronEditorPickerAction, CronScheduleDraft, ValidityAction, ValidityEndpoint,
    ValidityPicker,
};
use chrono::NaiveDate;
use desktop_i18n::I18n;
use desktop_ui_controls::system_settings_common::{
    SETTINGS_CONTROL_PADDING, SETTINGS_CONTROL_TEXT_SIZE, SettingsSelectLabel,
    primary_action_btn_style, rounded_action_btn_style, settings_close_button,
    settings_modal_layout, settings_muted_text_style,
};
use iced::widget::{button, column, container, row, text};
use iced::{Alignment, Element, Length};

pub(super) fn view<'a>(
    i18n: I18n,
    schedule: &CronScheduleDraft,
    today: NaiveDate,
    open: Option<CronEditorPicker>,
    editing: bool,
) -> Element<'a, Message> {
    let action = move |action| {
        Message::Settings(SettingsMessage::CronEditorPickerChanged { editing, action })
    };
    let update = move |event| action(CronEditorPickerAction::Validity(event));
    let close = action(CronEditorPickerAction::Close);
    let popup = match open {
        Some(CronEditorPicker::Validity(picker)) => {
            Some(card(i18n, today, picker, update, close.clone()))
        }
        _ => None,
    };
    let label = match (schedule.boundary_date(false), schedule.boundary_date(true)) {
        (Some(start), Some(end)) => format!("{start} – {end}"),
        (Some(date), None) => format!("{} · {date}", i18n.__("开始日期")),
        (None, Some(date)) => format!("{} · {date}", i18n.__("结束日期（含当日）")),
        (None, None) => i18n.__("长期有效").to_owned(),
    };
    let trigger = repeat_picker::trigger(
        SettingsSelectLabel::new(
            desktop_i18n::__!(i18n, "有效期：%s", label).expect("valid settings message"),
        ),
        popup.is_some(),
        update(ValidityAction::Toggle),
    );
    calendar::anchored(trigger.into(), popup, close)
}

fn card<'a>(
    i18n: I18n,
    today: NaiveDate,
    picker: ValidityPicker,
    update: impl Fn(ValidityAction) -> Message + Copy + 'a,
    close: Message,
) -> Element<'a, Message> {
    let boundary = |endpoint, label: &'static str, date: Option<NaiveDate>| {
        let title = date.map_or_else(|| label.to_owned(), |date| format!("{label} · {date}"));
        let active = picker.endpoint == endpoint;
        let trigger = button(text(title).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding(SETTINGS_CONTROL_PADDING)
            .style(move |theme, status| {
                let mut style = rounded_action_btn_style(theme, status);
                if active {
                    style.border.color = theme.palette().primary;
                }
                style
            })
            .on_press(update(ValidityAction::Focus(endpoint)));
        let mut field = row![trigger].spacing(3).align_y(Alignment::Center);
        if date.is_some() {
            field = field.push(settings_close_button(update(ValidityAction::Clear(endpoint))));
        }
        field
    };
    let header = row![
        boundary(ValidityEndpoint::Start, i18n.__("开始日期"), picker.start),
        boundary(ValidityEndpoint::End, i18n.__("结束日期（含当日）"), picker.end),
    ]
    .spacing(8)
    .wrap();
    let months = calendar::pair(
        i18n,
        picker.month,
        calendar::Selection { today, start: picker.start, end: picker.end },
        move |step| update(ValidityAction::Navigate(step)),
        move |date| update(ValidityAction::Pick(date)),
    );
    let mut body = column![months].spacing(6);
    if !picker.is_ordered() {
        body = body.push(
            text(i18n.__("有效期结束日期不能早于开始日期"))
                .size(12)
                .style(settings_muted_text_style),
        );
    }
    let actions = row![
        secondary_button(i18n.__("长期有效"), update(ValidityAction::NoExpiry)),
        secondary_button(i18n.__("取消"), close),
        button(text(i18n.__("确认")).size(SETTINGS_CONTROL_TEXT_SIZE))
            .padding(SETTINGS_CONTROL_PADDING)
            .style(primary_action_btn_style)
            .on_press_maybe(picker.is_ordered().then(|| update(ValidityAction::Confirm))),
    ]
    .spacing(8)
    .wrap();
    let footer = container(actions).align_right(Length::Fill);
    settings_modal_layout(header, body, Some(footer.into()))
        .style(schedule_popup_style)
        .width(calendar::POPOVER_WIDTH)
        .height(402)
        .into()
}
