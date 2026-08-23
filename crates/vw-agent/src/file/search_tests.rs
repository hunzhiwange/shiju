// 覆盖模糊文件搜索的评分、隐藏路径与结果限制。

use super::SearchInput;
use super::search::{fuzzy_score, is_hidden, search};
use std::fs;

#[test]
fn fuzzy_score_prefers_substring_matches_and_filters_hidden_when_query_is_hidden() {
    assert_eq!(fuzzy_score("lib", "src/lib.rs"), 6);
    assert!(is_hidden("src/.secret/file"));
    assert_eq!(fuzzy_score(".secret", "src/visible.rs"), i64::MAX);
}

#[test]
fn fuzzy_score_handles_hidden_path_queries_and_levenshtein_fallback() {
    assert_eq!(fuzzy_score("src/.env", "src/env"), i64::MAX);
    assert!(fuzzy_score(".secret", "src/.secret/file") < i64::MAX);
    assert_eq!(fuzzy_score("abc", "xyz"), 3);
    assert!(is_hidden(r"src\.cache\file"));
    assert!(is_hidden("src/.cache/"));
    assert!(!is_hidden("src/cache/file"));
}

#[test]
fn fuzzy_score_is_case_insensitive_and_exact_matches_have_zero_cost() {
    assert_eq!(fuzzy_score("LIB", "src/lib.rs"), 6);
    assert_eq!(fuzzy_score("README", "readme"), 0);
}

#[test]
fn is_hidden_requires_a_multi_character_dot_component() {
    assert!(!is_hidden("."));
    assert!(is_hidden(".."));
    assert!(is_hidden(".env"));
    assert!(!is_hidden("src/dotted.name"));
}

#[test]
fn search_empty_query_defaults_to_files_and_truncates_sorted_results() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("dir")).expect("create dir");
    fs::write(temp.path().join("b.rs"), "b").expect("write b");
    fs::write(temp.path().join("a.rs"), "a").expect("write a");
    fs::write(temp.path().join("dir/nested.rs"), "nested").expect("write nested");

    let result = search(
        temp.path(),
        SearchInput { query: "  ".to_string(), limit: 2, dirs: false, r#type: None },
    );

    assert_eq!(result, vec!["a.rs".to_string(), "b.rs".to_string()]);
}

#[test]
fn search_can_return_directories_or_all_items() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::create_dir_all(temp.path().join("docs")).expect("create docs");
    fs::write(temp.path().join("src/main.rs"), "main").expect("write main");
    fs::write(temp.path().join("docs/readme.md"), "docs").expect("write docs");

    let dirs = search(
        temp.path(),
        SearchInput {
            query: "".to_string(),
            limit: 10,
            dirs: false,
            r#type: Some("directory".to_string()),
        },
    );
    let all_from_dirs_flag = search(
        temp.path(),
        SearchInput { query: "".to_string(), limit: 10, dirs: true, r#type: None },
    );
    let all_from_unknown_type = search(
        temp.path(),
        SearchInput {
            query: "".to_string(),
            limit: 10,
            dirs: false,
            r#type: Some("unknown".to_string()),
        },
    );

    assert_eq!(dirs, vec!["docs/".to_string(), "src/".to_string()]);
    assert_eq!(
        all_from_dirs_flag,
        vec![
            "docs/".to_string(),
            "docs/readme.md".to_string(),
            "src/".to_string(),
            "src/main.rs".to_string(),
        ]
    );
    assert_eq!(all_from_unknown_type, all_from_dirs_flag);
}

