use super::*;
#[test]
fn duplicate_terminal_is_explicit() {
    let mut state = TurnLifecycleState::default();
    assert_eq!(state.start("turn-1"), TurnStartOutcome::Started);
    assert_eq!(state.finish("turn-1"), TurnFinishOutcome::Finished);
    assert_eq!(state.finish("turn-1"), TurnFinishOutcome::DuplicateTerminal);
}

#[test]
fn elapsed_seconds_tracks_only_the_active_turn() {
    let now = Instant::now();
    let mut state = TurnLifecycleState {
        active_started_at: Some(now - Duration::from_secs(7)),
        ..TurnLifecycleState::default()
    };

    assert_eq!(state.elapsed_at(now), Duration::from_secs(7));

    state.active_started_at = None;
    assert_eq!(state.elapsed_at(now), Duration::ZERO);
}
