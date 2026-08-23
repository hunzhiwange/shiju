//! 验证伪终端类型序列化、缓冲区游标和缺失会话的安全默认值。

use super::*;

#[test]
fn event_definitions_preserve_public_event_names() {
    assert_eq!(event::CREATED.r#type, "pty.created");
    assert_eq!(event::UPDATED.r#type, "pty.updated");
    assert_eq!(event::EXITED.r#type, "pty.exited");
    assert_eq!(event::DELETED.r#type, "pty.deleted");
    assert_eq!(event::DATA.r#type, "pty.data");
}

#[test]
fn status_serializes_and_deserializes_as_lowercase_values() {
    assert_eq!(serde_json::to_value(Status::Running).unwrap(), serde_json::json!("running"));
    assert_eq!(serde_json::to_value(Status::Exited).unwrap(), serde_json::json!("exited"));
    assert_eq!(
        serde_json::from_value::<Status>(serde_json::json!("running")).unwrap(),
        Status::Running
    );
    assert_eq!(
        serde_json::from_value::<Status>(serde_json::json!("exited")).unwrap(),
        Status::Exited
    );
}

#[test]
fn status_rejects_unknown_serialized_values() {
    let error = serde_json::from_value::<Status>(serde_json::json!("paused"))
        .expect_err("unknown status should be rejected");

    assert!(error.to_string().contains("unknown variant"));
}

#[test]
fn create_input_default_omits_all_optional_fields() {
    let value = serde_json::to_value(CreateInput::default()).unwrap();

    assert_eq!(value, serde_json::json!({}));
}

#[test]
fn create_input_round_trip_preserves_explicit_configuration() {
    let input = CreateInput {
        command: Some("shell".to_string()),
        args: Some(vec!["--login".to_string(), "--quiet".to_string()]),
        cwd: Some("/workspace".to_string()),
        title: Some("Workspace shell".to_string()),
        env: Some(HashMap::from([
            ("LANG".to_string(), "C.UTF-8".to_string()),
            ("TERM_PROGRAM".to_string(), "VibeWindow".to_string()),
        ])),
    };

    let decoded: CreateInput =
        serde_json::from_value(serde_json::to_value(input).unwrap()).unwrap();

    assert_eq!(decoded.command.as_deref(), Some("shell"));
    assert_eq!(decoded.args, Some(vec!["--login".to_string(), "--quiet".to_string()]));
    assert_eq!(decoded.cwd.as_deref(), Some("/workspace"));
    assert_eq!(decoded.title.as_deref(), Some("Workspace shell"));
    assert_eq!(
        decoded.env.as_ref().and_then(|env| env.get("LANG")).map(String::as_str),
        Some("C.UTF-8")
    );
    assert_eq!(
        decoded.env.as_ref().and_then(|env| env.get("TERM_PROGRAM")).map(String::as_str),
        Some("VibeWindow")
    );
}

#[test]
fn update_input_omits_absent_fields_and_round_trips_size() {
    let empty: UpdateInput = serde_json::from_value(serde_json::json!({})).unwrap();
    assert!(empty.title.is_none());
    assert!(empty.size.is_none());
    assert_eq!(serde_json::to_value(empty).unwrap(), serde_json::json!({}));

    let value = serde_json::to_value(UpdateInput {
        title: Some("Renamed".to_string()),
        size: Some(Size { rows: 40, cols: 120 }),
    })
    .unwrap();
    assert_eq!(
        value,
        serde_json::json!({ "title": "Renamed", "size": { "rows": 40, "cols": 120 } })
    );

    let decoded: UpdateInput = serde_json::from_value(value).unwrap();
    assert_eq!(decoded.title.as_deref(), Some("Renamed"));
    let size = decoded.size.expect("serialized size should be present");
    assert_eq!((size.rows, size.cols), (40, 120));
}

#[test]
fn info_round_trip_preserves_session_metadata() {
    let info = Info {
        id: "pty_test".to_string(),
        title: "Test terminal".to_string(),
        command: "shell".to_string(),
        args: vec!["--login".to_string()],
        cwd: "/workspace".to_string(),
        status: Status::Exited,
        pid: 42,
    };

    let value = serde_json::to_value(info).unwrap();
    assert_eq!(value["status"], serde_json::json!("exited"));

    let decoded: Info = serde_json::from_value(value).unwrap();
    assert_eq!(decoded.id, "pty_test");
    assert_eq!(decoded.title, "Test terminal");
    assert_eq!(decoded.command, "shell");
    assert_eq!(decoded.args, vec!["--login".to_string()]);
    assert_eq!(decoded.cwd, "/workspace");
    assert_eq!(decoded.status, Status::Exited);
    assert_eq!(decoded.pid, 42);
}

#[test]
fn pty_error_display_and_conversions_preserve_underlying_errors() {
    let invalid = Error::Invalid("bad input".to_string());
    assert_eq!(invalid.to_string(), "bad input");

    let io: Error = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "io denied").into();
    assert_eq!(io.to_string(), "io denied");
    match io {
        Error::Io(error) => assert_eq!(error.kind(), std::io::ErrorKind::PermissionDenied),
        other => panic!("expected io error, got {other:?}"),
    }

    let json_error = serde_json::from_str::<serde_json::Value>("{").unwrap_err();
    let json: Error = json_error.into();
    assert!(!json.to_string().is_empty());
    assert!(matches!(json, Error::Json(_)));
}

#[test]
fn buffer_state_cursor_modes_select_expected_bytes() {
    let mut buffer = BufferState::new();
    assert_eq!(buffer.slice_from(-1), (Vec::new(), 0, 0));

    buffer.push(b"abcdef");

    assert_eq!(buffer.slice_from(-2), (b"abcdef".to_vec(), 0, 6));
    assert_eq!(buffer.slice_from(-1), (Vec::new(), 0, 6));
    assert_eq!(buffer.slice_from(0), (b"abcdef".to_vec(), 0, 6));
    assert_eq!(buffer.slice_from(3), (b"def".to_vec(), 0, 6));
    assert_eq!(buffer.slice_from(6), (Vec::new(), 0, 6));
    assert_eq!(buffer.slice_from(99), (Vec::new(), 0, 6));
}

#[test]
fn buffer_state_keeps_exact_limit_without_advancing_start() {
    let mut buffer = BufferState::new();
    let chunk = vec![b'a'; BUFFER_LIMIT];
    buffer.push(&chunk);

    let (bytes, start, end) = buffer.slice_from(0);
    assert_eq!(bytes.len(), BUFFER_LIMIT);
    assert!(bytes.iter().all(|byte| *byte == b'a'));
    assert_eq!(start, 0);
    assert_eq!(end, BUFFER_LIMIT);
}

#[test]
// 缓冲区截断保留最新字节，游标继续使用绝对位置以支持增量读取。
fn buffer_state_trimming_keeps_newest_bytes_and_absolute_cursors() {
    let mut buffer = BufferState::new();
    let chunk = vec![b'a'; BUFFER_LIMIT];
    buffer.push(&chunk);
    buffer.push(b"tail");

    let (retained, start, end) = buffer.slice_from(0);
    assert_eq!(retained.len(), BUFFER_LIMIT);
    assert_eq!(start, 4);
    assert_eq!(end, BUFFER_LIMIT + 4);
    assert_eq!(&retained[retained.len() - 4..], b"tail");
    assert_eq!(buffer.slice_from(start as i64), (retained, start, end));
    assert_eq!(buffer.slice_from((end - 4) as i64), (b"tail".to_vec(), start, end));
}

#[test]
fn buffer_state_single_oversized_chunk_keeps_only_its_newest_bytes() {
    let mut buffer = BufferState::new();
    let mut chunk = vec![b'a'; BUFFER_LIMIT + 3];
    chunk[BUFFER_LIMIT..].copy_from_slice(b"end");

    buffer.push(&chunk);

    let (retained, start, end) = buffer.slice_from(0);
    assert_eq!(retained.len(), BUFFER_LIMIT);
    assert_eq!(start, 3);
    assert_eq!(end, BUFFER_LIMIT + 3);
    assert_eq!(&retained[retained.len() - 3..], b"end");
    assert_eq!(buffer.slice_from(2), (retained.clone(), start, end));
    assert_eq!(buffer.slice_from(3), (retained, start, end));
}

#[test]
fn buffer_state_empty_push_preserves_existing_cursor_and_bytes() {
    let mut buffer = BufferState::new();
    buffer.push(b"data");

    buffer.push(b"");

    assert_eq!(buffer.slice_from(0), (b"data".to_vec(), 0, 4));
}

#[test]
fn buffer_state_cursor_saturates_instead_of_wrapping() {
    let mut buffer = BufferState { bytes: Vec::new(), buffer_cursor: 0, cursor: usize::MAX - 1 };

    buffer.push(b"abc");

    assert_eq!(buffer.cursor, usize::MAX);
    assert_eq!(buffer.slice_from(0), (b"abc".to_vec(), 0, usize::MAX));
}

#[cfg(not(target_arch = "wasm32"))]
#[tokio::test]
// 未知会话的查询和清理返回安全默认值，不应触发外部进程操作。
async fn missing_session_operations_return_safe_defaults_without_external_effects() {
    let missing_id = "pty_static_test_missing_session";

    assert!(list().await.iter().all(|info| info.id != missing_id));
    assert!(get(missing_id).await.is_none());
    assert!(
        update(missing_id, UpdateInput { title: Some("ignored".to_string()), size: None })
            .await
            .expect("missing session update should be accepted")
            .is_none()
    );
    assert!(read(missing_id, 0).await.is_none());

    write(missing_id, "ignored").await;
    resize(missing_id, 120, 40).await;
    assert!(remove(missing_id).await.expect("missing session removal should be idempotent"));
}
