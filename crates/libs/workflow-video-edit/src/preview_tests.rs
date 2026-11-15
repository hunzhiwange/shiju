use super::{apply::VideoSourceWindow, preview::*};
use std::collections::BTreeSet;
fn evidence() -> (VideoTimelineEvidence, VideoPreviewEvidence) {
    (
        VideoTimelineEvidence {
            project_id: "p".into(),
            revision: "r2".into(),
            source_asset_id: "source".into(),
            windows: vec![VideoSourceWindow {
                source_start: 3.0,
                source_end: 8.0,
                timeline_start: 0.0,
                timeline_end: 5.0,
            }],
            captions_present: true,
            effect_ids: BTreeSet::new(),
            video_readable: true,
            original_audio_present: true,
            duration_sec: 5.0,
        },
        VideoPreviewEvidence {
            project_id: "p".into(),
            revision: "r2".into(),
            job_id: "job".into(),
            job_status: "succeeded".into(),
            media_playable: true,
            duration_sec: 5.0,
        },
    )
}
#[test]
fn video_edit_delivery_rejects_fake_ready_empty_timeline_and_unplayable_media() {
    let (t, p) = evidence();
    assert!(validate_preview(&t, &p).is_ok());
    for mode in 0..7 {
        let (mut t, mut p) = evidence();
        match mode {
            0 => t.windows.clear(),
            1 => t.video_readable = false,
            2 => t.captions_present = false,
            3 => p.media_playable = false,
            4 => p.job_status = "running".into(),
            5 => p.revision = "old".into(),
            _ => t.original_audio_present = false,
        }
        assert!(validate_preview(&t, &p).is_err(), "accepted invalid mode {mode}");
    }
}
#[test]
fn video_edit_delivery_rejects_duration_not_matching_actual_timeline() {
    let (mut t, mut p) = evidence();
    t.duration_sec = 8.0;
    p.duration_sec = 8.0;
    assert!(validate_preview(&t, &p).is_err(), "duration must come from actual windows");
}
#[test]
fn video_edit_delivery_rejects_empty_or_nonviewer_page() {
    for url in [
        "",
        "https://example.test/s/",
        "http://example.test/s/token",
        "https://user:password@example.test/s/token",
        "https://example.test/download.mp4",
    ] {
        assert!(validate_viewer_url(url).is_err());
    }
    assert!(validate_viewer_url("https://example.test/s/token").is_ok());
}
