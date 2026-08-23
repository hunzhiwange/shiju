//! 补丁数据类型的线格式与错误展示测试。
//!
//! 验证小写变体标签、更新块顺序、受影响路径集合及底层错误传播。

use serde_json::json;

#[test]
fn hunk_add_serializes_with_lowercase_tag() {
    let hunk = super::Hunk::Add {
        path: "src/lib.rs".to_string(),
        contents: "pub fn run() {}\n".to_string(),
    };

    let value = serde_json::to_value(hunk).unwrap();

    assert_eq!(
        value,
        json!({
            "type": "add",
            "path": "src/lib.rs",
            "contents": "pub fn run() {}\n"
        })
    );
}

#[test]
fn hunk_delete_deserializes_from_lowercase_tag() {
    let hunk: super::Hunk =
        serde_json::from_value(json!({ "type": "delete", "path": "old.txt" })).unwrap();

    match hunk {
        super::Hunk::Delete { path } => assert_eq!(path, "old.txt"),
        _ => panic!("expected delete hunk"),
    }
}

#[test]
fn hunk_update_round_trips_chunks_and_move_path() {
    let hunk = super::Hunk::Update {
        path: "src/main.rs".to_string(),
        move_path: Some("src/bin/main.rs".to_string()),
        chunks: vec![super::UpdateFileChunk {
            old_lines: vec!["fn main() {}".to_string()],
            new_lines: vec!["fn main() { println!(\"hi\"); }".to_string()],
            change_context: Some("fn main".to_string()),
            is_end_of_file: Some(true),
        }],
    };

    let value = serde_json::to_value(&hunk).unwrap();
    let parsed: super::Hunk = serde_json::from_value(value).unwrap();

    match parsed {
        super::Hunk::Update { path, move_path, chunks } => {
            assert_eq!(path, "src/main.rs");
            assert_eq!(move_path.as_deref(), Some("src/bin/main.rs"));
            assert_eq!(chunks.len(), 1);
            assert_eq!(chunks[0].old_lines, vec!["fn main() {}"]);
            assert_eq!(chunks[0].new_lines, vec!["fn main() { println!(\"hi\"); }"]);
            assert_eq!(chunks[0].change_context.as_deref(), Some("fn main"));
            assert_eq!(chunks[0].is_end_of_file, Some(true));
        }
        _ => panic!("expected update hunk"),
    }
}

#[test]
fn parse_result_preserves_hunk_order() {
    let result = super::ParseResult {
        hunks: vec![
            super::Hunk::Delete { path: "a.txt".to_string() },
            super::Hunk::Add { path: "b.txt".to_string(), contents: "new".to_string() },
        ],
    };

    let value = serde_json::to_value(result).unwrap();

    assert_eq!(value["hunks"][0]["type"], "delete");
    assert_eq!(value["hunks"][1]["type"], "add");
}

#[test]
fn affected_paths_serializes_all_path_sets() {
    let paths = super::AffectedPaths {
        added: vec!["new.txt".to_string()],
        modified: vec!["changed.txt".to_string()],
        deleted: vec!["old.txt".to_string()],
    };

    let value = serde_json::to_value(paths).unwrap();

    assert_eq!(
        value,
        json!({
            "added": ["new.txt"],
            "modified": ["changed.txt"],
            "deleted": ["old.txt"]
        })
    );
}

#[test]
fn apply_patch_file_update_serializes_diff_and_content() {
    let update = super::ApplyPatchFileUpdate {
        unified_diff: "--- a/file\n+++ b/file\n".to_string(),
        content: "updated".to_string(),
    };

    let value = serde_json::to_value(update).unwrap();

    assert_eq!(
        value,
        json!({
            "unified_diff": "--- a/file\n+++ b/file\n",
            "content": "updated"
        })
    );
}

#[test]
fn error_display_uses_inner_messages() {
    let parse = super::Error::Parse("bad hunk".to_string());
    let compute = super::Error::ComputeReplacements("missing context".to_string());

    assert_eq!(parse.to_string(), "bad hunk");
    assert_eq!(compute.to_string(), "missing context");
}

#[test]
fn io_error_converts_and_displays_inner_error() {
    let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "patch target missing");
    let error = super::Error::from(io_error);

    assert_eq!(error.to_string(), "patch target missing");

    match error {
        super::Error::Io(err) => assert_eq!(err.to_string(), "patch target missing"),
        _ => panic!("expected io error"),
    }
}
