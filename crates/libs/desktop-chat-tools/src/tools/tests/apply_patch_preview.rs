//! apply_patch_preview.rs 测试模块。
//!
//! 这些测试固定相邻解析器、视图辅助函数或状态计算的行为，防止后续 UI 重排时破坏边界契约。

use super::{
    ChangeFile, append_preview_gap, append_preview_line, apply_patch_paths_match,
    collect_apply_patch_changes, fill_missing_change_fields, find_apply_patch_change,
    looks_like_unified_diff, merge_apply_patch_changes, normalize_apply_patch_change_path,
    normalize_unified_diff_path, parse_apply_patch_change_files, parse_unified_diff_change_files,
};

/// 验证 apply patch paths match accepts relative absolute and diff prefixed paths 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn apply_patch_paths_match_accepts_relative_absolute_and_diff_prefixed_paths() {
    assert!(apply_patch_paths_match("a/src/main.rs", "src/main.rs"));
    assert!(apply_patch_paths_match("/Users/demo/project/src/main.rs", "src/main.rs"));
    assert!(apply_patch_paths_match("./src/lib.rs", "b/src/lib.rs"));
}

/// 验证 find apply patch change prefers preview with full file bodies 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn find_apply_patch_change_prefers_preview_with_full_file_bodies() {
    let changes = vec![
        ChangeFile {
            // path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            path: "/Users/demo/project/src/main.rs".into(),
            // additions 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            additions: 2,
            // deletions 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            deletions: 1,
            // before 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            before: String::new(),
            // after 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            after: String::new(),
        },
        ChangeFile {
            // path 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            path: "src/main.rs".into(),
            // additions 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            additions: 2,
            // deletions 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            deletions: 1,
            // before 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            before: "fn old_main() {}".into(),
            // after 保存该结构在渲染、解析或测试断言中需要直接访问的数据。
            after: "fn new_main() {}".into(),
        },
    ];

    let selected = find_apply_patch_change(&changes, "src/main.rs").expect("matched change");
    assert_eq!(selected.before, "fn old_main() {}");
    assert_eq!(selected.after, "fn new_main() {}");
}

/// 解析 apply patch change files builds preview from patch input 的输入文本，返回后续视图可以直接消费的结构化结果。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn parse_apply_patch_change_files_builds_preview_from_patch_input() {
    let patch = concat!(
        "*** Begin Patch\n",
        "*** Update File: src/main.rs\n",
        "@@ fn main() {\n",
        " fn main() {\n",
        "-    println!(\"old\");\n",
        "+    println!(\"new\");\n",
        " }\n",
        "*** End Patch"
    );

    let changes = parse_apply_patch_change_files(patch);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/main.rs");
    assert!(changes[0].before.contains("println!(\"old\")"));
    assert!(changes[0].after.contains("println!(\"new\")"));
    assert!(changes[0].additions >= 1);
    assert!(changes[0].deletions >= 1);
}

/// 验证 collect apply patch changes falls back to patch input when changes block is missing 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn collect_apply_patch_changes_falls_back_to_patch_input_when_changes_block_is_missing() {
    let input = concat!(
        "*** Begin Patch\n",
        "*** Add File: src/new.rs\n",
        "+fn main() {}\n",
        "*** End Patch"
    );

    let changes =
        collect_apply_patch_changes("Success. Updated the following files:\nA src/new.rs", input);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/new.rs");
    assert_eq!(changes[0].after, "fn main() {}");
}

/// 验证 collect apply patch changes returns deleted entry without preview body 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn collect_apply_patch_changes_returns_deleted_entry_without_preview_body() {
    let input = concat!("*** Begin Patch\n", "*** Delete File: src/old.rs\n", "*** End Patch");

    let changes =
        collect_apply_patch_changes("Success. Updated the following files:\nD src/old.rs", input);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/old.rs");
    assert!(changes[0].before.is_empty());
    assert!(changes[0].after.is_empty());
}

