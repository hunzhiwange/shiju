#[test]
fn task_639_test_module_is_wired() {
    let path = std::path::Path::new(file!());
    assert!(path.ends_with("icon_ui_tests.rs"));
}

use super::{Icon, register_icons};
use iced::widget::svg;
use std::collections::HashMap;

const REGISTERED_UI_ICONS: &[Icon] = &[
    Icon::QuestionCircle,
    Icon::ArrowLeft,
    Icon::ArrowRight,
    Icon::Globe,
    Icon::Back,
    Icon::Home,
    Icon::GitBranch,
    Icon::Clock,
    Icon::ShieldLock,
    Icon::TypeBold,
    Icon::TypeItalic,
    Icon::TypeUnderline,
    Icon::TypeStrikethrough,
    Icon::TextLeft,
    Icon::TextCenter,
    Icon::TextRight,
    Icon::AlignTop,
    Icon::AlignMiddle,
    Icon::AlignBottom,
    Icon::BorderStyle,
    Icon::Code,
    Icon::LayoutSidebar,
    Icon::LayoutSidebarReverse,
    Icon::Funnel,
    Icon::Eye,
    Icon::EyeSlash,
];

#[test]
fn register_icons_inserts_every_ui_icon_and_preserves_existing_entries() {
    let mut icons = HashMap::new();
    icons.insert(Icon::Logo, svg::Handle::from_memory(Vec::new()));

    register_icons(&mut icons);

    for icon in REGISTERED_UI_ICONS {
        assert!(icons.contains_key(icon), "missing registered ui icon: {icon:?}");
    }
    assert!(icons.contains_key(&Icon::Logo));
    assert_eq!(icons.len(), REGISTERED_UI_ICONS.len() + 1);
}

#[test]
fn register_icons_can_be_called_again_without_changing_key_set() {
    let mut icons = HashMap::new();

    register_icons(&mut icons);
    let first_keys = icons.keys().copied().collect::<std::collections::HashSet<_>>();
    register_icons(&mut icons);
    let second_keys = icons.keys().copied().collect::<std::collections::HashSet<_>>();

    assert_eq!(first_keys, second_keys);
    assert_eq!(second_keys.len(), REGISTERED_UI_ICONS.len());
}
