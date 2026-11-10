use super::lifecycle_state::{Lifecycle, LifecycleState, TransitionInProgress};

#[test]
fn cc_route_004_lifecycle_state_expected_behavior() {
    let mut lifecycle = Lifecycle::default();
    assert_eq!(lifecycle.state(), LifecycleState::Stopped);
    assert_eq!(lifecycle.begin_stop(), Ok(None));

    assert_eq!(lifecycle.begin_start(), Ok(None));
    assert_eq!(lifecycle.state(), LifecycleState::Starting);
    assert_eq!(lifecycle.begin_start(), Err(TransitionInProgress));
    assert_eq!(lifecycle.begin_stop(), Err(TransitionInProgress));

    lifecycle.finish_start(Some(1));
    assert_eq!(lifecycle.state(), LifecycleState::Running);
    assert_eq!(lifecycle.begin_start(), Ok(Some(&1)));
    assert_eq!(lifecycle.begin_stop(), Ok(Some(1)));
    assert_eq!(lifecycle.state(), LifecycleState::Stopping);
    assert_eq!(lifecycle.begin_start(), Err(TransitionInProgress));
    assert_eq!(lifecycle.begin_stop(), Err(TransitionInProgress));

    lifecycle.listener_exited();
    assert_eq!(lifecycle.state(), LifecycleState::Stopped);
    assert_eq!(lifecycle.begin_stop(), Ok(None));
}

#[test]
fn cc_route_004_lifecycle_recovers_from_failed_start_and_unexpected_exit() {
    let mut lifecycle = Lifecycle::default();
    assert_eq!(lifecycle.begin_start(), Ok(None));
    lifecycle.finish_start(None);
    assert_eq!(lifecycle.state(), LifecycleState::Failed);

    assert_eq!(lifecycle.begin_start(), Ok(None));
    lifecycle.finish_start(Some(1));
    lifecycle.listener_exited();
    assert_eq!(lifecycle.state(), LifecycleState::Failed);

    assert_eq!(lifecycle.begin_stop(), Ok(None));
    assert_eq!(lifecycle.state(), LifecycleState::Stopped);
}
