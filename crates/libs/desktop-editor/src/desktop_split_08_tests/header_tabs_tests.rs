use crate::components::preview_panel::view::{HeaderMessage, header_tabs::build_header_tabs};
use iced::{Element, Point};
fn keep(element: Element<'_, HeaderMessage>) {
    std::hint::black_box(element);
}
fn i18n() -> desktop_i18n::I18n {
    desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)
}
#[test]
fn header_tabs_builds_for_empty_tabs() {
    keep(build_header_tabs(
        std::iter::empty(),
        None,
        None,
        None,
        iced::widget::Id::unique(),
        false,
        i18n(),
    ));
}
#[test]
fn header_tabs_builds_selected_and_unselected_tabs() {
    keep(build_header_tabs(
        [("/tmp/a.rs", "a.rs"), ("/tmp/b.ts", "b.ts")],
        Some("/tmp/a.rs"),
        None,
        None,
        iced::widget::Id::unique(),
        false,
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
        iced::widget::Id::unique(),
        false,
        i18n(),
    ));
}
#[test]
fn header_tabs_uses_taller_scrollbar_container_when_tabs_overflow() {
    let paths: Vec<_> = (0..8).map(|i| format!("/tmp/very-long-file-name-{i}.rs")).collect();
    keep(build_header_tabs(
        paths.iter().map(|path| (path.as_str(), path.rsplit('/').next().unwrap())),
        Some("/tmp/very-long-file-name-0.rs"),
        None,
        None,
        iced::widget::Id::unique(),
        true,
        i18n(),
    ));
}
