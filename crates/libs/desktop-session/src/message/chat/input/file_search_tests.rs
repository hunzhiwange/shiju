use super::file_search::*;
#[test]
fn format_drop_mentions_appends_trailing_slash_for_directories() {
    let mentions = format_drop_mentions(
        Some("/workspace"),
        &[DroppedPath { path: "/workspace/src/components".to_string(), is_dir: true }],
        None,
    );

    assert_eq!(mentions, vec!["src/components/".to_string()]);
}
#[test]
fn ranked_file_search_entries_include_deduped_parent_directories() {
    let entries = build_ranked_file_search_entries(
        &["src/main.rs".to_string(), "src/bin/tool.rs".to_string(), "README.md".to_string()],
        "",
    );

    assert!(entries.iter().any(|entry| entry.path == "src/" && entry.is_dir));
    assert!(entries.iter().any(|entry| entry.path == "src/bin/" && entry.is_dir));
    assert_eq!(entries.iter().filter(|entry| entry.path == "src/").count(), 1);
    assert!(entries.iter().any(|entry| entry.path == "README.md" && !entry.is_dir));
}
#[test]
fn ranked_file_search_entries_prioritize_filename_matches() {
    let entries = build_ranked_file_search_entries(
        &[
            "docs/readme.md".to_string(),
            "src/search_panel.rs".to_string(),
            "src/file_search.rs".to_string(),
        ],
        "file_search",
    );

    assert_eq!(entries.first().map(|entry| entry.path.as_str()), Some("src/file_search.rs"));
}
#[test]
fn ranked_file_search_entries_filter_non_matching_queries() {
    let entries =
        build_ranked_file_search_entries(&["src/main.rs".to_string()], "zzzz-not-present");

    assert!(entries.is_empty());
}
#[test]
fn ranked_file_search_entries_normalize_parent_separators_and_keep_original_file_path() {
    let entries = build_ranked_file_search_entries(&["src\\bin\\tool.rs".to_string()], "");

    assert_eq!(entries[0].path, "src\\bin\\tool.rs");
    assert!(!entries[0].is_dir);
    assert!(entries.iter().any(|entry| entry.path == "src/" && entry.is_dir));
    assert!(entries.iter().any(|entry| entry.path == "src/bin/" && entry.is_dir));
}
#[test]
fn ranked_file_search_entries_put_files_before_directories_for_equal_scores() {
    let entries = build_ranked_file_search_entries(
        &["zeta/main.rs".to_string(), "alpha/lib.rs".to_string()],
        "",
    );
    let first_directory = entries.iter().position(|entry| entry.is_dir).expect("directory entry");

    assert!(entries[..first_directory].iter().all(|entry| !entry.is_dir));
    assert!(entries[first_directory..].iter().all(|entry| entry.is_dir));
}
#[test]
fn normalized_lower_path_converts_separators_and_ascii_case() {
    assert_eq!(normalized_lower_path("SRC\\Components/Main.RS"), "src/components/main.rs");
}
#[test]
fn fuzzy_subsequence_score_handles_empty_missing_contiguous_and_gapped_queries() {
    assert_eq!(fuzzy_subsequence_score("src/main.rs", ""), Some(0));
    assert_eq!(fuzzy_subsequence_score("src/main.rs", "xyz"), None);

    let contiguous = fuzzy_subsequence_score("file_search.rs", "file").expect("contiguous match");
    let gapped = fuzzy_subsequence_score("f_i_l_e.rs", "file").expect("gapped match");
    assert!(contiguous > gapped);
}
#[test]
fn file_match_score_rewards_exact_filename_and_rejects_non_subsequences() {
    let exact = file_match_score("src/main.rs", "main.rs").expect("exact filename match");
    let partial = file_match_score("src/domain.rs", "main").expect("partial match");

    assert!(exact > partial);
    assert_eq!(file_match_score("src/main.rs", "xyz"), None);
    assert_eq!(file_match_score("src/main.rs", ""), Some(0));
}
#[test]
fn file_search_character_policy_accepts_path_syntax_and_rejects_unsafe_punctuation() {
    for allowed in ['a', 'Z', '7', '中', '文', '/', '\\', '.', '_', '-', ':', '#'] {
        assert!(is_file_search_char(allowed), "expected {allowed:?} to be accepted");
    }
    for rejected in [' ', '@', '!', '?', ';', '\n'] {
        assert!(!is_file_search_char(rejected), "expected {rejected:?} to be rejected");
    }
}
#[test]
fn trailing_token_uses_last_unicode_whitespace_boundary() {
    assert_eq!(trailing_token("open src/main.rs"), "src/main.rs");
    assert_eq!(trailing_token("open\t@src\n@tests"), "@tests");
    assert_eq!(trailing_token("prefix\u{2003}@unicode"), "@unicode");
    assert_eq!(trailing_token("ends with space "), "");
}
#[test]
fn active_file_search_query_accepts_supported_suffixes_and_empty_query() {
    assert_eq!(active_file_search_query("open @src/main.rs:12#L"), Some("src/main.rs:12#L"));
    assert_eq!(active_file_search_query("@"), Some(""));
    assert_eq!(active_file_search_query("prefix@src"), Some("src"));
}
#[test]
fn active_file_search_query_rejects_absent_or_invalid_mentions() {
    assert_eq!(active_file_search_query("plain text"), None);
    assert_eq!(active_file_search_query("open @src/main.rs!"), None);
    assert_eq!(active_file_search_query("old @valid next invalid?"), None);
}
#[test]
fn replace_or_append_file_mention_replaces_only_active_valid_suffix() {
    assert_eq!(
        replace_or_append_file_mention("compare @old then @sr", "@src/main.rs"),
        "compare @old then @src/main.rs "
    );
    assert_eq!(
        replace_or_append_file_mention("keep @invalid!", "@src/main.rs"),
        "keep @invalid! @src/main.rs "
    );
}
#[test]
fn replace_or_append_file_mention_handles_empty_and_existing_space() {
    assert_eq!(replace_or_append_file_mention("", "@README.md"), "@README.md ");
    assert_eq!(replace_or_append_file_mention("review ", "@README.md"), "review @README.md ");
}
#[test]
fn format_drop_mentions_keeps_single_file_position() {
    let mentions = format_drop_mentions(
        Some("/workspace"),
        &[DroppedPath { path: "/workspace/src/main.rs".to_string(), is_dir: false }],
        Some((12, 4)),
    );

    assert_eq!(mentions, vec!["src/main.rs:12:4".to_string()]);
}
#[test]
fn format_drop_mentions_ignores_position_for_multiple_paths() {
    let mentions = format_drop_mentions(
        Some("/workspace"),
        &[
            DroppedPath { path: "/workspace/src/main.rs".to_string(), is_dir: false },
            DroppedPath { path: "/workspace/src/lib.rs".to_string(), is_dir: false },
        ],
        Some((12, 4)),
    );

    assert_eq!(mentions, vec!["src/main.rs".to_string(), "src/lib.rs".to_string()]);
}
#[test]
fn format_drop_mentions_normalizes_unrooted_paths_and_ignores_directory_position() {
    let mentions = format_drop_mentions(
        None,
        &[
            DroppedPath { path: "external\\main.rs".to_string(), is_dir: false },
            DroppedPath { path: "external\\assets".to_string(), is_dir: true },
        ],
        Some((5, 6)),
    );

    assert_eq!(mentions, vec!["external/main.rs".to_string(), "external/assets/".to_string()]);
}
#[test]
fn format_drop_mentions_returns_empty_for_empty_drop() {
    assert!(format_drop_mentions(Some("/workspace"), &[], Some((1, 2))).is_empty());
}
#[test]
fn chinese_file_query_replaces_active_mention() {
    assert_eq!(active_file_search_query("查看 @原始视"), Some("原始视"));
    assert_eq!(
        replace_or_append_file_mention("查看 @原始视", "@原始视频.mov"),
        "查看 @原始视频.mov "
    );
}