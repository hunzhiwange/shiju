use std::path::Path;

use super::*;

fn chunk(
    old_lines: &[&str],
    new_lines: &[&str],
    change_context: Option<&str>,
    is_end_of_file: Option<bool>,
) -> UpdateFileChunk {
    UpdateFileChunk {
        old_lines: old_lines.iter().map(|line| line.to_string()).collect(),
        new_lines: new_lines.iter().map(|line| line.to_string()).collect(),
        change_context: change_context.map(str::to_string),
        is_end_of_file,
    }
}

fn temp_file(contents: &str) -> tempfile::NamedTempFile {
    let file = tempfile::NamedTempFile::new().expect("temp file should be created");
    std::fs::write(file.path(), contents).expect("temp file contents should be written");
    file
}

#[test]
fn normalize_unicode_replaces_patch_confusable_characters() {
    let value = "\u{2018}a\u{2019} \u{201C}b\u{201D} a\u{2010}b a\u{2011}b a\u{2012}b a\u{2013}b a\u{2014}b a\u{2015}b x\u{2026}y a\u{00A0}b";

    assert_eq!(normalize_unicode(value), "'a' \"b\" a-b a-b a-b a-b a-b a-b x.y a b");
}

#[test]
fn seek_sequence_matches_exact_then_trimmed_then_unicode_normalized_lines() {
    let exact_lines = vec!["alpha".to_string()];
    let exact_pattern = vec!["alpha".to_string()];
    assert_eq!(seek_sequence(&exact_lines, &exact_pattern, 0, false), Some(0));

    let rstrip_lines = vec!["alpha   ".to_string()];
    let rstrip_pattern = vec!["alpha".to_string()];
    assert_eq!(seek_sequence(&rstrip_lines, &rstrip_pattern, 0, false), Some(0));

    let trim_lines = vec!["  alpha".to_string()];
    let trim_pattern = vec!["alpha".to_string()];
    assert_eq!(seek_sequence(&trim_lines, &trim_pattern, 0, false), Some(0));

    let unicode_lines = vec!["status: \u{201C}ready\u{201D}".to_string()];
    let unicode_pattern = vec!["status: \"ready\"".to_string()];
    assert_eq!(seek_sequence(&unicode_lines, &unicode_pattern, 0, false), Some(0));
}

#[test]
fn seek_sequence_rejects_empty_pattern() {
    let lines = vec!["alpha".to_string()];

    assert_eq!(seek_sequence(&lines, &[], 0, false), None);
}

#[test]
fn seek_sequence_matches_eof_with_each_comparison_strategy() {
    let exact_lines = vec!["first".to_string(), "last".to_string()];
    let exact_pattern = vec!["last".to_string()];
    assert_eq!(seek_sequence(&exact_lines, &exact_pattern, 0, true), Some(1));

    let rstrip_lines = vec!["first".to_string(), "last   ".to_string()];
    let rstrip_pattern = vec!["last".to_string()];
    assert_eq!(seek_sequence(&rstrip_lines, &rstrip_pattern, 0, true), Some(1));

    let trim_lines = vec!["first".to_string(), "  last".to_string()];
    let trim_pattern = vec!["last".to_string()];
    assert_eq!(seek_sequence(&trim_lines, &trim_pattern, 0, true), Some(1));

    let unicode_lines = vec!["first".to_string(), "\u{201C}last\u{201D}".to_string()];
    let unicode_pattern = vec!["\"last\"".to_string()];
    assert_eq!(seek_sequence(&unicode_lines, &unicode_pattern, 0, true), Some(1));
}

#[test]
fn seek_sequence_returns_none_when_eof_match_is_before_start() {
    let lines = vec!["needle".to_string()];
    let pattern = vec!["needle".to_string()];

    assert_eq!(seek_sequence(&lines, &pattern, 1, true), None);
}

#[test]
fn seek_sequence_matches_multiline_pattern_in_middle_of_lines() {
    let lines =
        vec!["intro".to_string(), "alpha".to_string(), "beta".to_string(), "outro".to_string()];
    let pattern = vec!["alpha".to_string(), "beta".to_string()];

    assert_eq!(seek_sequence(&lines, &pattern, 0, false), Some(1));
}

#[test]
fn seek_sequence_falls_back_to_loop_when_eof_tail_does_not_match() {
    // eof 为真但末尾行与模式不同，应跳过尾部分支走主循环匹配中间行。
    let lines = vec!["target".to_string(), "trailing".to_string()];
    let pattern = vec!["target".to_string()];

    assert_eq!(seek_sequence(&lines, &pattern, 0, true), Some(0));
}