/// 验证 collect apply patch changes prefers diff body over empty changes entry for deleted file 这一行为，确保对应解析或视图契约稳定。
///
/// # 参数
///
/// 参数沿用调用点中的应用状态、工具输入或渲染上下文，不在这里扩大权限或补造状态。
///
/// # 返回值
///
/// 无返回值时，函数通过发布消息或更新局部状态完成交互。
///
/// # 错误处理
///
/// 本函数不吞掉底层错误；没有显式错误通道时，会用空集合、`None` 或现有 UI 状态表达不可用结果。
#[test]
fn collect_apply_patch_changes_prefers_diff_body_over_empty_changes_entry_for_deleted_file() {
    let output = concat!(
        "Success. Updated the following files:\n",
        "D src/old.rs\n\n",
        "<diff>\n",
        "--- a/src/old.rs\n",
        "+++ /dev/null\n",
        "@@\n",
        "-old line\n",
        "-another line\n",
        "</diff>\n\n",
        "<changes>\n",
        "{\"files\":[{\"path\":\"src/old.rs\",\"additions\":0,\"deletions\":2,\"before\":\"\",\"after\":\"\"}]}\n",
        "</changes>"
    );

    let changes = collect_apply_patch_changes(output, "");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/old.rs");
    assert!(changes[0].before.contains("old line"));
    assert!(changes[0].before.contains("another line"));
    assert!(changes[0].after.is_empty());
}

#[test]
fn parse_unified_diff_change_files_handles_add_modify_delete_and_metadata() {
    let diff = concat!(
        "diff --git a/src/new.rs b/src/new.rs\n",
        "new file mode 100644\n",
        "--- /dev/null\n",
        "+++ b/src/new.rs\n",
        "@@ -0,0 +1 @@\n",
        "+new\n",
        "diff --git a/src/mod.rs b/src/mod.rs\n",
        "index 111..222 100644\n",
        "--- a/src/mod.rs\n",
        "+++ b/src/mod.rs\n",
        "@@ -1 +1 @@\n",
        "-old\n",
        "+new\n",
        "\\ No newline at end of file\n",
        "diff --git a/src/old.rs b/src/old.rs\n",
        "deleted file mode 100644\n",
        "--- a/src/old.rs\n",
        "+++ /dev/null\n",
        "@@ -1 +0,0 @@\n",
        "-old\n"
    );

    let changes = parse_unified_diff_change_files(diff);
    assert_eq!(changes.len(), 3);
    assert_eq!(changes[0].path, "src/new.rs");
    assert_eq!(changes[0].additions, 1);
    assert_eq!(changes[1].deletions, 1);
    assert_eq!(changes[2].path, "src/old.rs");
    assert!(changes[2].after.is_empty());
}

#[test]
fn collect_apply_patch_changes_prefers_changes_block_then_fills_missing_diff_fields() {
    let output = concat!(
        "<diff>\n",
        "--- a/src/main.rs\n",
        "+++ b/src/main.rs\n",
        "@@\n",
        "-old\n",
        "+new\n",
        "</diff>\n",
        "<changes>\n",
        "{\"files\":[{\"path\":\"src/main.rs\",\"additions\":1,\"deletions\":1,\"before\":\"\",\"after\":\"\"}]}\n",
        "</changes>"
    );

    let changes = collect_apply_patch_changes(output, "");
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/main.rs");
    assert_eq!(changes[0].before, "old");
    assert_eq!(changes[0].after, "new");
}

#[test]
fn parse_apply_patch_change_files_returns_empty_for_invalid_patch() {
    assert!(parse_apply_patch_change_files("not a patch").is_empty());
}

// --- collect_apply_patch_changes edge cases ---

#[test]
fn collect_apply_patch_changes_returns_empty_when_no_sources_match() {
    let changes = collect_apply_patch_changes("no diff block", "no patch marker");
    assert!(changes.is_empty());
}

#[test]
fn collect_apply_patch_changes_uses_input_as_unified_diff_when_changes_and_diff_both_missing() {
    let input =
        concat!("--- a/src/main.rs\n", "+++ b/src/main.rs\n", "@@ -1 +1 @@\n", "-old\n", "+new\n");
    let changes = collect_apply_patch_changes("plain output without tags", input);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/main.rs");
    assert!(changes[0].before.contains("old"));
    assert!(changes[0].after.contains("new"));
}

