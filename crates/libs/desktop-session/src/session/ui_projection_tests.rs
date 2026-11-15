use std::collections::HashSet;

#[test]
fn latest_pin_snapshot_wins_and_failure_preserves_previous_pins() {
    let mut projection = super::SessionUiProjection::<(), ()>::default();
    projection.pins_generation = 2;
    let pins = HashSet::from([("project".into(), "task".into())]);
    projection.apply_pins(2, Ok(pins.clone()));
    projection.apply_pins(1, Ok(HashSet::new()));
    assert_eq!(projection.pinned, pins);
    projection.apply_pins(2, Err("unavailable".into()));
    assert_eq!(projection.pinned, pins);
    projection.apply_pins(2, Ok(HashSet::new()));
    assert!(projection.pinned.is_empty());
}
