use super::*;
use agent_client_protocol::ContentBlock;

#[test]
fn image_only_current_message_does_not_reuse_previous_text() {
    let messages = json!([
        {"role": "user", "content": "old question"},
        {"role": "user", "content": [{"type": "image_url", "image_url": {"url": "data:image/png;base64,YQ=="}}]}
    ]);
    let blocks = build_request_content(&messages, false, AcpReplayStrategy::Discard, 3).unwrap();
    assert_eq!(blocks.len(), 1);
    assert!(
        matches!(&blocks[0], ContentBlock::Image(image) if image.data == "YQ==" && image.mime_type == "image/png")
    );
}

#[test]
fn text_markers_and_multiple_image_parts_are_preserved() {
    let messages = json!([{"role": "user", "content": [
        {"type": "text", "text": "look [IMAGE:data:image/png;base64,YQ==]"},
        {"type": "image", "url": "data:image/jpeg;base64,Yg=="}
    ]}]);
    let blocks = build_request_content(&messages, false, AcpReplayStrategy::Discard, 3).unwrap();
    assert_eq!(blocks.len(), 3);
    assert!(matches!(&blocks[0], ContentBlock::Text(text) if text.text == "look"));
    assert!(matches!(&blocks[2], ContentBlock::Image(image) if image.data == "Yg=="));
}

#[test]
fn replay_strategies_select_historical_images() {
    let messages = json!([
        {"role": "user", "content": [{"type": "image", "url": "data:image/png;base64,YQ=="}]},
        {"role": "assistant", "content": "answer"},
        {"role": "user", "content": "next"}
    ]);
    for (strategy, expected) in [
        (AcpReplayStrategy::Full, 1),
        (AcpReplayStrategy::Discard, 0),
        (AcpReplayStrategy::Recent, 0),
        (AcpReplayStrategy::Summary, 0),
    ] {
        let blocks = build_request_content(&messages, true, strategy, 1).unwrap();
        assert_eq!(blocks.iter().filter(|b| matches!(b, ContentBlock::Image(_))).count(), expected);
    }
}

#[test]
fn invalid_images_fail_explicitly() {
    for url in [
        "https://example.com/a.png",
        "data:text/plain;base64,YQ==",
        "data:image/png;base64,invalid",
    ] {
        let messages = json!([{"role": "user", "content": [{"type": "image", "url": url}]}]);
        assert!(build_request_content(&messages, false, AcpReplayStrategy::Discard, 3).is_err());
    }
}
