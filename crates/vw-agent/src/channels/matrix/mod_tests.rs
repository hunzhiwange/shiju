use super::*;
use std::{path::PathBuf, sync::Arc};

#[test]
fn debug_output_redacts_access_token_and_session_hints() {
    let channel = MatrixChannel::new_with_session_hint_and_vibewindow_dir(
        "https://matrix.example.com/".to_string(),
        "  syt_secret_token  ".to_string(),
        " !room:matrix.example.com ".to_string(),
        vec![" @user:matrix.example.com ".to_string()],
        Some(" @bot:matrix.example.com ".to_string()),
        Some(" DEVICEID ".to_string()),
        Some(PathBuf::from("/tmp/vibewindow")),
    );

    let debug = format!("{channel:?}");

    assert!(debug.contains("MatrixChannel"));
    assert!(debug.contains("https://matrix.example.com"));
    assert!(debug.contains("!room:matrix.example.com"));
    assert!(!debug.contains("syt_secret_token"));
    assert!(!debug.contains("DEVICEID"));
    assert!(!debug.contains("@bot:matrix.example.com"));
}

#[test]
fn clone_shares_runtime_caches_and_conflict_state() {
    let channel = MatrixChannel::new(
        "https://matrix.example.com".to_string(),
        "token".to_string(),
        "!room:matrix.example.com".to_string(),
        vec!["*".to_string()],
    );
    let cloned = channel.clone();

    channel.otk_conflict_detected.store(true, std::sync::atomic::Ordering::SeqCst);

    assert!(cloned.otk_conflict_detected.load(std::sync::atomic::Ordering::SeqCst));
    assert!(Arc::ptr_eq(&channel.resolved_room_id_cache, &cloned.resolved_room_id_cache));
    assert!(Arc::ptr_eq(&channel.sdk_client, &cloned.sdk_client));
}
