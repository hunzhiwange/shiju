use super::menus::{
    MenuEntry, edit_menu, edit_menu_entries, file_menu, file_menu_entries, help_menu,
    help_menu_entries, view_menu, view_menu_entries,
};
use crate::app::message::view::MenuType;
use crate::app::{App, Message, Screen, message};

fn test_app(screen: Screen) -> App {
    let (mut app, _task) = App::new();
    app.screen = screen;
    app.active_menu = None;
    app
}

fn item(entry: &MenuEntry) -> (&str, Option<&str>, &Message) {
    match entry {
        MenuEntry::Item { label, shortcut, message } => (*label, *shortcut, message.as_ref()),
        MenuEntry::Separator => panic!("expected menu item"),
    }
}

fn assert_separator(entry: &MenuEntry) {
    assert!(matches!(entry, MenuEntry::Separator));
}

#[test]
fn project_file_menu_entries_preserve_order_shortcuts_and_actions() {
    let entries = file_menu_entries(true);

    assert_eq!(entries.len(), 7);
    let expected = [("新建会话", None), ("新建项目", None), ("查看会话", None), ("查看项目", None)];
    for (entry, expected) in entries[..4].iter().zip(expected) {
        let (label, shortcut, _) = item(entry);
        assert_eq!((label, shortcut), expected);
    }
    assert_separator(&entries[4]);
    assert_eq!(item(&entries[5]).0, "保存文件");
    assert_eq!(item(&entries[5]).1, Some("⌘S"));
    assert_eq!(item(&entries[6]).0, "保存所有文件");
    assert_eq!(item(&entries[6]).1, Some("⌥⌘S"));

    assert!(matches!(
        item(&entries[0]).2,
        Message::View(message::ViewMessage::ProjectFileNewSession)
    ));
    assert!(matches!(
        item(&entries[1]).2,
        Message::View(message::ViewMessage::ProjectFileNewProject)
    ));
    assert!(matches!(
        item(&entries[2]).2,
        Message::View(message::ViewMessage::ProjectFileShowSessions)
    ));
    assert!(matches!(
        item(&entries[3]).2,
        Message::View(message::ViewMessage::ProjectFileShowProjects)
    ));
    assert!(matches!(item(&entries[5]).2, Message::Preview(message::PreviewMessage::SaveFile)));
    assert!(matches!(item(&entries[6]).2, Message::View(message::ViewMessage::ProjectFileSaveAll)));
}

#[test]
fn non_project_file_menu_entries_offer_save_and_home_actions() {
    let entries = file_menu_entries(false);

    assert_eq!(entries.len(), 2);
    assert_eq!((item(&entries[0]).0, item(&entries[0]).1), ("保存代码", Some("⌘S")));
    assert!(matches!(item(&entries[0]).2, Message::Preview(message::PreviewMessage::SaveFile)));
    assert_eq!((item(&entries[1]).0, item(&entries[1]).1), ("返回首页", None));
    assert!(matches!(item(&entries[1]).2, Message::View(message::ViewMessage::GoHome)));
}

#[test]
fn project_edit_menu_entries_preserve_groups_shortcuts_and_actions() {
    let entries = edit_menu_entries(true);

    assert_eq!(entries.len(), 12);
    assert_separator(&entries[2]);
    assert_separator(&entries[6]);
    assert_separator(&entries[9]);
    let expected = [
        (0, "撤销", Some("⌘Z")),
        (1, "重做", Some("⇧⌘Z")),
        (3, "剪切", Some("⌘X")),
        (4, "复制", Some("⌘C")),
        (5, "粘贴", Some("⌘V")),
        (7, "搜索", Some("⌘F")),
        (8, "替换", Some("⌥⌘F")),
        (10, "在文件中搜索", None),
        (11, "在文件中替换", None),
    ];
    for (index, label, shortcut) in expected {
        assert_eq!((item(&entries[index]).0, item(&entries[index]).1), (label, shortcut));
    }

    assert!(matches!(item(&entries[0]).2, Message::Editor(message::editor::EditorMessage::Undo)));
    assert!(matches!(item(&entries[1]).2, Message::Editor(message::editor::EditorMessage::Redo)));
    assert!(matches!(item(&entries[3]).2, Message::Editor(message::editor::EditorMessage::Cut)));
    assert!(matches!(item(&entries[4]).2, Message::Editor(message::editor::EditorMessage::Copy)));
    assert!(matches!(item(&entries[5]).2, Message::Editor(message::editor::EditorMessage::Paste)));
    assert!(matches!(
        item(&entries[7]).2,
        Message::Editor(message::editor::EditorMessage::OpenSearch)
    ));
    assert!(matches!(
        item(&entries[8]).2,
        Message::Editor(message::editor::EditorMessage::OpenReplace)
    ));
    assert!(matches!(
        item(&entries[10]).2,
        Message::Project(message::ProjectMessage::FileTreeFindInProject)
    ));
    assert!(matches!(
        item(&entries[11]).2,
        Message::Project(message::ProjectMessage::FileTreeReplaceInProject)
    ));
}

