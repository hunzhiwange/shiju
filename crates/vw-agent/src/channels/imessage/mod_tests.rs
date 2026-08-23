use super::*;
use crate::app::agent::channels::traits::{Channel, SendMessage};
use rusqlite::Connection;
use tokio::sync::mpsc;

#[test]
fn build_send_script_escapes_content() {
    let message = SendMessage::new("hello \"there\"\nnext", "user@example.com");
    let script = build_send_script(&message).unwrap();

    assert!(script.contains(r#"participant "user@example.com""#));
    assert!(script.contains(r#"send "hello \"there\"\nnext""#));
}

#[test]
fn build_send_script_rejects_invalid_target() {
    let message = SendMessage::new("hello", "not-a-target");
    let error = build_send_script(&message).unwrap_err().to_string();

    assert!(error.contains("Invalid iMessage target"));
}

#[tokio::test]
async fn send_rejects_invalid_target_before_process_spawn() {
    let channel = IMessageChannel::new(vec!["*".to_string()]);
    let message = SendMessage::new("hello", "not-a-target");
    let error = channel.send(&message).await.unwrap_err().to_string();

    assert!(error.contains("Invalid iMessage target"));
}

#[tokio::test]
async fn forward_imessage_messages_filters_and_updates_rowid() {
    let channel = IMessageChannel::new(vec!["allowed@example.com".to_string()]);
    let (tx, mut rx) = mpsc::channel(4);
    let mut last_rowid = 5;

    let receiver_open = forward_imessage_messages(
        &channel,
        &tx,
        &mut last_rowid,
        vec![
            (6, "blocked@example.com".to_string(), "blocked".to_string()),
            (7, "allowed@example.com".to_string(), "   ".to_string()),
            (8, "allowed@example.com".to_string(), "hello".to_string()),
        ],
    )
    .await;

    assert!(receiver_open);
    assert_eq!(last_rowid, 8);

    let msg = rx.recv().await.unwrap();
    assert_eq!(msg.id, "8");
    assert_eq!(msg.sender, "allowed@example.com");
    assert_eq!(msg.reply_target, "allowed@example.com");
    assert_eq!(msg.content, "hello");
    assert_eq!(msg.channel, "imessage");
    assert!(msg.thread_ts.is_none());
    assert!(matches!(rx.try_recv(), Err(mpsc::error::TryRecvError::Empty)));
}

#[tokio::test]
async fn forward_imessage_messages_reports_closed_receiver() {
    let channel = IMessageChannel::new(vec!["*".to_string()]);
    let (tx, rx) = mpsc::channel(1);
    let mut last_rowid = 0;
    drop(rx);

    let receiver_open = forward_imessage_messages(
        &channel,
        &tx,
        &mut last_rowid,
        vec![(1, "sender@example.com".to_string(), "hello".to_string())],
    )
    .await;

    assert!(!receiver_open);
    assert_eq!(last_rowid, 1);
}

#[tokio::test]
async fn forward_imessage_messages_preserves_high_water_mark_for_older_row() {
    let channel = IMessageChannel::new(vec!["*".to_string()]);
    let (tx, mut rx) = mpsc::channel(1);
    let mut last_rowid = 10;

    let receiver_open = forward_imessage_messages(
        &channel,
        &tx,
        &mut last_rowid,
        vec![(9, "sender@example.com".to_string(), "older".to_string())],
    )
    .await;

    assert!(receiver_open);
    assert_eq!(last_rowid, 10);
    assert_eq!(rx.recv().await.unwrap().content, "older");
}

#[test]
fn ensure_imessage_database_exists_reports_missing_path() {
    let dir = tempfile::tempdir().expect("temp dir");
    let missing = dir.path().join("Library/Messages/chat.db");

    let error = ensure_imessage_database_exists(&missing).unwrap_err().to_string();

    assert!(error.contains("Messages database not found"));
    assert!(error.contains("Full Disk Access"));
}

#[tokio::test]
async fn imessage_connection_helpers_load_initial_rowid_and_poll_new_messages() {
    let (_dir, db_path) = create_chat_db();
    {
        let conn = Connection::open(&db_path).expect("open fixture");
        conn.execute("INSERT INTO handle (ROWID, id) VALUES (1, 'allowed@example.com')", [])
            .expect("insert handle");
        conn.execute(
            "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (10, 1, 'old', 0)",
            [],
        )
        .expect("insert old");
        conn.execute(
            "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (20, 1, 'new', 0)",
            [],
        )
        .expect("insert new");
        conn.execute(
            "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (30, 1, 'sent', 1)",
            [],
        )
        .expect("insert sent");
        conn.execute(
            "INSERT INTO message (ROWID, handle_id, text, is_from_me) VALUES (40, 1, NULL, 0)",
            [],
        )
        .expect("insert null");
    }

    ensure_imessage_database_exists(&db_path).expect("fixture exists");
    let conn = open_imessage_connection(&db_path).await.expect("open readonly fixture");
    let (conn, initial_rowid) = load_initial_rowid(conn).await.expect("initial rowid");
    assert_eq!(initial_rowid, 40);

    let (conn, poll_result) = poll_imessage_connection(conn, 10).await.expect("poll join");
    assert_eq!(
        poll_result.expect("poll rows"),
        vec![(20, "allowed@example.com".to_string(), "new".to_string())]
    );
    let (_conn, empty_result) = poll_imessage_connection(conn, 40).await.expect("empty poll join");
    assert!(empty_result.expect("empty rows").is_empty());
}

#[tokio::test]
async fn imessage_poll_connection_returns_query_errors_without_closing_connection() {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("chat.db");
    Connection::open(&db_path).expect("create empty sqlite fixture");
    let conn = open_imessage_connection(&db_path).await.expect("open empty fixture");

    let (_conn, poll_result) = poll_imessage_connection(conn, 0).await.expect("poll join");

    assert!(poll_result.unwrap_err().to_string().contains("no such table"));
}

#[tokio::test]
async fn imessage_health_check_reports_platform_availability() {
    let channel = IMessageChannel::new(vec!["*".to_string()]);

    let healthy = channel.health_check().await;

    if !cfg!(target_os = "macos") {
        assert!(!healthy);
    }
}

fn create_chat_db() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let db_path = dir.path().join("chat.db");
    let conn = Connection::open(&db_path).expect("create sqlite fixture");
    conn.execute_batch(
        "CREATE TABLE handle (
            ROWID INTEGER PRIMARY KEY,
            id TEXT NOT NULL
        );
        CREATE TABLE message (
            ROWID INTEGER PRIMARY KEY,
            handle_id INTEGER,
            text TEXT,
            is_from_me INTEGER DEFAULT 0,
            FOREIGN KEY (handle_id) REFERENCES handle(ROWID)
        );",
    )
    .expect("create messages schema");
    (dir, db_path)
}
// 本文件验证 iMessage 脚本转义、数据库轮询与平台健康检查。
