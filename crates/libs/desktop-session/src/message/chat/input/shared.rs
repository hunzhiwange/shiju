use session_ui_types as models;
pub fn preferred_chat_message_index_by_id(
    chat: &[models::ChatMessage],
    message_ids: &[Option<String>],
    message_id: &str,
) -> Option<usize> {
    let mut first_match = None;

    for (idx, candidate) in message_ids.iter().enumerate() {
        if candidate.as_deref() != Some(message_id) {
            continue;
        }

        if first_match.is_none() {
            first_match = Some(idx);
        }

        if chat.get(idx).is_some_and(|message| message.role == models::ChatRole::Tool) {
            return Some(idx);
        }
    }

    first_match
}