#[test]
fn search_trims_query_scores_results_and_filters_visible_files_for_hidden_queries() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src")).expect("create src");
    fs::create_dir_all(temp.path().join(".secret")).expect("create secret");
    fs::write(temp.path().join("src/lib.rs"), "lib").expect("write lib");
    fs::write(temp.path().join("src/main.rs"), "main").expect("write main");
    fs::write(temp.path().join(".secret/token"), "token").expect("write token");
    fs::write(temp.path().join("visible.txt"), "visible").expect("write visible");

    let best_match = search(
        temp.path(),
        SearchInput { query: " lib ".to_string(), limit: 1, dirs: false, r#type: None },
    );
    let hidden_match = search(
        temp.path(),
        SearchInput { query: ".secret".to_string(), limit: 10, dirs: false, r#type: None },
    );

    assert_eq!(best_match, vec!["src/lib.rs".to_string()]);
    assert_eq!(hidden_match, vec![".secret/token".to_string()]);
}

#[test]
fn search_hidden_directory_query_keeps_hidden_dirs_and_filters_visible_dirs() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join(".secret/nested")).expect("create hidden dirs");
    fs::create_dir_all(temp.path().join("secret/nested")).expect("create visible dirs");
    fs::write(temp.path().join(".secret/nested/data.txt"), "hidden").expect("write hidden");
    fs::write(temp.path().join("secret/nested/data.txt"), "visible").expect("write visible");

    let result = search(
        temp.path(),
        SearchInput {
            query: ".secret".to_string(),
            limit: 10,
            dirs: false,
            r#type: Some("directory".to_string()),
        },
    );

    assert_eq!(result, vec![".secret/".to_string(), ".secret/nested/".to_string()]);
}

#[test]
fn search_zero_limit_applies_to_empty_and_scored_queries() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("alpha.txt"), "alpha").expect("write alpha");

    let empty_query = search(
        temp.path(),
        SearchInput { query: "".to_string(), limit: 0, dirs: false, r#type: None },
    );
    let scored_query = search(
        temp.path(),
        SearchInput { query: "alpha".to_string(), limit: 0, dirs: false, r#type: None },
    );

    assert!(empty_query.is_empty());
    assert!(scored_query.is_empty());
}

#[test]
fn search_breaks_equal_score_ties_by_path() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::write(temp.path().join("bq.txt"), "b").expect("write b");
    fs::write(temp.path().join("aq.txt"), "a").expect("write a");

    let result = search(
        temp.path(),
        SearchInput { query: "q".to_string(), limit: 10, dirs: false, r#type: None },
    );

    assert_eq!(result, vec!["aq.txt".to_string(), "bq.txt".to_string()]);
}

#[test]
fn search_nested_directories_are_deduplicated_and_searchable() {
    let temp = tempfile::tempdir().expect("tempdir");
    fs::create_dir_all(temp.path().join("src/nested")).expect("create nested");
    fs::create_dir_all(temp.path().join("src/other")).expect("create other");
    fs::write(temp.path().join("src/nested/a.rs"), "a").expect("write nested a");
    fs::write(temp.path().join("src/nested/b.rs"), "b").expect("write nested b");
    fs::write(temp.path().join("src/other/c.rs"), "c").expect("write other c");

    let all_dirs = search(
        temp.path(),
        SearchInput {
            query: "".to_string(),
            limit: 10,
            dirs: false,
            r#type: Some("directory".to_string()),
        },
    );
    let best_match = search(
        temp.path(),
        SearchInput {
            query: "nested".to_string(),
            limit: 1,
            dirs: false,
            r#type: Some("directory".to_string()),
        },
    );

    assert_eq!(
        all_dirs,
        vec!["src/".to_string(), "src/nested/".to_string(), "src/other/".to_string()]
    );
    assert_eq!(best_match, vec!["src/nested/".to_string()]);
}

#[test]
fn search_file_root_returns_only_the_empty_relative_file_entry() {
    let temp = tempfile::tempdir().expect("tempdir");
    let file_root = temp.path().join("single.txt");
    fs::write(&file_root, "single").expect("write single");

    let files = search(
        &file_root,
        SearchInput {
            query: "".to_string(),
            limit: 10,
            dirs: false,
            r#type: Some("file".to_string()),
        },
    );
    let dirs = search(
        &file_root,
        SearchInput {
            query: "".to_string(),
            limit: 10,
            dirs: false,
            r#type: Some("directory".to_string()),
        },
    );

    assert_eq!(files, vec![String::new()]);
    assert!(dirs.is_empty());
}

#[test]
fn search_missing_root_returns_empty_results() {
    let temp = tempfile::tempdir().expect("tempdir");

    let result = search(
        temp.path().join("missing"),
        SearchInput { query: "".to_string(), limit: 10, dirs: true, r#type: None },
    );

    assert!(result.is_empty());
}