#[test]
fn collect_apply_patch_changes_uses_input_as_apply_patch_when_other_fallbacks_miss() {
    let input = concat!(
        "*** Begin Patch\n",
        "*** Add File: src/generated.rs\n",
        "+fn auto() {}\n",
        "*** End Patch"
    );
    let changes = collect_apply_patch_changes("some output", input);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/generated.rs");
    assert_eq!(changes[0].after, "fn auto() {}");
}

// --- find_apply_patch_change edge cases ---

#[test]
fn find_apply_patch_change_returns_none_for_empty_list() {
    let result = find_apply_patch_change(&[], "src/main.rs");
    assert!(result.is_none());
}

#[test]
fn find_apply_patch_change_returns_none_when_path_does_not_match() {
    let changes = vec![ChangeFile {
        path: "src/other.rs".into(),
        additions: 0,
        deletions: 0,
        before: String::new(),
        after: String::new(),
    }];
    let result = find_apply_patch_change(&changes, "src/main.rs");
    assert!(result.is_none());
}

// --- apply_patch_paths_match edge cases ---

#[test]
fn apply_patch_paths_match_returns_false_for_unrelated_paths() {
    assert!(!apply_patch_paths_match("src/main.rs", "src/lib.rs"));
}

#[test]
fn apply_patch_paths_match_matches_by_filename_when_directory_differs() {
    assert!(apply_patch_paths_match("/a/b/main.rs", "/c/d/main.rs"));
}

#[test]
fn apply_patch_paths_match_returns_false_for_different_filenames() {
    assert!(!apply_patch_paths_match("src/main.rs", "src/lib.rs"));
}

// --- parse_apply_patch_change_files -- all Hunk variants ---

#[test]
fn parse_apply_patch_change_files_handles_add_hunk_with_multiple_lines() {
    let patch = concat!(
        "*** Begin Patch\n",
        "*** Add File: src/new.rs\n",
        "+fn one() {}\n",
        "+fn two() {}\n",
        "*** End Patch"
    );
    let changes = parse_apply_patch_change_files(patch);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/new.rs");
    assert_eq!(changes[0].additions, 2);
    assert_eq!(changes[0].deletions, 0);
    assert!(changes[0].before.is_empty());
    assert!(changes[0].after.contains("fn one() {}"));
    assert!(changes[0].after.contains("fn two() {}"));
}

#[test]
fn parse_apply_patch_change_files_handles_delete_hunk() {
    let patch = concat!("*** Begin Patch\n", "*** Delete File: src/obsolete.rs\n", "*** End Patch");
    let changes = parse_apply_patch_change_files(patch);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/obsolete.rs");
    assert_eq!(changes[0].additions, 0);
    assert_eq!(changes[0].deletions, 0);
    assert!(changes[0].before.is_empty());
    assert!(changes[0].after.is_empty());
}

#[test]
fn parse_apply_patch_change_files_handles_update_with_move_path() {
    let patch = concat!(
        "*** Begin Patch\n",
        "*** Update File: src/old_name.rs\n",
        "*** Move to: src/new_name.rs\n",
        "@@ some context\n",
        " unchanged\n",
        "-removed\n",
        "+added\n",
        "*** End Patch"
    );
    let changes = parse_apply_patch_change_files(patch);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/new_name.rs");
    assert!(changes[0].before.contains("removed"));
    assert!(changes[0].after.contains("added"));
    assert!(changes[0].before.contains("unchanged"));
    assert!(changes[0].after.contains("unchanged"));
}

#[test]
fn parse_apply_patch_change_files_handles_update_with_multiple_chunks() {
    let patch = concat!(
        "*** Begin Patch\n",
        "*** Update File: src/main.rs\n",
        "@@ first\n",
        "-old1\n",
        "+new1\n",
        "@@ second\n",
        "-old2\n",
        "+new2\n",
        "*** End Patch"
    );
    let changes = parse_apply_patch_change_files(patch);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/main.rs");
    assert!(changes[0].before.contains("old1"));
    assert!(changes[0].before.contains("old2"));
    assert!(changes[0].after.contains("new1"));
    assert!(changes[0].after.contains("new2"));
}

// --- parse_unified_diff_change_files edge cases ---

#[test]
fn parse_unified_diff_change_files_returns_empty_for_empty_input() {
    assert!(parse_unified_diff_change_files("").is_empty());
}