#[test]
fn seek_sequence_skips_eof_branch_when_lines_shorter_than_pattern() {
    let lines = vec!["only".to_string()];
    let pattern = vec!["only".to_string(), "extra".to_string()];

    assert_eq!(seek_sequence(&lines, &pattern, 0, true), None);
}

#[test]
fn seek_sequence_honors_start_index_skipping_earlier_match() {
    let lines = vec!["dup".to_string(), "mid".to_string(), "dup".to_string()];
    let pattern = vec!["dup".to_string()];

    assert_eq!(seek_sequence(&lines, &pattern, 1, false), Some(2));
}

#[test]
fn seek_sequence_returns_none_when_no_strategy_matches() {
    let lines = vec!["alpha".to_string(), "beta".to_string()];
    let pattern = vec!["zeta".to_string()];

    assert_eq!(seek_sequence(&lines, &pattern, 0, false), None);
}

#[test]
fn compute_replacements_inserts_at_current_context_when_old_lines_are_empty() {
    let lines = vec!["top".to_string(), "anchor".to_string(), "bottom".to_string()];
    let chunks = vec![chunk(&[], &["inserted"], Some("anchor"), None)];

    let replacements = compute_replacements(&lines, Path::new("sample.txt"), &chunks)
        .expect("insertion should be computed");

    assert_eq!(replacements, vec![(2, 0, vec!["inserted".to_string()])]);
}

#[test]
fn compute_replacements_caps_insertion_after_missing_context_at_file_end() {
    let lines = vec!["top".to_string(), "bottom".to_string()];
    let chunks = vec![chunk(&[], &["tail"], Some("missing"), None)];

    let replacements = compute_replacements(&lines, Path::new("sample.txt"), &chunks)
        .expect("insertion without matching context should still be computed");

    assert_eq!(replacements, vec![(0, 0, vec!["tail".to_string()])]);
}

#[test]
fn compute_replacements_drops_trailing_empty_old_line_for_matching() {
    let lines = vec!["alpha".to_string(), "beta".to_string()];
    let chunks = vec![chunk(&["beta", ""], &["gamma", ""], None, None)];

    let replacements = compute_replacements(&lines, Path::new("sample.txt"), &chunks)
        .expect("trailing empty line should be ignored for matching");

    assert_eq!(replacements, vec![(1, 1, vec!["gamma".to_string()])]);
}

#[test]
fn compute_replacements_drops_trailing_empty_old_line_without_trimming_new_lines() {
    let lines = vec!["alpha".to_string(), "beta".to_string()];
    let chunks = vec![chunk(&["beta", ""], &["gamma"], None, None)];

    let replacements = compute_replacements(&lines, Path::new("sample.txt"), &chunks)
        .expect("trailing empty old line should be ignored for matching");

    assert_eq!(replacements, vec![(1, 1, vec!["gamma".to_string()])]);
}

#[test]
fn compute_replacements_keeps_new_trailing_empty_when_old_pattern_keeps_last_line() {
    let lines = vec!["alpha".to_string(), "beta".to_string()];
    let chunks = vec![chunk(&["beta"], &["gamma", ""], None, None)];

    let replacements = compute_replacements(&lines, Path::new("sample.txt"), &chunks)
        .expect("replacement should be computed");

    assert_eq!(replacements, vec![(1, 1, vec!["gamma".to_string(), String::new()])]);
}

#[test]
fn compute_replacements_returns_contextual_error_when_old_lines_are_missing() {
    let lines = vec!["alpha".to_string()];
    let chunks = vec![chunk(&["missing"], &["new"], None, None)];

    let error = compute_replacements(&lines, Path::new("sample.txt"), &chunks)
        .expect_err("missing old lines should be reported");

    match error {
        Error::ComputeReplacements(message) => {
            assert!(message.contains("sample.txt"));
            assert!(message.contains("期望匹配的旧内容"));
            assert!(message.contains("missing"));
        }
        Error::Io(_) | Error::Parse(_) => panic!("unexpected error variant"),
    }
}

#[test]
fn compute_replacements_advances_line_index_past_matched_context() {
    // change_context 命中后 line_index 前移，旧内容应在上下文之后定位。
    let lines =
        vec!["dup".to_string(), "marker".to_string(), "dup".to_string(), "tail".to_string()];
    let chunks = vec![chunk(&["dup"], &["DUP"], Some("marker"), None)];

    let replacements = compute_replacements(&lines, Path::new("sample.txt"), &chunks)
        .expect("replacement should anchor after the matched context");

    assert_eq!(replacements, vec![(2, 1, vec!["DUP".to_string()])]);
}

