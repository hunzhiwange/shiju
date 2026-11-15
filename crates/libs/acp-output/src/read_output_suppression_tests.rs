//! 验证读类工具识别仅依据明确类型或标题首词，避免误抑制输出。
//!
//! 最小工具描述符用于隔离类型与标题启发式，不涉及实际输出写入。

use super::*;

// 下列行为用例覆盖显式类型、标题首词、空标题和反例优先级。
#[test]
fn read_like_tool_detects_explicit_kind_case_insensitively() {
    assert!(is_read_like_tool(&ReadLikeToolDescriptor {
        title: None,
        kind: Some(" READ ".to_string()),
    }));
}

#[test]
fn read_like_tool_infers_from_title_head_only() {
    for title in ["read: file", "cat file", "open: file", "view file"] {
        assert!(is_read_like_tool(&ReadLikeToolDescriptor {
            title: Some(title.to_string()),
            kind: None,
        }));
    }

    assert!(!is_read_like_tool(&ReadLikeToolDescriptor {
        title: Some("edit: then read".to_string()),
        kind: None,
    }));
}

#[test]
fn read_like_tool_rejects_non_read_kind_without_read_like_title() {
    assert!(!is_read_like_tool(&ReadLikeToolDescriptor {
        title: Some("edit file".to_string()),
        kind: Some("write".to_string()),
    }));
}

#[test]
fn read_like_tool_ignores_missing_or_blank_title() {
    assert!(!is_read_like_tool(&ReadLikeToolDescriptor { title: None, kind: None }));
    assert!(!is_read_like_tool(&ReadLikeToolDescriptor {
        title: Some("   ".to_string()),
        kind: None,
    }));
}

#[test]
fn read_like_tool_ignores_empty_title_head() {
    assert!(!is_read_like_tool(&ReadLikeToolDescriptor {
        title: Some(": read file".to_string()),
        kind: None,
    }));
}