#[test]
fn parse_unified_diff_change_files_handles_space_context_lines() {
    let diff = concat!(
        "--- a/src/main.rs\n",
        "+++ b/src/main.rs\n",
        "@@ -1,3 +1,3 @@\n",
        " context1\n",
        "-old\n",
        "+new\n",
        " context2\n"
    );
    let changes = parse_unified_diff_change_files(diff);
    assert_eq!(changes.len(), 1);
    assert!(changes[0].before.contains("context1"));
    assert!(changes[0].before.contains("context2"));
    assert!(changes[0].after.contains("context1"));
    assert!(changes[0].after.contains("context2"));
    assert!(changes[0].before.contains("old"));
    assert!(changes[0].after.contains("new"));
}

#[test]
fn parse_unified_diff_change_files_handles_metadata_lines() {
    let diff = concat!(
        "diff --git a/src/a.rs b/src/a.rs\n",
        "index 111..222 100644\n",
        "--- a/src/a.rs\n",
        "+++ b/src/a.rs\n",
        "@@ -1 +1 @@\n",
        "-a\n",
        "+b\n",
        "\\ No newline at end of file\n",
        "diff --git a/src/b.rs b/src/b.rs\n",
        "new file mode 100644\n",
        "similarity index 100%\n",
        "--- /dev/null\n",
        "+++ b/src/b.rs\n",
        "@@ -0,0 +1 @@\n",
        "+b\n"
    );
    let changes = parse_unified_diff_change_files(diff);
    assert_eq!(changes.len(), 2);
    assert_eq!(changes[0].path, "src/a.rs");
    assert_eq!(changes[0].deletions, 1);
    assert_eq!(changes[0].additions, 1);
    assert_eq!(changes[1].path, "src/b.rs");
    assert_eq!(changes[1].additions, 1);
}

#[test]
fn parse_unified_diff_change_files_skips_lines_without_current_file_context() {
    let diff = concat!("--- a/src/main.rs\n", "@@\n", "-orphan\n");
    let changes = parse_unified_diff_change_files(diff);
    assert!(changes.is_empty());
}

#[test]
fn parse_unified_diff_change_files_handles_lines_outside_hunk_blocks() {
    let diff = concat!(
        "--- a/src/main.rs\n",
        "+++ b/src/main.rs\n",
        "@@ -1 +1 @@\n",
        "-a\n",
        "+b\n",
        "some random text\n", // not prefixed with +, -, or space - should be ignored
        " +indented\n"        // starts with space - treated as context
    );
    let changes = parse_unified_diff_change_files(diff);
    assert_eq!(changes.len(), 1);
    assert_eq!(changes[0].path, "src/main.rs");
    assert!(changes[0].before.contains("a"));
    assert!(changes[0].after.contains("b"));
}

// --- looks_like_unified_diff edge cases ---

#[test]
fn looks_like_unified_diff_returns_false_for_various_invalid_inputs() {
    assert!(!looks_like_unified_diff(""));
    assert!(!looks_like_unified_diff("   "));
    assert!(!looks_like_unified_diff("--- a/src/main.rs\n")); // missing \n+++ and \n@@
    assert!(!looks_like_unified_diff("+++ b/src/main.rs\n@@ -1 +1 @@\n")); // doesn't start with ---
    assert!(!looks_like_unified_diff("--- a/src/main.rs\n+++ b/src/main.rs\n")); // missing \n@@
}

// --- normalize_apply_patch_change_path variants ---

#[test]
fn normalize_apply_patch_change_path_strips_prefixes_and_normalizes_separators() {
    assert_eq!(normalize_apply_patch_change_path("a/src/main.rs"), "src/main.rs");
    assert_eq!(normalize_apply_patch_change_path("b/src/main.rs"), "src/main.rs");
    assert_eq!(normalize_apply_patch_change_path("./src/main.rs"), "src/main.rs");
    assert_eq!(normalize_apply_patch_change_path("src\\main.rs"), "src/main.rs");
    assert_eq!(normalize_apply_patch_change_path("plain.rs"), "plain.rs");
}

// --- normalize_unified_diff_path edge cases ---

#[test]
fn normalize_unified_diff_path_returns_none_for_dev_null_or_empty() {
    assert_eq!(normalize_unified_diff_path(""), None);
    assert_eq!(normalize_unified_diff_path("/dev/null"), None);
}

