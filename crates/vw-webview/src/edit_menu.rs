//! macOS Edit 菜单：为 child WebView 提供 Cmd+C/V/X/A 等系统快捷键。
//!
//! wry 在 child webview 上会把 `performKeyEquivalent:` 返回 NO，转而依赖
//! `NSApplication.mainMenu` 的加速键。没有 Edit 菜单时复制粘贴快捷键无效。

use objc2::sel;
use objc2_app_kit::{NSApplication, NSEventModifierFlags, NSMenu, NSMenuItem};
use objc2_foundation::{MainThreadMarker, NSString};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EditMenuAction {
    pub(crate) title: &'static str,
    pub(crate) selector: &'static str,
    pub(crate) key: &'static str,
    pub(crate) shift: bool,
}

/// Edit 菜单动作列表（供测试与安装共用）。
pub(crate) const EDIT_MENU_ACTIONS: &[EditMenuAction] = &[
    EditMenuAction { title: "Undo", selector: "undo:", key: "z", shift: false },
    EditMenuAction { title: "Redo", selector: "redo:", key: "z", shift: true },
    EditMenuAction { title: "Cut", selector: "cut:", key: "x", shift: false },
    EditMenuAction { title: "Copy", selector: "copy:", key: "c", shift: false },
    EditMenuAction { title: "Paste", selector: "paste:", key: "v", shift: false },
    EditMenuAction {
        title: "Select All",
        selector: "selectAll:",
        key: "a",
        shift: false,
    },
];

/// 安装包含剪贴板快捷键的 macOS 主菜单。
pub(crate) fn install_edit_menu() {
    let Some(mtm) = MainThreadMarker::new() else {
        return;
    };

    let app = NSApplication::sharedApplication(mtm);
    let menubar = NSMenu::new(mtm);

    // 首个菜单会被系统提升为应用菜单。
    let app_menu_item = NSMenuItem::new(mtm);
    menubar.addItem(&app_menu_item);
    let app_menu = NSMenu::new(mtm);
    app_menu_item.setSubmenu(Some(&app_menu));

    let edit_menu_item = NSMenuItem::new(mtm);
    menubar.addItem(&edit_menu_item);
    let edit_menu = NSMenu::new(mtm);
    edit_menu.setTitle(&NSString::from_str("Edit"));
    edit_menu_item.setSubmenu(Some(&edit_menu));

    for (index, action) in EDIT_MENU_ACTIONS.iter().enumerate() {
        if index == 2 {
            edit_menu.addItem(&NSMenuItem::separatorItem(mtm));
        }
        add_edit_item(&edit_menu, action);
    }

    app.setMainMenu(Some(&menubar));
}

fn add_edit_item(edit_menu: &NSMenu, action: &EditMenuAction) {
    let selector = match action.selector {
        "undo:" => sel!(undo:),
        "redo:" => sel!(redo:),
        "cut:" => sel!(cut:),
        "copy:" => sel!(copy:),
        "paste:" => sel!(paste:),
        "selectAll:" => sel!(selectAll:),
        _ => return,
    };

    let item = unsafe {
        edit_menu.addItemWithTitle_action_keyEquivalent(
            &NSString::from_str(action.title),
            Some(selector),
            &NSString::from_str(action.key),
        )
    };

    if action.shift {
        item.setKeyEquivalentModifierMask(
            NSEventModifierFlags::Command | NSEventModifierFlags::Shift,
        );
    }
}
