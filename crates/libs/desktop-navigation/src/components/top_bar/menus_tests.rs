use super::menus::*;
use super::types::WorkspaceMode;
use crate::presentation::WorkResourceMode;
fn item(entry: &MenuEntry) -> (&str, Option<&str>, &Message) {
    match entry {
        MenuEntry::Item { label, shortcut, message, .. } => (*label, *shortcut, message.as_ref()),
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
        Message::ProjectFileNewSession
    ));
    assert!(matches!(
        item(&entries[1]).2,
        Message::ProjectFileNewProject
    ));
    assert!(matches!(
        item(&entries[2]).2,
        Message::ProjectFileShowSessions
    ));
    assert!(matches!(
        item(&entries[3]).2,
        Message::ProjectFileShowProjects
    ));
    assert!(matches!(item(&entries[5]).2, Message::SaveFile));
    assert!(matches!(item(&entries[6]).2, Message::ProjectFileSaveAll));
}

#[test]
fn non_project_file_menu_entries_offer_save_and_home_actions() {
    let entries = file_menu_entries(false);

    assert_eq!(entries.len(), 2);
    assert_eq!((item(&entries[0]).0, item(&entries[0]).1), ("保存代码", Some("⌘S")));
    assert!(matches!(item(&entries[0]).2, Message::SaveFile));
    assert_eq!((item(&entries[1]).0, item(&entries[1]).1), ("返回首页", None));
    assert!(matches!(item(&entries[1]).2, Message::GoHome));
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

    assert!(matches!(item(&entries[0]).2, Message::Undo));
    assert!(matches!(item(&entries[1]).2, Message::Redo));
    assert!(matches!(item(&entries[3]).2, Message::CutShortcut));
    assert!(matches!(item(&entries[4]).2, Message::CopyShortcut));
    assert!(matches!(item(&entries[5]).2, Message::PasteShortcut));
    assert!(matches!(
        item(&entries[7]).2,
        Message::OpenSearch
    ));
    assert!(matches!(
        item(&entries[8]).2,
        Message::OpenReplace
    ));
    assert!(matches!(
        item(&entries[10]).2,
        Message::FileTreeFindInProject
    ));
    assert!(matches!(
        item(&entries[11]).2,
        Message::FileTreeReplaceInProject
    ));
}

#[test]
fn non_project_edit_menu_entries_are_empty() {
    assert!(edit_menu_entries(false).is_empty());
}

#[test]
fn view_menu_entries_request_visible_panel_and_changed_files() {
    let entries = view_menu_entries(WorkspaceMode::Code, false, false);

    assert_eq!(entries.len(), if cfg!(target_arch = "wasm32") { 4 } else { 5 });
    assert_eq!(item(&entries[0]).0, "切换左侧面板");
    assert!(matches!(
        item(&entries[0]).2,
        Message::ToggleSettingsPanel
    ));
    assert_eq!(item(&entries[1]).0, "显示右侧面板");
    assert!(matches!(
        item(&entries[1]).2,
        Message::FileManagerPanelVisible(true)
    ));
    assert_eq!(item(&entries[2]).0, "切换到更改");
    assert!(matches!(
        item(&entries[2]).2,
        Message::FileManagerShowChanges(true)
    ));
    assert_eq!(item(&entries[3]).0, "切换 Diff 面板");
    assert!(matches!(item(&entries[3]).2, Message::ToggleDiffPanel));
    #[cfg(not(target_arch = "wasm32"))]
    {
        assert_eq!(item(&entries[4]).0, "切换终端面板");
        assert!(matches!(
            item(&entries[4]).2,
            Message::ToggleTerminalPanel
        ));
    }
    #[cfg(target_arch = "wasm32")]
    assert!(entries.iter().all(|entry| !matches!(
        item(entry).2,
        Message::ToggleTerminalPanel
    )));
}

#[test]
fn view_menu_entries_request_hidden_panel_and_all_files() {
    let entries = view_menu_entries(WorkspaceMode::Code, true, true);

    assert_eq!(entries.len(), if cfg!(target_arch = "wasm32") { 4 } else { 5 });
    assert_eq!(item(&entries[1]).0, "隐藏右侧面板");
    assert!(matches!(
        item(&entries[1]).2,
        Message::FileManagerPanelVisible(false)
    ));
    assert_eq!(item(&entries[2]).0, "切换到全部文件");
    assert!(matches!(
        item(&entries[2]).2,
        Message::FileManagerShowChanges(false)
    ));
}

#[test]
fn work_view_menu_exposes_only_work_mode_surfaces() {
    let entries = view_menu_entries(WorkspaceMode::Work, true, true);

    assert_eq!(entries.len(), 4);
    assert_eq!(item(&entries[0]).0, "切换工作侧栏");
    assert!(matches!(item(&entries[0]).2, Message::WorkSidebarToggled));
    for (entry, expected_mode) in entries[1..].iter().zip([
        WorkResourceMode::Overview,
        WorkResourceMode::WorkspaceFiles,
        WorkResourceMode::Browser,
    ]) {
        assert!(matches!(
            item(entry).2,
            Message::WorkResourceModeSelected(mode)
                if *mode == expected_mode
        ));
    }
    assert!(
        entries.iter().all(|entry| {
            !matches!(
                item(entry).2,
                Message::ToggleTerminalPanel
                        | Message::ToggleDiffPanel

            )
        }),
        "work mode must not expose terminal or review actions"
    );
}

#[test]
fn help_menu_entries_preserve_separator_and_action_mapping() {
    let entries = help_menu_entries();

    assert_eq!(entries.len(), if cfg!(target_arch = "wasm32") { 5 } else { 6 });
    assert_eq!(item(&entries[0]).0, "关于");
    assert!(matches!(item(&entries[0]).2, Message::ToggleAboutModal));
    assert_separator(&entries[1]);
    assert_eq!(item(&entries[2]).0, "重启");
    assert!(matches!(item(&entries[2]).2, Message::RestartApp));
    assert_eq!(item(&entries[3]).0, "安装 CLI 命令工具");
    assert!(matches!(item(&entries[3]).2, Message::InstallCliTool));
    #[cfg(not(target_arch = "wasm32"))]
    {
        assert_eq!(item(&entries[4]).0, "安装 Web 版本");
        assert!(matches!(
            item(&entries[4]).2,
            Message::InstallWeb
        ));
    }
    let update_index = entries.len() - 1;
    assert_eq!(item(&entries[update_index]).0, "检测更新");
    assert!(matches!(
        item(&entries[update_index]).2,
        Message::OpenAppUpdateModal
    ));
}
