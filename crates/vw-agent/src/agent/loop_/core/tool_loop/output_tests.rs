use super::*;

fn drain_available(rx: &mut tokio::sync::mpsc::Receiver<String>) -> Vec<String> {
    let mut messages = Vec::new();
    while let Ok(message) = rx.try_recv() {
        messages.push(message);
    }
    messages
}

#[tokio::test]
async fn progress_helpers_emit_expected_messages() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);

    update_thinking_progress(Some(&tx), 0).await;
    update_tool_call_progress(Some(&tx), 2, 7).await;
    send_retry_progress(Some(&tx)).await;

    assert!(rx.recv().await.unwrap().contains(DRAFT_PROGRESS_SENTINEL));
    assert!(rx.recv().await.unwrap().contains("2 tool call"));
    assert!(rx.recv().await.unwrap().contains("Retrying"));
}

#[tokio::test]
async fn thinking_progress_marks_later_iterations() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    update_thinking_progress(Some(&tx), 2).await;

    assert_eq!(rx.recv().await.unwrap(), format!("{DRAFT_PROGRESS_SENTINEL}💡 思考中 (第3轮)\n"));
}

#[tokio::test]
async fn progress_helpers_skip_when_delta_is_absent_or_empty() {
    update_thinking_progress(None, 0).await;
    update_tool_call_progress(None, 1, 3).await;
    send_retry_progress(None).await;

    let (tx, mut rx) = tokio::sync::mpsc::channel(1);
    update_tool_call_progress(Some(&tx), 0, 3).await;

    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn progress_helpers_ignore_closed_receivers() {
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    update_thinking_progress(Some(&tx), 0).await;
    update_tool_call_progress(Some(&tx), 1, 0).await;
    send_retry_progress(Some(&tx)).await;
}

#[tokio::test]
async fn final_response_starts_by_clearing_draft() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);

    stream_final_response("hello world", Some(&tx), None).await.unwrap();

    assert_eq!(rx.recv().await.unwrap(), DRAFT_CLEAR_SENTINEL);
    let mut final_text = String::new();
    while let Ok(chunk) = rx.try_recv() {
        final_text.push_str(&chunk);
    }
    assert_eq!(final_text, "hello world");
}

#[tokio::test]
async fn final_response_splits_on_whitespace_boundaries() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(8);

    stream_final_response("one two three four", Some(&tx), None).await.unwrap();

    assert_eq!(rx.recv().await.unwrap(), DRAFT_CLEAR_SENTINEL);
    let chunks = drain_available(&mut rx);

    assert_eq!(chunks, vec!["one two ".to_string(), "three ".to_string(), "four".to_string()]);
}

#[tokio::test]
async fn final_response_skips_when_delta_is_absent() {
    stream_final_response("testcov-0089", None, None).await.unwrap();
}

#[tokio::test]
async fn final_response_sends_only_clear_for_empty_text() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    stream_final_response("", Some(&tx), None).await.unwrap();

    assert_eq!(rx.recv().await.unwrap(), DRAFT_CLEAR_SENTINEL);
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn final_response_returns_cancelled_when_token_is_cancelled() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(4);
    let token = CancellationToken::new();
    token.cancel();

    let err = stream_final_response("testcov-0089", Some(&tx), Some(&token)).await.unwrap_err();

    assert!(err.is::<ToolLoopCancelled>());
    assert_eq!(rx.recv().await.unwrap(), DRAFT_CLEAR_SENTINEL);
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn final_response_stops_cleanly_when_receiver_is_closed() {
    let (tx, rx) = tokio::sync::mpsc::channel(1);
    drop(rx);

    stream_final_response("testcov-0089 closed channel", Some(&tx), None).await.unwrap();
}

#[tokio::test]
async fn final_response_stops_when_receiver_closes_after_clear() {
    let (tx, mut rx) = tokio::sync::mpsc::channel(1);

    let task = tokio::spawn(async move {
        stream_final_response("testcov-0089 after clear", Some(&tx), None).await.unwrap();
    });

    assert_eq!(rx.recv().await.unwrap(), DRAFT_CLEAR_SENTINEL);
    drop(rx);

    task.await.unwrap();
}