#[test]
fn non_project_edit_menu_entries_are_empty() {
    assert!(edit_menu_entries(false).is_empty());
}

#[test]
fn view_menu_entries_request_visible_panel_and_changed_files() {
    let entries = view_menu_entries(false, false);

    assert_eq!(entries.len(), 5);
    assert_eq!(item(&entries[0]).0, "切换左侧面板");
    assert!(matches!(
        item(&entries[0]).2,
        Message::View(message::ViewMessage::ToggleSettingsPanel)
    ));
    assert_eq!(item(&entries[1]).0, "显示右侧面板");
    assert!(matches!(
        item(&entries[1]).2,
        Message::View(message::ViewMessage::FileManagerPanelVisible(true))
    ));
    assert_eq!(item(&entries[2]).0, "切换到更改");
    assert!(matches!(
        item(&entries[2]).2,
        Message::Project(message::ProjectMessage::FileManagerShowChanges(true))
    ));
    assert_eq!(item(&entries[3]).0, "切换 Diff 面板");
    assert!(matches!(item(&entries[3]).2, Message::View(message::ViewMessage::ToggleDiffPanel)));
    assert_eq!(item(&entries[4]).0, "切换终端面板");
    assert!(matches!(
        item(&entries[4]).2,
        Message::View(message::ViewMessage::ToggleTerminalPanel)
    ));
}

#[test]
fn view_menu_entries_request_hidden_panel_and_all_files() {
    let entries = view_menu_entries(true, true);

    assert_eq!(entries.len(), 5);
    assert_eq!(item(&entries[1]).0, "隐藏右侧面板");
    assert!(matches!(
        item(&entries[1]).2,
        Message::View(message::ViewMessage::FileManagerPanelVisible(false))
    ));
    assert_eq!(item(&entries[2]).0, "切换到全部文件");
    assert!(matches!(
        item(&entries[2]).2,
        Message::Project(message::ProjectMessage::FileManagerShowChanges(false))
    ));
}

#[test]
fn help_menu_entries_preserve_separator_and_action_mapping() {
    let entries = help_menu_entries();

    assert_eq!(entries.len(), 5);
    assert_eq!(item(&entries[0]).0, "关于");
    assert!(matches!(item(&entries[0]).2, Message::View(message::ViewMessage::ToggleAboutModal)));
    assert_separator(&entries[1]);
    assert_eq!(item(&entries[2]).0, "重启");
    assert!(matches!(item(&entries[2]).2, Message::View(message::ViewMessage::RestartApp)));
    assert_eq!(item(&entries[3]).0, "安装 CLI 命令工具");
    assert!(matches!(item(&entries[3]).2, Message::View(message::ViewMessage::InstallCliTool)));
    assert_eq!(item(&entries[4]).0, "检测更新");
    assert!(matches!(item(&entries[4]).2, Message::View(message::ViewMessage::OpenAppUpdateModal)));
}

#[test]
fn file_menu_renders_screen_specific_items() {
    for screen in [Screen::Project, Screen::Home] {
        let mut app = test_app(screen);
        app.active_menu = Some(MenuType::File);
        let _element = file_menu(&app);
    }
}

#[test]
fn file_menu_renders_when_file_menu_is_closed() {
    let mut app = test_app(Screen::Project);
    app.active_menu = None;

    let _element = file_menu(&app);
}

#[test]
fn edit_menu_renders_project_and_empty_fallback() {
    for screen in [Screen::Project, Screen::Home] {
        let mut app = test_app(screen);
        app.active_menu = Some(MenuType::Edit);
        let _element = edit_menu(&app);
    }
}

#[test]
fn edit_menu_renders_project_when_menu_is_closed() {
    let mut app = test_app(Screen::Project);
    app.active_menu = None;

    let _element = edit_menu(&app);
}

#[test]
fn view_menu_renders_panel_and_mode_toggle_states() {
    let mut app = test_app(Screen::Project);
    app.active_menu = Some(MenuType::View);
    app.show_file_manager = false;
    app.file_manager_show_changes = false;
    let _element = view_menu(&app);
    drop(_element);

    app.show_file_manager = true;
    app.file_manager_show_changes = true;
    let _element = view_menu(&app);
}

#[test]
fn help_menu_renders_available_actions() {
    let mut app = test_app(Screen::Home);
    app.active_menu = Some(MenuType::Help);
    let _element = help_menu(&app);
}

#[test]
fn help_menu_renders_closed_state() {
    let app = test_app(Screen::Home);

    let _element = help_menu(&app);
}
