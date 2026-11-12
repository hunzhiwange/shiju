use super::*;
use chrono::{Datelike, NaiveDate, NaiveTime};
fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).unwrap()
}
fn picker() -> DateTimePicker {
    DateTimePicker::new(false, date(2024, 2, 29), NaiveTime::from_hms_opt(9, 30, 0).unwrap())
}
#[test]
fn wb_schedule_picker_cross_month() {
    let mut p = picker();
    p.navigate(1);
    assert_eq!(p.month, date(2024, 3, 1));
    p.navigate(-1);
    assert_eq!(p.month, date(2024, 2, 1));
}
#[test]
fn wb_schedule_picker_cross_year() {
    let mut p = picker();
    p.navigate(-12);
    assert_eq!(p.month, date(2023, 2, 1));
    p.navigate(24);
    assert_eq!(p.month, date(2025, 2, 1));
}
#[test]
fn wb_schedule_picker_leap_year() {
    let p = picker();
    assert!(p.days().contains(&Some(date(2024, 2, 29))));
    let mut p = p;
    p.navigate(12);
    assert_eq!(p.days().iter().flatten().filter(|d| d.month() == 2).count(), 28);
}
#[test]
fn wb_schedule_picker_monday_complete_weeks() {
    let p = picker();
    let days = p.days();
    assert_eq!(days[0], Some(date(2024, 1, 29)));
    assert_eq!(days.len() % 7, 0);
    assert_eq!(days.last().unwrap().unwrap().weekday(), chrono::Weekday::Sun);
}
#[test]
fn wb_schedule_picker_select_adjacent_date() {
    let mut p = picker();
    p.select(date(2024, 3, 2));
    assert_eq!(p.date, date(2024, 3, 2));
    assert_eq!(p.month, date(2024, 3, 1));
}
#[test]
fn wb_schedule_picker_time_range() {
    let mut p = picker();
    p.set_time(23, 59);
    assert_eq!(p.time, NaiveTime::from_hms_opt(23, 59, 0).unwrap());
    p.set_time(24, 0);
    p.set_time(0, 60);
    assert_eq!(p.time, NaiveTime::from_hms_opt(23, 59, 0).unwrap());
}
#[test]
fn wb_schedule_picker_boundary_navigation() {
    let mut p = picker();
    p.month = NaiveDate::MIN.with_day(1).unwrap();
    let before = p.month;
    p.navigate(-1);
    assert_eq!(p.month, before);
    assert_eq!(p.days().len() % 7, 0);
    p.navigate(i32::MAX);
    assert_eq!(p.month, before);
}

#[tokio::test]
async fn wb_schedule_picker_backdrop_and_escape_cancel() {
    use iced::advanced::{Layout, Shell, clipboard, layout, renderer::Headless, widget::Tree};
    use iced::keyboard::{self, Key, key::Named};
    use iced::{Event, Font, Point, Rectangle, Renderer, Size, mouse};
    let renderer = Renderer::new(Font::DEFAULT, 16.0.into(), Some("tiny-skia")).await.unwrap();
    for keyboard in [false, true] {
        let p = picker();
        let mut element = overlay(&p, "UTC", iced::widget::text("草稿").into());
        let mut tree = Tree::new(element.as_widget());
        let viewport = Rectangle::with_size(Size::new(420.0, 460.0));
        let node = element.as_widget_mut().layout(
            &mut tree,
            &renderer,
            &layout::Limits::new(Size::ZERO, viewport.size()),
        );
        let event = if keyboard {
            Event::Keyboard(keyboard::Event::KeyPressed {
                key: Key::Named(Named::Escape),
                modified_key: Key::Named(Named::Escape),
                physical_key: keyboard::key::Physical::Unidentified(
                    keyboard::key::NativeCode::Unidentified,
                ),
                location: keyboard::Location::Standard,
                modifiers: keyboard::Modifiers::empty(),
                text: None,
                repeat: false,
            })
        } else {
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left))
        };
        let mut messages = vec![];
        element.as_widget_mut().update(
            &mut tree,
            &event,
            Layout::new(&node),
            mouse::Cursor::Available(Point::new(1.0, 1.0)),
            &renderer,
            &mut clipboard::Null,
            &mut Shell::new(&mut messages),
            &viewport,
        );
        assert!(matches!(
            messages.as_slice(),
            [Message::Settings(SettingsMessage::CronPickerCancel)]
        ));
    }
}
