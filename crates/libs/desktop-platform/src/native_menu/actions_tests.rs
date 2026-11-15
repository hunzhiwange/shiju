use super::*;
use crate::menu_actions::NativeMenuAction;
#[test]
fn native_menu_action_ids_round_trip() {
    let action = NativeMenuAction::ViewWorkBrowser;
    assert_eq!(NativeMenuAction::from_id(action.id()), Some(action));
    assert!(NativeMenuAction::from_id("native.unknown").is_none());
}
#[test]
fn project_display_label_uses_basename_or_fallback() {
    assert_eq!(project_display_label(None), "未打开项目");
    assert_eq!(project_display_label(Some("")), "未打开项目");
    assert_eq!(project_display_label(Some("/Users/me/work/vibe-window")), "vibe-window");
}
#[test]
fn window_tab_menu_ids_round_trip() {
    let id = window_tab_menu_id("project_/repo/alpha");
    assert_eq!(parse_window_tab_menu_id(&id), Some("project_/repo/alpha"));
    assert!(parse_window_tab_menu_id("native.window.other").is_none());
    assert!(parse_window_tab_menu_id("native.window.tab.").is_none());
}
#[test]
fn window_tab_title_prefers_project_and_title() {
    assert_eq!(
        window_tab_title("README.md", Some("/Users/me/work/vibe-window")),
        "vibe-window — README.md"
    );
    assert_eq!(
        window_tab_title("vibe-window", Some("/Users/me/work/vibe-window")),
        "vibe-window"
    );
    assert_eq!(window_tab_title("首页", None), "首页");
}
#[test]
fn native_menu_action_ids_are_unique() {
    let ids = [
        NativeMenuAction::FileNewSession.id(),
        NativeMenuAction::FileSave.id(),
        NativeMenuAction::EditCopy.id(),
        NativeMenuAction::ViewWorkBrowser.id(),
        NativeMenuAction::HelpAbout.id(),
    ];
    let mut sorted = ids.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), ids.len());
}
