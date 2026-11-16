use crate::input_panel::input_menu::{Action, update};
use desktop_session::message::chat::input::context_menus::InputContextMenuState;
use iced::widget::text_editor;
#[test]
fn input_context_menu_cut_without_selection_only_closes_menu() {
    let mut menu = InputContextMenuState::default();
    let mut editor = text_editor::Content::with_text("unchanged");
    menu.open(2.0, 3.0);

    let _task = update(&mut menu, &mut editor, Action::Cut);

    assert!(!menu.open);
    assert_eq!(menu.position, None);
    assert_eq!(editor.text(), "unchanged");
}

#[test]
fn input_context_menu_copy_without_selection_only_closes_menu() {
    let mut menu = InputContextMenuState::default();
    let mut editor = text_editor::Content::with_text("unchanged");
    menu.open(2.0, 3.0);

    let _task = update(&mut menu, &mut editor, Action::Copy);

    assert!(!menu.open);
    assert_eq!(menu.position, None);
    assert_eq!(editor.text(), "unchanged");
}

#[test]
fn close_input_context_menu_clears_open_state_and_position() {
    let mut menu = InputContextMenuState::default();
    menu.open = true;
    menu.position = Some((12.0, 24.0));

    menu.close();

    assert!(!menu.open);
    assert_eq!(menu.position, None);
}
