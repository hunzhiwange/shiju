use serde_json::json;

use super::*;

#[test]
fn truncate_tool_args_uses_tool_specific_fields() {
    assert_eq!(
        truncate_tool_args_for_progress("shell", &json!({"command": "echo hello"}), 30),
        "echo hello"
    );
    assert_eq!(
        truncate_tool_args_for_progress("file_edit", &json!({"filePath": "src/main.rs"}), 30),
        "src/main.rs"
    );
}

#[test]
fn sentinels_and_interval_are_stable() {
    assert_eq!(PROGRESS_MIN_INTERVAL_MS, 500);
    assert_eq!(DRAFT_CLEAR_SENTINEL, "\x00CLEAR\x00");
    assert_eq!(DRAFT_PROGRESS_SENTINEL, "\x00PROGRESS\x00");
    assert_eq!(DRAFT_WS_EVENT_SENTINEL, "\x00WS_EVENT\x00");
}

#[test]
fn truncate_tool_args_covers_path_aliases_and_fallback_fields() {
    assert_eq!(
        truncate_tool_args_for_progress("file_read", &json!({"path": "/tmp/input.txt"}), 30),
        "/tmp/input.txt"
    );
    assert_eq!(
        truncate_tool_args_for_progress("file_write", &json!({"path": "/tmp/output.txt"}), 30),
        "/tmp/output.txt"
    );
    assert_eq!(
        truncate_tool_args_for_progress("notebook_edit", &json!({"filePath": "nb.ipynb"}), 30),
        "nb.ipynb"
    );
    assert_eq!(
        truncate_tool_args_for_progress("notebook_edit", &json!({"file_path": "alt.ipynb"}), 30),
        "alt.ipynb"
    );
    assert_eq!(
        truncate_tool_args_for_progress("file_edit", &json!({"file_path": "src/lib.rs"}), 30),
        "src/lib.rs"
    );
    assert_eq!(
        truncate_tool_args_for_progress("other", &json!({"action": "refresh"}), 30),
        "refresh"
    );
    assert_eq!(
        truncate_tool_args_for_progress("other", &json!({"query": "find docs"}), 30),
        "find docs"
    );
    assert_eq!(truncate_tool_args_for_progress("other", &json!({"unused": true}), 30), "");
}

#[test]
fn truncate_tool_args_uses_primary_paths_and_skips_non_string_aliases() {
    assert_eq!(
        truncate_tool_args_for_progress(
            "notebook_edit",
            &json!({"path": "primary.ipynb", "filePath": "secondary.ipynb"}),
            30,
        ),
        "primary.ipynb"
    );
    assert_eq!(
        truncate_tool_args_for_progress(
            "notebook_edit",
            &json!({"path": 1, "filePath": false, "file_path": "fallback.ipynb"}),
            30,
        ),
        "fallback.ipynb"
    );
    assert_eq!(
        truncate_tool_args_for_progress(
            "file_edit",
            &json!({"file_path": 1, "filePath": false, "path": "legacy.rs"}),
            30,
        ),
        "legacy.rs"
    );
}

#[test]
fn truncate_tool_args_applies_ellipsis_limit() {
    let hint = truncate_tool_args_for_progress(
        "shell",
        &json!({"command": "0123456789abcdefghijklmnopqrstuvwxyz"}),
        12,
    );

    assert_eq!(hint, "0123456789ab...");
}

#[test]
fn progress_label_includes_hint_when_available() {
    let args = json!({"command": "cargo clippy --all-targets"});

    assert!(tool_progress_label("shell", &args).contains("cargo clippy"));
    assert_eq!(tool_progress_actions("file_read"), ("读取中", "执行完毕"));
}

#[test]
fn progress_actions_distinguish_read_alias_from_generic_tools() {
    assert_eq!(tool_progress_actions("read"), ("读取中", "执行完毕"));
    assert_eq!(tool_progress_actions("shell"), ("执行中", "执行完毕"));
}

#[test]
fn file_progress_label_includes_path_and_clamped_ranges() {
    assert_eq!(
        tool_progress_label(
            "file_read",
            &json!({"path": "/tmp/app.log", "offset": -5, "limit": 25})
        ),
        "/tmp/app.log [offset=1, limit=25]"
    );
    assert_eq!(tool_progress_label("read", &json!({"limit": 10})), "文件 [limit=10]");
    assert_eq!(tool_progress_label("file_read", &json!({})), "文件");
}

#[test]
fn generic_progress_label_omits_empty_hint() {
    assert_eq!(tool_progress_label("custom_tool", &json!({})), "custom_tool");
    assert_eq!(
        tool_progress_label("custom_tool", &json!({"query": "search this"})),
        "custom_tool: search this"
    );
}
