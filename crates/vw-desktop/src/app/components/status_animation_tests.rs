use super::status_animation::{
    ACTIVITY_ANIMATION_INTERVAL_MS, STATUS_SPINNER_FRAMES, spinner_frame,
};

#[test]
fn activity_animation_interval_limits_request_time_refresh_rate() {
    assert_eq!(ACTIVITY_ANIMATION_INTERVAL_MS, 90);
}

#[test]
fn spinner_frame_wraps_by_frame_count() {
    assert_eq!(spinner_frame(0), STATUS_SPINNER_FRAMES[0]);
    assert_eq!(spinner_frame(STATUS_SPINNER_FRAMES.len()), STATUS_SPINNER_FRAMES[0]);
}
