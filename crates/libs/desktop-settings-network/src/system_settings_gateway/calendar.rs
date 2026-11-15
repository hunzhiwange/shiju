use super::*;

pub(super) fn parse_skey_date(raw: &str) -> Option<NaiveDate> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    NaiveDate::parse_from_str(trimmed, "%Y-%m-%d").ok().or_else(|| {
        chrono::DateTime::parse_from_rfc3339(trimmed).ok().map(|value| value.date_naive())
    })
}

pub(super) fn month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).unwrap_or(date)
}

pub(super) fn parse_calendar_month(raw: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(&format!("{}-01", raw.trim()), "%Y-%m-%d").ok()
}

pub(super) fn skey_calendar_month(state: &GatewaySettingsState) -> NaiveDate {
    parse_calendar_month(&state.new_skey_calendar_month)
        .or_else(|| parse_skey_date(&state.new_skey_expires_at_input).map(month_start))
        .unwrap_or_else(|| month_start(Utc::now().date_naive()))
}

pub(super) fn days_in_month(year: i32, month: u32) -> u32 {
    let (next_year, next_month) = if month == 12 { (year + 1, 1) } else { (year, month + 1) };
    NaiveDate::from_ymd_opt(next_year, next_month, 1)
        .and_then(|first_next| first_next.pred_opt())
        .map(|date| date.day())
        .unwrap_or(31)
}

pub fn skey_calendar_popover<'a>(app: View<'a>) -> Element<'a, Message> {
    let s = app.gateway_settings;
    let month = skey_calendar_month(s);
    let selected_date = parse_skey_date(&s.new_skey_expires_at_input);

    let header = row![
        button(text("<"))
            .padding([6, 10])
            .on_press(Message::Gateway(
                GatewayMessage::NewSkeyExpiresMonthChanged(-1)
            ))
            .style(rounded_action_btn_style),
        text(month.format("%Y-%m").to_string())
            .size(13)
            .width(Length::Fill)
            .align_x(Alignment::Center),
        button(text(">"))
            .padding([6, 10])
            .on_press(Message::Gateway(
                GatewayMessage::NewSkeyExpiresMonthChanged(1)
            ))
            .style(rounded_action_btn_style),
        button(text(connection_text(app.desktop_locale, "永不过期")))
            .padding([6, 12])
            .on_press(Message::Gateway(
                GatewayMessage::NewSkeyExpiresAtCleared
            ))
            .style(rounded_action_btn_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .width(Length::Fill);

    let weekdays = [
        connection_text(app.desktop_locale, "一"),
        connection_text(app.desktop_locale, "二"),
        connection_text(app.desktop_locale, "三"),
        connection_text(app.desktop_locale, "四"),
        connection_text(app.desktop_locale, "五"),
        connection_text(app.desktop_locale, "六"),
        connection_text(app.desktop_locale, "日"),
    ]
    .into_iter()
    .fold(row![].spacing(6), |row, label| {
        row.push(
            container(text(label).size(11).style(settings_muted_text_style))
                .width(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE))
                .center_x(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE)),
        )
    });

    let first_weekday = month.weekday().num_days_from_monday() as i32;
    let days = days_in_month(month.year(), month.month()) as i32;
    let mut calendar = column![header, weekdays].spacing(8);
    for week in 0..(first_weekday + days + 6) / 7 {
        let mut days_row = row![].spacing(6);
        for weekday in 0..7 {
            let day = week * 7 + weekday - first_weekday + 1;
            if day < 1 || day > days {
                days_row = days_row.push(
                    container(text(""))
                        .width(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE))
                        .height(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE)),
                );
                continue;
            }

            let date =
                NaiveDate::from_ymd_opt(month.year(), month.month(), day as u32).unwrap_or(month);
            let is_selected = selected_date == Some(date);
            days_row = days_row.push(
                button(
                    text(day.to_string())
                        .size(12)
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center(),
                )
                .padding(0)
                .width(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE))
                .height(Length::Fixed(GATEWAY_SKEY_CALENDAR_CELL_SIZE))
                .on_press(Message::Gateway(
                    GatewayMessage::NewSkeyExpiresDateSelected(date.format("%Y-%m-%d").to_string()),
                ))
                .style(move |theme, status| {
                    if is_selected {
                        primary_action_btn_style(theme, status)
                    } else {
                        rounded_action_btn_style(theme, status)
                    }
                }),
            );
        }
        calendar = calendar.push(days_row);
    }

    settings_group_card(calendar.align_x(Alignment::Center))
        .padding([14, 18])
        .width(Length::Fixed(340.0))
        .into()
}

pub fn skey_date_picker<'a>(app: View<'a>) -> Element<'a, Message> {
    let s = app.gateway_settings;
    let selected_date = parse_skey_date(&s.new_skey_expires_at_input);
    let action_label = if s.new_skey_calendar_open {
        connection_text(app.desktop_locale, "收起")
    } else {
        connection_text(app.desktop_locale, "日历")
    };

    let mut trigger = row![
        settings_text_input(
            connection_text(app.desktop_locale, "YYYY-MM-DD，可手写"),
            &s.new_skey_expires_at_input
        )
        .on_input(|value| Message::Gateway(
            GatewayMessage::NewSkeyExpiresAtChanged(value)
        ))
        .width(Length::Fill),
        button(text(action_label).size(13))
            .padding([10, 12])
            .on_press(Message::Gateway(
                GatewayMessage::NewSkeyCalendarToggled
            ))
            .style(rounded_action_btn_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);
    if selected_date.is_some() {
        trigger = trigger.push(
            button(text(connection_text(app.desktop_locale, "永不过期")).size(13))
                .padding([10, 12])
                .on_press(Message::Gateway(
                    GatewayMessage::NewSkeyExpiresAtCleared,
                ))
                .style(rounded_action_btn_style),
        );
    }

    BelowOverlay::new(trigger, skey_calendar_popover(app))
        .show(s.new_skey_calendar_open)
        .gap(6.0)
        .on_close(Message::Gateway(
            GatewayMessage::NewSkeyCalendarClosed,
        ))
        .into()
}
