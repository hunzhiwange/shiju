use super::*;
use agent_client_protocol::SessionConfigSelectOption;

fn select(id: &str, selected: &str, choices: &[&str]) -> SessionConfigOption {
    SessionConfigOption::select(
        id.to_string(),
        id,
        selected.to_string(),
        choices
            .iter()
            .map(|value| SessionConfigSelectOption::new(value.to_string(), *value))
            .collect::<Vec<_>>(),
    )
}

#[test]
fn acp_full_access_goose_permission_control_preserves_selected_settings() {
    let original = [
        select("provider", "custom", &["custom"]),
        select("model", "a", &["a"]),
        select("mode", "auto", &["auto", "approve"]),
    ];
    let refreshed = [
        select("mode", "approve", &["auto", "approve"]),
        select("model", "a", &["a", "b"]),
        select("provider", "custom", &["custom"]),
    ];
    assert!(same_selected_settings(&original, &refreshed));
    assert!(!same_selected_settings(&original, &refreshed[..2]));
    let mut changed = refreshed;
    changed[1] = select("model", "b", &["a", "b"]);
    assert!(!same_selected_settings(&original, &changed));
}
