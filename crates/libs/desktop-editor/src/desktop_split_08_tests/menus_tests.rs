use crate::components::preview_panel::view::menus::build_menu_ui;
fn i18n() -> desktop_i18n::I18n {
    desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese)
}
#[test]
fn menu_ui_builds_empty_layers_when_hidden() {
    std::hint::black_box(build_menu_ui(false, false, i18n(), None));
}
#[test]
fn menu_ui_builds_nav_popup_for_directory_and_file_items() {
    let nav = (
        "/tmp/project".to_string(),
        10.0,
        20.0,
        vec![("src".to_string(), true), ("main.rs".to_string(), false)],
    );
    std::hint::black_box(build_menu_ui(false, false, i18n(), Some(&nav)));
}
