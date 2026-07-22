use super::*;
use crate::app::agent::tools::traits::Tool;
use serde_json::json;

#[test]
fn score_spec_covers_exact_name_alias_description_and_miss() {
    let spec = ToolSpec::new("file_read", "Read project files", json!({}))
        .with_display_name("Read")
        .with_aliases(vec!["open_file"]);

    assert_eq!(default_limit(), 10);
    assert_eq!(score_spec(&spec, "file_read").unwrap().score, 100);
    assert_eq!(score_spec(&spec, "read").unwrap().score, 100);
    assert_eq!(score_spec(&spec, "open").unwrap().score, 70);
    assert_eq!(score_spec(&spec, "project").unwrap().score, 50);
    assert!(score_spec(&spec, "missing").is_none());
}

#[test]
fn score_spec_partial_name_returns_name_match() {
    let spec = ToolSpec::new("file_read", "Read project files", json!({}));

    let name_match = score_spec(&spec, "file_").unwrap();

    assert_eq!(name_match.score, 80);
    assert_eq!(name_match.reason, "name match");
}

#[test]
fn spec_is_read_only_strict_and_has_query_schema() {
    let spec = ToolSearchTool::new().spec();

    assert_eq!(spec.display_name, "ToolSearch");
    assert!(spec.read_only);
    assert!(spec.concurrency_safe);
    assert!(spec.strict);
    assert!(spec.input_schema["required"].as_array().unwrap().contains(&json!("query")));
}
