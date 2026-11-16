use serde_json::json;
#[test]
fn video_edit_delivery_normalizes_live_media_probe_projection_without_losing_audio() {
    let probe=crate::cloud::probe::normalize_media_probe(json!({"durationSec":2.262,"video":{"width":640,"height":360,"fps":10},"streams":[{"type":"video","codec":"h264"},{"type":"audio","codec":"aac"}]})).unwrap();
    assert_eq!(probe["streams"][0]["codec_type"], "video");
    assert_eq!(probe["streams"][0]["width"], 640);
    assert_eq!(probe["streams"][0]["avg_frame_rate"], 10);
    assert_eq!(probe["streams"][1]["codec_type"], "audio");
}
