//! 验证通用异步生成、UTF-8 安全截断及三态可选值辅助逻辑。
//! 截断断言按字符而非字节计数，并覆盖边界内外索引和尾部空白处理。

use super::{MaybeSet, floor_utf8_char_boundary, spawn, truncate_with_ellipsis};

#[tokio::test]
async fn spawn_returns_future_output() {
    let handle = spawn(async { 42 });

    assert_eq!(handle.await.expect("spawned future should complete"), 42);
}

#[test]
fn truncate_returns_original_at_or_below_character_limit() {
    assert_eq!(truncate_with_ellipsis("", 0), "");
    assert_eq!(truncate_with_ellipsis("hello", 5), "hello");
    assert_eq!(truncate_with_ellipsis("hi", 5), "hi");
}

#[test]
fn truncate_zero_limit_marks_non_empty_input_as_truncated() {
    assert_eq!(truncate_with_ellipsis("hello", 0), "...");
}

#[test]
fn truncate_counts_unicode_characters_instead_of_bytes() {
    assert_eq!(truncate_with_ellipsis("你好世界", 2), "你好...");
}

#[test]
fn truncate_removes_trailing_whitespace_from_retained_prefix() {
    assert_eq!(truncate_with_ellipsis("hello   world", 8), "hello...");
}

#[test]
fn floor_utf8_char_boundary_clamps_indices_at_or_past_end() {
    assert_eq!(floor_utf8_char_boundary("", 0), 0);
    assert_eq!(floor_utf8_char_boundary("你好", 6), 6);
    assert_eq!(floor_utf8_char_boundary("你好", usize::MAX), 6);
}

#[test]
fn floor_utf8_char_boundary_keeps_existing_boundaries() {
    assert_eq!(floor_utf8_char_boundary("hello", 3), 3);
    assert_eq!(floor_utf8_char_boundary("你好", 3), 3);
}

#[test]
fn floor_utf8_char_boundary_moves_inside_character_to_its_start() {
    assert_eq!(floor_utf8_char_boundary("你好", 1), 0);
    assert_eq!(floor_utf8_char_boundary("你好", 2), 0);
    assert_eq!(floor_utf8_char_boundary("你好", 4), 3);
    assert_eq!(floor_utf8_char_boundary("你好", 5), 3);
}

#[test]
fn maybe_set_preserves_set_unset_and_null_as_distinct_states() {
    let set = MaybeSet::Set(String::from("value"));
    let unset: MaybeSet<String> = MaybeSet::Unset;
    let null: MaybeSet<String> = MaybeSet::Null;

    assert_eq!(set.clone(), MaybeSet::Set(String::from("value")));
    assert_ne!(set, unset);
    assert_ne!(unset, null);
    assert_ne!(null, MaybeSet::Set(String::from("value")));
}
