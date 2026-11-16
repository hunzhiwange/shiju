#[test]
fn extract_file_link_blocks_collects_complete_blocks_only() {
    let text = concat!(
        "before\n",
        "<file_link>\npath: a.md\n</file_link>\n",
        "middle\n",
        "<file_link>\npath: b.md\n</file_link>\n",
        "<file_link>\npath: missing-end\n"
    );

    let blocks = super::extract_file_link_blocks(text);

    assert_eq!(blocks.len(), 2);
    assert!(blocks[0].contains("path: a.md"));
    assert!(blocks[1].contains("path: b.md"));
}

#[test]
fn extract_file_link_blocks_handles_absent_unclosed_and_terminal_blocks() {
    assert!(super::extract_file_link_blocks("plain output").is_empty());
    assert!(super::extract_file_link_blocks("<file_link>\npath: open.md").is_empty());

    let terminal = "<file_link>\npath: terminal.md\n</file_link>";
    assert_eq!(super::extract_file_link_blocks(terminal), vec![terminal.to_string()]);
}

#[test]
fn compact_file_link_replaces_block_with_path_line() {
    let text = concat!(
        "prefix\n",
        "<file_link>\n",
        "path: docs/readme.md\n",
        "open: file:///tmp/readme.md\n",
        "size_bytes: 12\n",
        "</file_link>\n",
        "suffix"
    );

    let compacted = super::compact_file_link(text);

    assert_eq!(compacted, "prefix\npath: docs/readme.md\n\nsuffix");
}

#[test]
fn compact_file_link_keeps_unparseable_input_usable() {
    assert_eq!(super::compact_file_link("plain output"), "plain output");
    assert_eq!(
        super::compact_file_link("<file_link>\nopen: file:///tmp/a\n</file_link>\nbody"),
        "body"
    );
    assert_eq!(
        super::compact_file_link("before <file_link>\npath: a.md"),
        "before <file_link>\npath: a.md"
    );
}

#[test]
fn compact_file_link_normalizes_path_and_leaves_later_blocks_intact() {
    let second = concat!(
        "<file_link>\n",
        "path: docs/second.md\n",
        "open: file:///tmp/second.md\n",
        "</file_link>"
    );
    let text = format!(
        "head\n<file_link>\n  path:   docs/first.md  \nopen: file:///tmp/first.md\n</file_link>\nbetween\n{second}\ntail"
    );

    let compacted = super::compact_file_link(&text);

    assert_eq!(compacted, format!("head\npath: docs/first.md\n\nbetween\n{second}\ntail"));
}

#[test]
fn extract_file_path_from_input_accepts_json_aliases_and_plain_references() {
    assert_eq!(
        super::extract_file_path_from_input(r#"{"filePath":"src/main.rs"}"#),
        Some("src/main.rs".to_string())
    );
    assert_eq!(
        super::extract_file_path_from_input(r#"{"file_path":"src/lib.rs"}"#),
        Some("src/lib.rs".to_string())
    );
    assert_eq!(
        super::extract_file_path_from_input(r#"{"path":"docs/readme.md"}"#),
        Some("docs/readme.md".to_string())
    );
    assert_eq!(
        super::extract_file_path_from_input("[README](docs/readme.md#L12)"),
        Some("docs/readme.md".to_string())
    );
    assert_eq!(
        super::extract_file_path_from_input("`file:///tmp/demo.txt#line-4`"),
        Some("tmp/demo.txt".to_string())
    );
    assert_eq!(super::extract_file_path_from_input("   "), None);
    assert_eq!(super::extract_file_path_from_input(r#"{"path":42}"#), None);
    assert_eq!(super::extract_file_path_from_input("{not json}"), None);
}

#[test]
fn extract_file_path_from_input_normalizes_wrappers_fragments_and_empty_values() {
    assert_eq!(
        super::extract_file_path_from_input(r#"  "docs/guide.md#L7"  "#),
        Some("docs/guide.md".to_string())
    );
    assert_eq!(
        super::extract_file_path_from_input("'file://relative/note.txt#line-9'"),
        Some("relative/note.txt".to_string())
    );
    assert_eq!(super::extract_file_path_from_input("``"), None);
    assert_eq!(super::extract_file_path_from_input(r#"{"path":" ` ` "}"#), None);
    assert_eq!(super::extract_file_path_from_input("#L12"), None);
    assert_eq!(super::extract_file_path_from_input("{}"), None);
}
