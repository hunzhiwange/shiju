use crate::empty::{EmptySessionInput, empty_session_placeholder, session_loading_placeholder};
fn input(path: Option<&str>, branch: Option<&str>) -> EmptySessionInput {
    let i18n = desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese);
    EmptySessionInput::from_context(
        i18n,
        None,
        10,
        false,
        crate::utils::app_state::current_project_path_label(path, i18n),
        branch.map(|branch| (branch.to_owned(), "刚刚".to_owned())),
    )
}
#[test]
fn session_loading_placeholder_produces_element_with_default_app() {
    let _element: iced::Element<'_, ()> = session_loading_placeholder("正在恢复会话");
}
#[test]
fn session_loading_placeholder_produces_element_with_context() {
    let context = input(Some("/project"), Some("main"));
    let _empty: iced::Element<'_, ()> = empty_session_placeholder(context);
    let _element: iced::Element<'_, ()> = session_loading_placeholder("正在恢复会话");
}
#[test]
fn empty_session_placeholder_produces_element_with_default_app() {
    let _element: iced::Element<'_, ()> = empty_session_placeholder(input(None, None));
}
#[test]
fn empty_session_placeholder_produces_element_with_context() {
    let _element: iced::Element<'_, ()> =
        empty_session_placeholder(input(Some("/project"), Some("develop")));
}
