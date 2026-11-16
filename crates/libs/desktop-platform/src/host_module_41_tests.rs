use crate::subscriptions::Activity;

#[test]
fn mini_companion_hidden_stops_avatar_and_expansion_ticks() {
    let mut activity =
        Activity { companion_visible: true, companion_animating: true, ..Default::default() };
    assert!(activity.needs_tick());
    activity.companion_visible = false;
    assert!(!activity.needs_tick());
    activity.companion_visible = true;
    activity.companion_animating = false;
    assert!(!activity.needs_tick());
    activity.companion_animating = true;
    assert!(activity.needs_tick());
}

#[test]
fn requesting_sessions_animate_on_conversation_screen_and_stop_when_idle() {
    let mut activity = Activity::default();
    assert!(!activity.needs_tick());
    activity.conversation_requesting = true;
    assert!(activity.needs_tick());
    activity.conversation_requesting = false;
    assert!(!activity.needs_tick());
    activity.startup_transition_active = true;
    assert!(activity.needs_tick());
}
