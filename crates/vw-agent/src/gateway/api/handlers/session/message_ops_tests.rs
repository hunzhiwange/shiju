//! Concrete host handler signatures remain registered here.
use super::*;

#[test]
fn message_handlers_are_available() {
    let _ = session_message_list;
    let _ = session_message_get;
    let _ = session_message_part_delete;
    let _ = session_message_part_patch;
}
