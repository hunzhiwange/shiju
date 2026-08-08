use super::edit_menu::{EDIT_MENU_ACTIONS, EditMenuAction};

#[test]
fn edit_menu_actions_cover_clipboard_shortcuts() {
    let by_title = |title| EDIT_MENU_ACTIONS.iter().find(|action| action.title == title);

    assert_eq!(
        by_title("Copy"),
        Some(&EditMenuAction {
            title: "Copy",
            selector: "copy:",
            key: "c",
            shift: false,
        })
    );
    assert_eq!(
        by_title("Paste"),
        Some(&EditMenuAction {
            title: "Paste",
            selector: "paste:",
            key: "v",
            shift: false,
        })
    );
    assert_eq!(
        by_title("Cut"),
        Some(&EditMenuAction {
            title: "Cut",
            selector: "cut:",
            key: "x",
            shift: false,
        })
    );
    assert_eq!(
        by_title("Select All"),
        Some(&EditMenuAction {
            title: "Select All",
            selector: "selectAll:",
            key: "a",
            shift: false,
        })
    );
    assert_eq!(
        by_title("Redo"),
        Some(&EditMenuAction {
            title: "Redo",
            selector: "redo:",
            key: "z",
            shift: true,
        })
    );
}

#[test]
fn edit_menu_actions_keep_stable_order() {
    let titles: Vec<_> = EDIT_MENU_ACTIONS.iter().map(|action| action.title).collect();
    assert_eq!(titles, ["Undo", "Redo", "Cut", "Copy", "Paste", "Select All"]);
}
