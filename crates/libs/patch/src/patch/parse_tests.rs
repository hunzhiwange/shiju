use super::parse_patch;
use crate::patch::{Error, Hunk};

#[test]
fn parse_patch_reads_add_delete_update_move_and_chunks() {
    let parsed = parse_patch(
        r#"
ignored prefix
*** Begin Patch
*** Add File: src/new.rs
+fn main() {
+    println!("new");
+}
*** Delete File: src/old.rs
*** Update File: src/lib.rs
*** Move to: src/core.rs
@@ fn run()
 context
-old
+new
@@
-tail
+tail_new
*** End Patch
ignored suffix
"#,
    )
    .expect("patch should parse");

    assert_eq!(parsed.hunks.len(), 3);

    match &parsed.hunks[0] {
        Hunk::Add { path, contents } => {
            assert_eq!(path, "src/new.rs");
            assert_eq!(contents, "fn main() {\n    println!(\"new\");\n}");
        }
        other => panic!("expected add hunk, got {other:?}"),
    }

    match &parsed.hunks[1] {
        Hunk::Delete { path } => assert_eq!(path, "src/old.rs"),
        other => panic!("expected delete hunk, got {other:?}"),
    }

    match &parsed.hunks[2] {
        Hunk::Update { path, move_path, chunks } => {
            assert_eq!(path, "src/lib.rs");
            assert_eq!(move_path.as_deref(), Some("src/core.rs"));
            assert_eq!(chunks.len(), 2);
            assert_eq!(chunks[0].change_context.as_deref(), Some("fn run()"));
            assert_eq!(chunks[0].old_lines, vec!["context", "old"]);
            assert_eq!(chunks[0].new_lines, vec!["context", "new"]);
            assert_eq!(chunks[0].is_end_of_file, None);
            assert_eq!(chunks[1].change_context, None);
            assert_eq!(chunks[1].old_lines, vec!["tail"]);
            assert_eq!(chunks[1].new_lines, vec!["tail_new"]);
        }
        other => panic!("expected update hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_strips_single_quoted_heredoc() {
    let parsed = parse_patch(
        r#"apply_patch <<'PATCH'
*** Begin Patch
*** Add File: note.txt
+hello
PATCH
*** End Patch
PATCH
"#,
    )
    .expect("quoted heredoc patch should parse");

    match parsed.hunks.as_slice() {
        [Hunk::Add { path, contents }] => {
            assert_eq!(path, "note.txt");
            assert_eq!(contents, "hello");
        }
        other => panic!("expected single add hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_keeps_full_input_when_heredoc_quote_is_unclosed() {
    let parsed = parse_patch("apply_patch <<'PATCH\n*** Begin Patch\n*** End Patch").unwrap();

    assert!(parsed.hunks.is_empty());
}

#[test]
fn parse_patch_keeps_full_input_when_heredoc_body_does_not_start_on_next_line() {
    let parsed =
        parse_patch("apply_patch <<PATCH trailing\n*** Begin Patch\n*** End Patch").unwrap();

    assert!(parsed.hunks.is_empty());
}

#[test]
fn parse_patch_keeps_full_input_when_heredoc_end_marker_is_missing() {
    let parsed = parse_patch(
        "apply_patch <<PATCH\n*** Begin Patch\n*** Add File: note.txt\n+hello\n*** End Patch\n",
    )
    .expect("patch body should still parse when heredoc terminator is missing");

    match parsed.hunks.as_slice() {
        [Hunk::Add { path, contents }] => {
            assert_eq!(path, "note.txt");
            assert_eq!(contents, "hello");
        }
        other => panic!("expected single add hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_supports_double_quoted_and_unquoted_heredoc_markers() {
    let double_quoted = parse_patch(
        "apply_patch <<\"PATCH\"\n*** Begin Patch\n*** Delete File: a.txt\n*** End Patch\nPATCH\n",
    )
    .expect("double quoted heredoc should parse");
    let unquoted = parse_patch(
        "apply_patch <<PATCH\n*** Begin Patch\n*** Delete File: b.txt\n*** End Patch\nPATCH\n",
    )
    .expect("unquoted heredoc should parse");

    match double_quoted.hunks.as_slice() {
        [Hunk::Delete { path }] => assert_eq!(path, "a.txt"),
        other => panic!("expected delete hunk, got {other:?}"),
    }
    match unquoted.hunks.as_slice() {
        [Hunk::Delete { path }] => assert_eq!(path, "b.txt"),
        other => panic!("expected delete hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_supports_chinese_markers_and_fullwidth_colons() {
    let parsed = parse_patch(
        r#"
*** 开始补丁
*** 添加文件： zh.txt
+你好
*** 更新文件： src/原.rs
*** 移动到： src/新.rs
@@ 片段
-旧
+新
*** 删除文件： old.txt
*** 结束补丁
"#,
    )
    .expect("chinese patch should parse");

    assert_eq!(parsed.hunks.len(), 3);
    match &parsed.hunks[0] {
        Hunk::Add { path, contents } => {
            assert_eq!(path, "zh.txt");
            assert_eq!(contents, "你好");
        }
        other => panic!("expected add hunk, got {other:?}"),
    }
    match &parsed.hunks[1] {
        Hunk::Update { path, move_path, chunks } => {
            assert_eq!(path, "src/原.rs");
            assert_eq!(move_path.as_deref(), Some("src/新.rs"));
            assert_eq!(chunks[0].change_context.as_deref(), Some("片段"));
            assert_eq!(chunks[0].old_lines, vec!["旧"]);
            assert_eq!(chunks[0].new_lines, vec!["新"]);
        }
        other => panic!("expected update hunk, got {other:?}"),
    }
    match &parsed.hunks[2] {
        Hunk::Delete { path } => assert_eq!(path, "old.txt"),
        other => panic!("expected delete hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_accepts_header_colon_variants_and_colonless_headers() {
    let parsed = parse_patch(
        r#"
*** Begin Patch
*** Add File no-colon.txt
+plain
*** 删除文件: ascii-colon.txt
*** Update File: spaced.txt
@@
-old
+new
*** End Patch
"#,
    )
    .expect("header variants should parse");

    assert_eq!(parsed.hunks.len(), 3);
    assert!(matches!(
        &parsed.hunks[0],
        Hunk::Add { path, contents } if path == "no-colon.txt" && contents == "plain"
    ));
    assert!(matches!(&parsed.hunks[1], Hunk::Delete { path } if path == "ascii-colon.txt"));
    assert!(matches!(&parsed.hunks[2], Hunk::Update { path, .. } if path == "spaced.txt"));
}

#[test]
fn parse_patch_records_end_of_file_marker() {
    let parsed = parse_patch(
        r#"
*** Begin Patch
*** Update File: src/lib.rs
@@
-last
+next
*** End of File
*** End Patch
"#,
    )
    .expect("patch should parse");

    match parsed.hunks.as_slice() {
        [Hunk::Update { chunks, .. }] => {
            assert_eq!(chunks.len(), 1);
            assert_eq!(chunks[0].is_end_of_file, Some(true));
            assert_eq!(chunks[0].old_lines, vec!["last"]);
            assert_eq!(chunks[0].new_lines, vec!["next"]);
        }
        other => panic!("expected update hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_ignores_non_hunk_lines_and_update_noise() {
    let parsed = parse_patch(
        r#"
*** Begin Patch
plain text
*** Update File: src/lib.rs
noise before chunk
@@
! ignored marker
 context
*** End Patch
"#,
    )
    .expect("patch should parse");

    match parsed.hunks.as_slice() {
        [Hunk::Update { chunks, .. }] => {
            assert_eq!(chunks.len(), 1);
            assert_eq!(chunks[0].old_lines, vec!["context"]);
            assert_eq!(chunks[0].new_lines, vec!["context"]);
        }
        other => panic!("expected update hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_returns_empty_result_for_marker_only_patch() {
    let parsed =
        parse_patch("*** Begin Patch\nunrecognized\n*** Move to: nowhere\n*** End Patch").unwrap();

    assert!(parsed.hunks.is_empty());
}

#[test]
fn parse_patch_errors_when_markers_are_missing() {
    let err = parse_patch("*** Add File: a.txt\n+hello").unwrap_err();

    assert_parse_error_contains(err, "missing Begin/End markers");
}

#[test]
fn parse_patch_errors_when_begin_is_after_end() {
    let err = parse_patch("*** End Patch\n*** Begin Patch").unwrap_err();

    assert_parse_error_contains(err, "Begin marker after End marker");
}

#[test]
fn parse_patch_keeps_full_input_when_heredoc_end_marker_missing() {
    // 打开 heredoc 且正文起始正确，但缺少结尾标记行，应回退为完整输入。
    let parsed = parse_patch(
        "apply_patch <<'PATCH'\n*** Begin Patch\n*** Delete File: a.txt\n*** End Patch\n",
    )
    .expect("patch should still parse from full input");

    match parsed.hunks.as_slice() {
        [Hunk::Delete { path }] => assert_eq!(path, "a.txt"),
        other => panic!("expected delete hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_add_file_with_no_plus_lines_yields_empty_contents() {
    // Add File 后紧跟下一个 header，没有任何 `+` 行，内容应为空且不触发末尾换行裁剪。
    let parsed = parse_patch(
        r#"
*** Begin Patch
*** Add File: empty.txt
*** Delete File: gone.txt
*** End Patch
"#,
    )
    .expect("patch should parse");

    match parsed.hunks.as_slice() {
        [Hunk::Add { path, contents }, Hunk::Delete { path: del }] => {
            assert_eq!(path, "empty.txt");
            assert_eq!(contents, "");
            assert_eq!(del, "gone.txt");
        }
        other => panic!("expected add then delete hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_update_chunk_terminated_by_next_header() {
    // update chunk 在遇到下一个 `***` header 时结束，而非文件结尾标记。
    let parsed = parse_patch(
        r#"
*** Begin Patch
*** Update File: src/a.rs
@@
-old
+new
*** Delete File: src/b.rs
*** End Patch
"#,
    )
    .expect("patch should parse");

    match parsed.hunks.as_slice() {
        [Hunk::Update { path, chunks, .. }, Hunk::Delete { path: del }] => {
            assert_eq!(path, "src/a.rs");
            assert_eq!(chunks.len(), 1);
            assert_eq!(chunks[0].old_lines, vec!["old"]);
            assert_eq!(chunks[0].new_lines, vec!["new"]);
            assert_eq!(chunks[0].is_end_of_file, None);
            assert_eq!(del, "src/b.rs");
        }
        other => panic!("expected update then delete hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_update_without_move_header() {
    // update 之后不是 move header 时，move_path 应为 None 且不消费该行。
    let parsed = parse_patch(
        r#"
*** Begin Patch
*** Update File: src/a.rs
@@
 keep
*** End Patch
"#,
    )
    .expect("patch should parse");

    match parsed.hunks.as_slice() {
        [Hunk::Update { path, move_path, chunks }] => {
            assert_eq!(path, "src/a.rs");
            assert_eq!(move_path.as_deref(), None);
            assert_eq!(chunks[0].old_lines, vec!["keep"]);
            assert_eq!(chunks[0].new_lines, vec!["keep"]);
        }
        other => panic!("expected single update hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_update_with_no_chunks() {
    // Update File 后直接结束，chunks 为空。
    let parsed = parse_patch("*** Begin Patch\n*** Update File: src/a.rs\n*** End Patch")
        .expect("patch should parse");

    match parsed.hunks.as_slice() {
        [Hunk::Update { path, move_path, chunks }] => {
            assert_eq!(path, "src/a.rs");
            assert_eq!(move_path.as_deref(), None);
            assert!(chunks.is_empty());
        }
        other => panic!("expected single update hunk, got {other:?}"),
    }
}

#[test]
fn parse_patch_end_of_file_marker_terminates_chunk_collection() {
    // *** End of File 标记结束 chunk 后，后续无更多 chunk。
    let parsed = parse_patch(
        r#"
*** Begin Patch
*** Update File: src/a.rs
@@ ctx
-a
+b
*** End of File
*** Delete File: src/b.rs
*** End Patch
"#,
    )
    .expect("patch should parse");

    match parsed.hunks.as_slice() {
        [Hunk::Update { chunks, .. }, Hunk::Delete { path }] => {
            assert_eq!(chunks.len(), 1);
            assert_eq!(chunks[0].is_end_of_file, Some(true));
            assert_eq!(path, "src/b.rs");
        }
        other => panic!("expected update then delete hunk, got {other:?}"),
    }
}

fn assert_parse_error_contains(err: Error, expected: &str) {
    match err {
        Error::Parse(message) => assert!(
            message.contains(expected),
            "expected parse error to contain {expected:?}, got {message:?}"
        ),
        other => panic!("expected parse error, got {other:?}"),
    }
}