#[test]
fn normalize_unified_diff_path_strips_common_prefixes() {
    assert_eq!(normalize_unified_diff_path("a/src/main.rs").unwrap(), "src/main.rs");
    assert_eq!(normalize_unified_diff_path("b/src/lib.rs").unwrap(), "src/lib.rs");
    assert_eq!(normalize_unified_diff_path("src/main.rs").unwrap(), "src/main.rs");
}

// --- append_preview_gap behavior ---

#[test]
fn append_preview_gap_handles_empty_and_filled_buffer() {
    let mut buf = String::new();
    append_preview_gap(&mut buf);
    assert_eq!(buf, "");

    append_preview_line(&mut buf, "line1");
    append_preview_gap(&mut buf);
    assert_eq!(buf, "line1\n");

    append_preview_gap(&mut buf);
    assert_eq!(buf, "line1\n\n");

    append_preview_gap(&mut buf);
    assert_eq!(buf, "line1\n\n");
}

// --- append_preview_line behavior ---

#[test]
fn append_preview_line_handles_empty_and_nonempty_buffer() {
    let mut buf = String::new();
    append_preview_line(&mut buf, "first");
    assert_eq!(buf, "first");

    append_preview_line(&mut buf, "second");
    assert_eq!(buf, "first\nsecond");
}

// --- merge_apply_patch_changes with overlapping paths ---

#[test]
fn merge_apply_patch_changes_replaces_with_higher_scoring_change() {
    let primary = vec![ChangeFile {
        path: "src/main.rs".into(),
        additions: 1,
        deletions: 0,
        before: String::new(),
        after: String::new(),
    }];
    let fallback = vec![ChangeFile {
        path: "src/main.rs".into(),
        additions: 1,
        deletions: 1,
        before: "old".into(),
        after: "new".into(),
    }];
    let merged = merge_apply_patch_changes(primary, fallback);
    assert_eq!(merged.len(), 1);
    assert_eq!(merged[0].before, "old");
    assert_eq!(merged[0].after, "new");
}

#[test]
fn merge_apply_patch_changes_fills_missing_fields_when_primary_scored_higher() {
    let primary = vec![ChangeFile {
        path: "src/main.rs".into(),
        additions: 1,
        deletions: 1,
        before: "existing".into(),
        after: "new_content".into(),
    }];
    let fallback = vec![ChangeFile {
        path: "src/main.rs".into(),
        additions: 0,
        deletions: 0,
        before: String::new(),
        after: String::new(),
    }];
    let merged = merge_apply_patch_changes(primary, fallback);
    assert_eq!(merged.len(), 1);
    // Primary had higher score, so it stays. Fallback has nothing to fill.
    assert_eq!(merged[0].before, "existing");
    assert_eq!(merged[0].after, "new_content");
}

// --- fill_missing_change_fields ---

#[test]
fn fill_missing_change_fields_copies_empty_fields_from_source() {
    let mut target = ChangeFile {
        path: "src/main.rs".into(),
        additions: 0,
        deletions: 0,
        before: String::new(),
        after: String::new(),
    };
    let source = ChangeFile {
        path: "src/other.rs".into(),
        additions: 5,
        deletions: 3,
        before: "fill_before".into(),
        after: "fill_after".into(),
    };
    fill_missing_change_fields(&mut target, &source);
    assert_eq!(target.before, "fill_before");
    assert_eq!(target.after, "fill_after");
    assert_eq!(target.additions, 5);
    assert_eq!(target.deletions, 3);
    assert_eq!(target.path, "src/main.rs");
}

#[test]
fn fill_missing_change_fields_does_not_overwrite_existing_values() {
    let mut target = ChangeFile {
        path: "src/main.rs".into(),
        additions: 2,
        deletions: 1,
        before: "existing".into(),
        after: String::new(),
    };
    let source = ChangeFile {
        path: "src/other.rs".into(),
        additions: 99,
        deletions: 99,
        before: "overwrite".into(),
        after: "new".into(),
    };
    fill_missing_change_fields(&mut target, &source);
    assert_eq!(target.before, "existing");
    assert_eq!(target.after, "new");
    assert_eq!(target.additions, 2);
    assert_eq!(target.deletions, 1);
}
