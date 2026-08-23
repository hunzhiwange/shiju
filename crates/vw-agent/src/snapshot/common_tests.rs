//! 验证跨平台快照模型的差异统计、序列化与路径处理。

use super::*;
#[cfg(not(target_arch = "wasm32"))]
use serde_json::Value;
use serde_json::json;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
#[cfg(windows)]
use std::os::windows::process::ExitStatusExt;

#[cfg(unix)]
fn exit_status(code: i32) -> std::process::ExitStatus {
    std::process::ExitStatus::from_raw(code << 8)
}

#[cfg(windows)]
fn exit_status(code: i32) -> std::process::ExitStatus {
    std::process::ExitStatus::from_raw(code as u32)
}

#[test]
fn patch_serializes_hash_and_files() {
    let patch = Patch {
        hash: "abc123".to_string(),
        files: vec!["/tmp/file.rs".to_string(), "/tmp/other.rs".to_string()],
    };

    assert_eq!(
        serde_json::to_value(&patch).unwrap(),
        json!({
            "hash": "abc123",
            "files": ["/tmp/file.rs", "/tmp/other.rs"],
        })
    );
}

#[test]
fn patch_deserializes_empty_file_list() {
    let patch: Patch = serde_json::from_value(json!({
        "hash": "tree-hash",
        "files": [],
    }))
    .unwrap();

    assert_eq!(patch.hash, "tree-hash");
    assert!(patch.files.is_empty());
}

#[test]
fn io_error_conversion_preserves_kind_and_message() {
    let error =
        Error::from(std::io::Error::new(std::io::ErrorKind::PermissionDenied, "snapshot denied"));

    match error {
        Error::Io(inner) => {
            assert_eq!(inner.kind(), std::io::ErrorKind::PermissionDenied);
            assert_eq!(inner.to_string(), "snapshot denied");
        }
        other => panic!("expected I/O error, got {other:?}"),
    }
}

#[test]
fn utf8_error_conversion_preserves_invalid_bytes() {
    let bytes = vec![b'f', 0x80];
    let source = String::from_utf8(bytes.clone()).unwrap_err();
    let error = Error::from(source);

    match error {
        Error::Utf8(inner) => {
            assert_eq!(inner.as_bytes(), bytes.as_slice());
            assert!(inner.to_string().contains("invalid utf-8"));
        }
        other => panic!("expected UTF-8 error, got {other:?}"),
    }
}

#[test]
fn git_error_displays_message() {
    let error = Error::Git("git failed".to_string());

    assert_eq!(error.to_string(), "git failed");
}

#[test]
fn source_errors_display_inner_messages() {
    let io = Error::Io(std::io::Error::new(std::io::ErrorKind::Other, "snapshot I/O failed"));
    let utf8_source = String::from_utf8(vec![0xff]).unwrap_err();
    let utf8_message = utf8_source.to_string();
    let utf8 = Error::Utf8(utf8_source);

    assert_eq!(io.to_string(), "snapshot I/O failed");
    assert_eq!(utf8.to_string(), utf8_message);
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn extra_accepts_empty_input() {
    assert!(extra::<0>([]).is_empty());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn extra_uses_last_value_for_duplicate_keys() {
    let fields = extra([
        ("operation", Value::String("track".to_string())),
        ("operation", Value::String("restore".to_string())),
        ("enabled", Value::Bool(true)),
    ]);

    assert_eq!(fields.len(), 2);
    assert_eq!(fields.get("operation"), Some(&json!("restore")));
    assert_eq!(fields.get("enabled"), Some(&json!(true)));
}

#[cfg(any(unix, windows))]
#[test]
fn extra_from_output_records_nonzero_exit_and_streams() {
    let output = std::process::Output {
        status: exit_status(7),
        stdout: b"snapshot stdout".to_vec(),
        stderr: b"snapshot stderr".to_vec(),
    };

    let fields = extra_from_output(&output, [("operation", json!("diff"))]);

    assert_eq!(fields.get("operation"), Some(&json!("diff")));
    assert_eq!(fields.get("exit_code"), Some(&json!(7)));
    assert_eq!(fields.get("stdout"), Some(&json!("snapshot stdout")));
    assert_eq!(fields.get("stderr"), Some(&json!("snapshot stderr")));
}

#[cfg(any(unix, windows))]
#[test]
fn extra_from_output_replaces_reserved_keys_and_decodes_lossily() {
    let output = std::process::Output {
        status: exit_status(0),
        stdout: vec![b'o', 0xff],
        stderr: vec![0xfe],
    };
    let fields = extra_from_output(
        &output,
        [
            ("exit_code", json!(99)),
            ("stdout", json!("stale stdout")),
            ("stderr", json!("stale stderr")),
        ],
    );

    assert_eq!(fields.get("exit_code"), Some(&json!(0)));
    assert_eq!(fields.get("stdout"), Some(&json!("o�")));
    assert_eq!(fields.get("stderr"), Some(&json!("�")));
}

#[cfg(unix)]
#[test]
fn extra_from_output_uses_negative_one_when_exit_code_is_unavailable() {
    let output = std::process::Output {
        status: std::process::ExitStatus::from_raw(9),
        stdout: Vec::new(),
        stderr: Vec::new(),
    };

    let fields = extra_from_output(&output, []);

    assert_eq!(fields.get("exit_code"), Some(&json!(-1)));
}
