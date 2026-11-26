use crate::input_panel::send_controls::bottom_bar;
use crate::input_panel::send_message::Message;
use iced::{
    Element,
    widget::{Space, button, text},
};
fn spacer() -> Element<'static, Message> {
    Space::new().into()
}
#[test]
fn bottom_bar_groups_session_tabs_apart_from_actions() {
    let source = include_str!("../input_panel/panel.rs");
    for identity in [
        "SessionToolSelectorTab::Agent => ComposerIcon::Agent",
        "SessionToolSelectorTab::Tools => ComposerIcon::Tools",
        "SessionToolSelectorTab::Skills => ComposerIcon::Skill",
    ] {
        assert!(source.contains(identity));
    }
    let normal = bottom_bar(
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        spacer(),
        button(text("model")).into(),
        button(text("usage")).into(),
        button(text("attach")).into(),
        None,
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        false,
    );
    let children = normal.as_widget().children();
    assert_eq!(children.len(), 3);
    assert_eq!(children[1].children.len(), 5);
    assert_eq!(children[2].children.len(), 5);
}
