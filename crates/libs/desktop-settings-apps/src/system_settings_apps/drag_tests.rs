use super::drag::*;
fn ids(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| v.to_string()).collect()
}
#[test]
fn apps_drag_reorder_preserves_uninstalled_and_hidden_slots_in_both_directions() {
    let order = ids(&["claude", "not-installed", "codex", "hidden-installed", "gemini"]);
    let movable = ids(&["claude", "codex", "gemini"]);
    assert_eq!(
        reordered(&order, &movable, "claude", "gemini"),
        Some(ids(&["codex", "not-installed", "gemini", "hidden-installed", "claude"]))
    );
    assert_eq!(
        reordered(&order, &movable, "gemini", "claude"),
        Some(ids(&["gemini", "not-installed", "claude", "hidden-installed", "codex"]))
    );
    assert!(reordered(&order, &movable, "claude", "not-installed").is_none());
    assert!(reordered(&order, &movable, "claude", "claude").is_none());
}
