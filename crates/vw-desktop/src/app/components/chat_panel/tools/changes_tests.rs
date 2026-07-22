use super::{parse_changes_file_summaries, parse_changes_files};

#[test]
fn parse_changes_files_skips_invalid_entries_and_defaults_optional_fields() {
    let output = concat!(
        "<changes>\n",
        r#"{"files":[null,{"path":42},{"path":"src/defaults.rs"}]}"#,
        "\n</changes>",
    );

    let files = parse_changes_files(output);

    assert_eq!(files.len(), 1);
    assert_eq!(files[0].path, "src/defaults.rs");
    assert_eq!(files[0].additions, 0);
    assert_eq!(files[0].deletions, 0);
    assert!(files[0].before.is_empty());
    assert!(files[0].after.is_empty());
}

#[test]
fn parse_changes_file_summaries_rejects_invalid_blocks_and_skips_missing_paths() {
    for output in [
        "no changes block",
        "<changes>\nnot-json\n</changes>",
        "<changes>\n{\"files\":{}}\n</changes>",
    ] {
        assert!(parse_changes_file_summaries(output).is_empty());
    }

    let summaries = parse_changes_file_summaries(
        "<changes>\n{\"files\":[{}, {\"path\":\"src/defaults.rs\"}]}\n</changes>",
    );
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].kind, 'M');
    assert_eq!(summaries[0].additions, 0);
    assert_eq!(summaries[0].deletions, 0);
}
