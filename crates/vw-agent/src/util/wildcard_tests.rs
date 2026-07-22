use super::wildcard::{StructuredInput, all, all_structured, matches};

#[test]
fn wildcard_matches_strings_and_structured_inputs() {
    assert!(matches("hello world", "hello *"));
    assert!(matches("file.txt", "file.?xt"));
    assert_eq!(
        all("hello world", &[("hello*".to_string(), 1), ("hello world".to_string(), 2)]),
        Some(2)
    );
    let input = StructuredInput { head: "cmd".into(), tail: vec!["a".into(), "b".into()] };
    assert_eq!(all_structured(&input, &[("cmd a *".to_string(), 3)]), Some(3));
}

#[test]
fn structured_matching_skips_blank_and_nonmatching_patterns() {
    let input = StructuredInput { head: "deploy".into(), tail: vec!["prod".into()] };
    let patterns =
        [("   ".to_string(), 1), ("build *".to_string(), 2), ("deploy staging".to_string(), 3)];

    assert_eq!(all_structured(&input, &patterns), None);
}

#[test]
fn wildcard_matching_handles_empty_sets_and_literal_regex_characters() {
    assert!(matches("config[1].toml", "config[1].toml"));
    assert!(!matches("config11.toml", "config[1].toml"));
    assert_eq!(all::<usize>("anything", &[]), None);
}
