fn long_chat_chunk_snapshot() -> super::ChatUiChunkSnapshot {
    let chat: desktop_session::session::loading::SharedChatMessages = std::sync::Arc::from(
        (0..10_000)
            .map(|idx| session_ui_types::ChatMessage {
                role: session_ui_types::ChatRole::Assistant,
                content: format!("message-{idx}"),
                model: None,
                think_timing: Vec::new(),
                processing_timing: None,
            })
            .collect::<Vec<_>>(),
    );

    super::chat_ui_chunk_snapshot(&chat, 9_952)
        .expect("the requested history chunk should have a bounded snapshot")
}

#[test]
fn chat_ui_chunk_snapshots_requested_32_only() {
    let snapshot = long_chat_chunk_snapshot();

    assert_eq!(snapshot.chunk_start_idx, 9_952);
    assert_eq!(snapshot.messages.len(), crate::chat_ui::CHAT_UI_CHUNK_SIZE);
    assert_eq!(snapshot.messages[0].content, "message-9952");
    assert_eq!(snapshot.messages[31].content, "message-9983");
}

#[test]
fn prepare_session_ui_does_not_clone_full_history() {
    let snapshot = long_chat_chunk_snapshot();

    assert_eq!(snapshot.messages.len(), 32);
    assert!(snapshot.messages.len() < 10_000);
}
