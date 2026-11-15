use super::*;
use iced::time::Duration;

#[test]
fn monthly_quota_ring_animates_to_the_new_balance_and_stops() {
    let mut state = RingState::default();
    let now = Instant::now();
    assert!(state.update(Some(0.8), now));
    assert!(state.update(Some(0.8), now + Duration::from_millis(150)));
    assert!(state.progress > 0.0 && state.progress < 0.8);
    let finished = now + Duration::from_millis(400);
    assert!(!state.update(Some(0.8), finished));
    assert_eq!(state.progress, 0.8);

    assert!(state.update(Some(0.1), finished));
    assert!(state.update(Some(0.1), finished + Duration::from_millis(150)));
    assert!(state.progress > 0.1 && state.progress < 0.8);
    assert!(!state.update(None, finished));
    assert_eq!(state.progress, 0.0);
}

#[test]
fn monthly_quota_ring_labels_and_colors_distinguish_balance_states() {
    for (ring, label) in [
        (QuotaRing::Limited(0.0), "0%"),
        (QuotaRing::Limited(0.000059), "<1%"),
        (QuotaRing::Limited(0.29), "29%"),
        (QuotaRing::Limited(0.9999), "99%"),
        (QuotaRing::Limited(1.0), "100%"),
        (QuotaRing::Unlimited, "∞"),
        (QuotaRing::Unknown, "—"),
    ] {
        assert_eq!(ring.label(), label);
    }

    for (fraction, expected) in [
        (0.0, [255, 0, 0, 255]),
        (0.000059, [255, 0, 0, 255]),
        (0.01, [255, 0, 0, 255]),
        (0.5, [255, 255, 0, 255]),
        (1.0, [0, 255, 0, 255]),
    ] {
        assert_eq!(remaining_color(fraction).into_rgba8(), expected);
    }
}
