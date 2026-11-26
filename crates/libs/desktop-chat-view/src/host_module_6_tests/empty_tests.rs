use crate::empty::{EmptySessionInput, empty_session_placeholder, session_loading_placeholder};
fn input(work_mode: bool, branch: Option<&str>) -> EmptySessionInput {
    let i18n = desktop_i18n::I18n::new(config_ui::DesktopLocale::SimplifiedChinese);
    EmptySessionInput::from_context(
        i18n,
        None,
        10,
        work_mode,
        branch.map(|branch| (branch.to_owned(), "刚刚".to_owned())),
    )
}

fn scene_selector(_: bool) -> iced::Element<'static, ()> {
    iced::widget::button(iced::widget::text("办公")).on_press(()).into()
}
#[test]
fn session_loading_placeholder_produces_element_with_default_app() {
    let _element: iced::Element<'_, ()> = session_loading_placeholder("正在恢复会话");
}
#[test]
fn session_loading_placeholder_produces_element_with_context() {
    let context = input(false, Some("main"));
    let _empty = empty_session_placeholder(context, scene_selector);
    let _element: iced::Element<'_, ()> = session_loading_placeholder("正在恢复会话");
}
#[test]
fn empty_session_placeholder_produces_element_with_default_app() {
    let _element = empty_session_placeholder(input(false, None), scene_selector);
}
#[test]
fn empty_session_placeholder_produces_element_with_context() {
    let _element = empty_session_placeholder(input(true, Some("develop")), scene_selector);
}
