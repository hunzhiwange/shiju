use super::edit_menu::{
    BROWSER_MENU_ACTIONS, EDIT_MENU_ACTIONS, HELP_MENU_ACTIONS, MenuAction, WINDOW_MENU_ACTIONS,
};

#[test]
fn edit_menu_actions_cover_clipboard_shortcuts() {
    let by_title = |title| EDIT_MENU_ACTIONS.iter().find(|action| action.title == title);

    assert_eq!(
        by_title("Copy"),
        Some(&MenuAction {
            title: "Copy",
            selector: "copy:",
            key: "c",
            shift: false,
            control: false,
        })
    );
    assert_eq!(
        by_title("Paste"),
        Some(&MenuAction {
            title: "Paste",
            selector: "paste:",
            key: "v",
            shift: false,
            control: false,
        })
    );
    assert_eq!(
        by_title("Cut"),
        Some(&MenuAction {
            title: "Cut",
            selector: "cut:",
            key: "x",
            shift: false,
            control: false,
        })
    );
    assert_eq!(
        by_title("Select All"),
        Some(&MenuAction {
            title: "Select All",
            selector: "selectAll:",
            key: "a",
            shift: false,
            control: false,
        })
    );
    assert_eq!(
        by_title("Redo"),
        Some(&MenuAction {
            title: "Redo",
            selector: "redo:",
            key: "z",
            shift: true,
            control: false,
        })
    );
}

#[test]
fn edit_menu_actions_keep_stable_order() {
    let titles: Vec<_> = EDIT_MENU_ACTIONS.iter().map(|action| action.title).collect();
    assert_eq!(titles, ["Undo", "Redo", "Cut", "Copy", "Paste", "Select All"]);
}

#[test]
fn browser_menu_actions_cover_navigation_controls() {
    let titles: Vec<_> = BROWSER_MENU_ACTIONS.iter().map(|action| action.title).collect();
    assert_eq!(titles, ["Back", "Forward", "Reload", "Open in External Browser", "Close Page",]);
    assert_eq!(BROWSER_MENU_ACTIONS[2].key, "r");
    assert_eq!(BROWSER_MENU_ACTIONS[0].selector, "browserBack:");
    assert_eq!(BROWSER_MENU_ACTIONS[4].selector, "browserClose:");
}

#[test]
fn window_menu_actions_cover_standard_window_controls() {
    let titles: Vec<_> = WINDOW_MENU_ACTIONS.iter().map(|action| action.title).collect();
    assert_eq!(titles, ["Minimize", "Zoom", "Enter Full Screen"]);
    assert_eq!(WINDOW_MENU_ACTIONS[0].selector, "performMiniaturize:");
    assert_eq!(WINDOW_MENU_ACTIONS[0].key, "m");
    assert_eq!(WINDOW_MENU_ACTIONS[2].selector, "toggleFullScreen:");
    assert!(WINDOW_MENU_ACTIONS[2].control);
}

#[test]
fn help_menu_actions_include_about() {
    assert_eq!(HELP_MENU_ACTIONS.len(), 1);
    assert_eq!(HELP_MENU_ACTIONS[0].title, "About Vibe Window WebView");
    assert_eq!(HELP_MENU_ACTIONS[0].selector, "orderFrontStandardAboutPanel:");
}

#[test]
fn native_menu_registers_window_and_help_with_nsapp() {
    // 回归：setWindowsMenu / setHelpMenu 才会让系统注入填充、居中、平铺等项。
    let src = include_str!("edit_menu.rs");
    assert!(src.contains("setWindowsMenu"));
    assert!(src.contains("setHelpMenu"));
    assert!(src.contains("append_browser_submenu"));
    assert!(src.contains("browserBack:"));
}
