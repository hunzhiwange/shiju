//! 宿主仅保留会话工具页签的消息装配契约。
#[test]
fn bottom_bar_maps_session_tabs_to_host_actions() {
    let source = include_str!("mod.rs");
    assert!(!source.contains("goal_btn"));
    for tab in ["Agent", "Tools", "Skills"] {
        assert!(source.contains(&format!("SessionToolSelectorTab::{tab}")));
        assert!(source.contains("OpenSessionToolSelectorTab"));
    }
    assert!(!source.contains("\"会话控制\""));
}
