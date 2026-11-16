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
fn bottom_bar_places_workflow_after_full_access_and_before_session_tabs() {
    let controls = include_str!("../input_panel/send_controls.rs");
    let workflow_push = controls.find("bar = bar.push(workflow_btn)").expect("workflow push");
    let permission_push = controls.find("bar = bar.push(permission_btn)").expect("permission push");
    let agent_push = controls.find("bar = bar.push(agent_tab_btn)").expect("agent tab push");
    assert!(permission_push < workflow_push && workflow_push < agent_push);
    assert!(!controls.contains("bar = bar.push(goal_btn)"));
    assert!(!controls.contains("bar = bar.push(subagent_btn)"));
    let source = include_str!("../input_panel/panel.rs");
    for identity in [
        "SessionToolSelectorTab::Agent => Icon::Hexagon",
        "SessionToolSelectorTab::Tools => Icon::Terminal",
        "SessionToolSelectorTab::Skills => Icon::Star",
    ] {
        assert!(source.contains(identity));
    }
    let normal = bottom_bar(
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        spacer(),
        button(text("model")).into(),
        button(text("usage")).into(),
        button(text("attach")).into(),
        Some(spacer()),
        Some(spacer()),
        Some(spacer()),
        false,
    );
    let children = normal.as_widget().children();
    // permission + workflow + agent + tools + skills + acp + reasoning + model + fill + utility + actions
    assert_eq!(children.len(), 11);
    assert_eq!(children[9].children.len(), 2);
    assert_eq!(children[10].children.len(), 3);
}
