//! Pure artifact payload behavior owned by session-engine.

use super::session_messages_payload;
use crate::session_memory::Session;

#[test]
fn session_messages_payload_returns_empty_array_for_empty_session() {
    let session = Session { id: "artifacts-test-empty".to_string(), messages: Vec::new() };

    assert!(session_messages_payload(&session).is_empty());
}