#[test]
fn compute_replacements_sorts_multiple_chunks_by_start_index() {
    let lines = vec!["one".to_string(), "two".to_string(), "three".to_string(), "four".to_string()];
    let chunks =
        vec![chunk(&["one"], &["ONE"], None, None), chunk(&["three"], &["THREE"], None, None)];

    let replacements = compute_replacements(&lines, Path::new("sample.txt"), &chunks)
        .expect("both chunks should be located");

    assert_eq!(
        replacements,
        vec![(0, 1, vec!["ONE".to_string()]), (2, 1, vec!["THREE".to_string()])]
    );
}

#[test]
fn compute_replacements_keeps_error_when_trailing_empty_pop_still_misses() {
    // 模式尾部为空被裁掉后仍找不到匹配，应返回上下文错误。
    let lines = vec!["alpha".to_string()];
    let chunks = vec![chunk(&["missing", ""], &["new", ""], None, None)];

    let error = compute_replacements(&lines, Path::new("sample.txt"), &chunks)
        .expect_err("popping the trailing empty line should not create a match");

    match error {
        Error::ComputeReplacements(message) => assert!(message.contains("missing")),
        Error::Io(_) | Error::Parse(_) => panic!("unexpected error variant"),
    }
}

#[test]
fn apply_replacements_applies_segments_from_end_to_start() {
    let lines = vec!["a".to_string(), "b".to_string(), "c".to_string()];
    let replacements =
        vec![(0, 1, vec!["A".to_string()]), (2, 1, vec!["C".to_string(), "D".to_string()])];

    let result = apply_replacements(&lines, &replacements);

    assert_eq!(result, vec!["A", "b", "C", "D"]);
}

#[test]
fn generate_unified_diff_includes_changed_added_removed_and_context_lines() {
    let diff = generate_unified_diff("same\nold\nremoved\n", "same\nnew\n\nadded\n");

    assert_eq!(diff, "@@ -1 +1 @@\n same\n-old\n+new\n-removed\n+added\n");
}

#[test]
fn generate_unified_diff_returns_empty_for_identical_content() {
    assert_eq!(generate_unified_diff("same\n", "same\n"), "");
}

#[test]
fn derive_new_contents_from_chunks_applies_replacements_and_generates_diff() {
    let file = temp_file("alpha\nbeta\ngamma\n");
    let chunks = vec![chunk(&["beta"], &["BETA"], Some("alpha"), None)];

    let update = derive_new_contents_from_chunks(file.path(), &chunks)
        .expect("new contents should be derived");

    assert_eq!(update.content, "alpha\nBETA\ngamma\n");
    assert_eq!(update.unified_diff, "@@ -1 +1 @@\n alpha\n-beta\n+BETA\n gamma\n");
}

#[test]
fn derive_new_contents_from_chunks_appends_trailing_newline_to_non_empty_result() {
    let file = temp_file("alpha");
    let chunks = vec![chunk(&["alpha"], &["beta"], None, None)];

    let update = derive_new_contents_from_chunks(file.path(), &chunks)
        .expect("new contents should be derived");

    assert_eq!(update.content, "beta\n");
}

#[test]
fn derive_new_contents_from_chunks_preserves_empty_file_without_extra_newline() {
    let file = temp_file("");
    let chunks = vec![chunk(&[], &[], None, None)];

    let update = derive_new_contents_from_chunks(file.path(), &chunks)
        .expect("empty content should be derived");

    assert_eq!(update.content, "");
    assert_eq!(update.unified_diff, "");
}

#[test]
fn derive_new_contents_from_chunks_returns_io_error_for_missing_file() {
    let error = derive_new_contents_from_chunks(Path::new("definitely-missing-file.txt"), &[])
        .expect_err("missing file should return an io error");

    match error {
        Error::Io(_) => {}
        Error::Parse(_) | Error::ComputeReplacements(_) => panic!("unexpected error variant"),
    }
}

#[test]
fn derive_new_contents_from_chunks_returns_compute_error_for_missing_old_lines() {
    let file = temp_file("alpha\n");
    let chunks = vec![chunk(&["missing"], &["beta"], None, None)];

    let error =
        derive_new_contents_from_chunks(file.path(), &chunks).expect_err("missing old lines fail");

    match error {
        Error::ComputeReplacements(message) => assert!(message.contains("missing")),
        Error::Io(_) | Error::Parse(_) => panic!("unexpected error variant"),
    }
}
