use crate::components::preview_panel::view::{HeaderMessage, header_tabs::build_header_tabs};
use iced::advanced::{clipboard, renderer::Headless};
use iced::{Element, Event, Font, Pixels, Point, Renderer, Size, mouse};
use iced_runtime::user_interface::{Cache, UserInterface};
fn keep(element: Element<'_, HeaderMessage>) {
    std::hint::black_box(element);
}
fn i18n() -> desktop_i18n::I18n {
    desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)
}
#[test]
fn header_tabs_builds_for_empty_tabs() {
    keep(build_header_tabs(std::iter::empty(), None, None, None, i18n()));
}
#[test]
fn header_tabs_builds_selected_and_unselected_tabs() {
    keep(build_header_tabs(
        [("/tmp/a.rs", "a.rs"), ("/tmp/b.ts", "b.ts")],
        Some("/tmp/a.rs"),
        None,
        None,
        i18n(),
    ));
}
#[test]
fn header_tabs_builds_context_menu_overlay_for_target_tab() {
    keep(build_header_tabs(
        [("/tmp/a.rs", "a.rs"), ("/tmp/b.ts", "b.ts")],
        Some("/tmp/a.rs"),
        Some("/tmp/b.ts"),
        Some(Point::new(12.0, 24.0)),
        i18n(),
    ));
}
#[tokio::test(flavor = "multi_thread")]
async fn right_click_opens_the_overflow_and_selects_a_hidden_file() {
    let mut renderer =
        <Renderer as Headless>::new(Font::DEFAULT, Pixels(16.0), Some("tiny-skia")).await.unwrap();
    let mut ui = UserInterface::build(
        build_header_tabs(
            [("/tmp/a.rs", "a.rs"), ("/tmp/b.rs", "b.rs"), ("/tmp/c.rs", "c.rs")],
            Some("/tmp/c.rs"),
            None,
            None,
            i18n(),
        ),
        Size::new(232.0, 400.0),
        Cache::new(),
        &mut renderer,
    );

    let mut messages = Vec::new();
    let (_, statuses) = ui.update(
        &[Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Right))],
        mouse::Cursor::Available(Point::new(218.0, 14.0)),
        &mut renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    assert_eq!(statuses, [iced::event::Status::Captured]);
    assert!(messages.is_empty());

    ui.update(
        &[
            Event::Mouse(mouse::Event::ButtonPressed(mouse::Button::Left)),
            Event::Mouse(mouse::Event::ButtonReleased(mouse::Button::Left)),
        ],
        mouse::Cursor::Available(Point::new(100.0, 55.0)),
        &mut renderer,
        &mut clipboard::Null,
        &mut messages,
    );
    assert!(matches!(messages.as_slice(), [HeaderMessage::Select(path)] if path == "/tmp/a.rs"));
}
