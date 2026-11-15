use super::startup_retry_interval;
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn startup_retry_interval_slows_after_each_elapsed_threshold() {
    assert_eq!(startup_retry_interval(Duration::ZERO), Duration::from_secs(2));
    assert_eq!(startup_retry_interval(Duration::from_secs(299)), Duration::from_secs(2));
    assert_eq!(startup_retry_interval(Duration::from_secs(300)), Duration::from_secs(10));
    assert_eq!(startup_retry_interval(Duration::from_secs(3_599)), Duration::from_secs(10));
    assert_eq!(startup_retry_interval(Duration::from_secs(3_600)), Duration::from_secs(20));
    assert_eq!(startup_retry_interval(Duration::from_secs(86_399)), Duration::from_secs(20));
    assert_eq!(startup_retry_interval(Duration::from_secs(86_400)), Duration::from_secs(60));
}