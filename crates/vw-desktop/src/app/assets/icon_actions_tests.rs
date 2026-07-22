#[test]
fn task_635_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("icon_actions_tests.rs"));
}

#[test]
fn register_icons_inserts_every_action_icon_without_clearing_existing_entries() {
    use super::Icon;
    use iced::widget::svg;
    use std::collections::HashMap;

    let sentinel = Icon::QuestionCircle;
    let sentinel_handle = svg::Handle::from_memory(b"<svg></svg>");
    let mut icons = HashMap::from([(sentinel, sentinel_handle)]);

    super::register_icons(&mut icons);

    for icon in [
        Icon::Save,
        Icon::ArrowCounterClockwise,
        Icon::ArrowClockwise,
        Icon::Search,
        Icon::Robot,
        Icon::Eraser,
        Icon::ArrowRepeat,
        Icon::X,
        Icon::Terminal,
        Icon::Grid1x2,
        Icon::Columns,
        Icon::SymmetryHorizontal,
        Icon::SymmetryVertical,
        Icon::ChevronUp,
        Icon::Clipboard,
        Icon::Check,
        Icon::CheckSquare,
    ] {
        assert!(icons.contains_key(&icon), "{icon:?} should be registered");
    }

    assert!(icons.contains_key(&sentinel));
    assert_eq!(icons.len(), 18);
}
