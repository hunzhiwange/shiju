use super::{LeakDetector, LeakResult};

#[test]
fn leak_detector_reexport_preserves_identity_and_redaction() {
    let detector: content_guard::leak_detector::LeakDetector = LeakDetector::new();
    let fixture = "ghp_1234567890abcdefghijklmnopqrstuvwxyz123456";
    let legacy_result: LeakResult = detector.scan(fixture);
    let result: content_guard::leak_detector::LeakResult = legacy_result;

    match result {
        content_guard::leak_detector::LeakResult::Detected { patterns, redacted } => {
            assert!(patterns.iter().any(|pattern| pattern == "GitHub token"));
            assert!(!redacted.contains(fixture));
            assert!(redacted.contains("[REDACTED_API_KEY]"));
        }
        content_guard::leak_detector::LeakResult::Clean => panic!("expected fixture detection"),
    }
}
