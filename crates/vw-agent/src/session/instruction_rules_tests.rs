use super::instruction_rules;

#[test]
fn agents_rules_activate_only_for_matching_paths() {
    let workspace = tempfile::tempdir().expect("workspace");
    let rule = workspace.path().join(".vibewindow/rules/rust.md");
    std::fs::create_dir_all(rule.parent().expect("rule parent")).expect("create rule directory");
    std::fs::write(
        &rule,
        "---\npaths:\n  - src/**/*.rs\n  - Cargo.toml\n---\nOnly use Rust rules.\n",
    )
    .expect("write conditional rule");

    let rust = instruction_rules::load_if_matches(
        &rule,
        workspace.path(),
        &workspace.path().join("src/lib.rs"),
    )
    .expect("rust target activates rule");
    assert_eq!(rust.conditions, vec!["src/**/*.rs", "Cargo.toml"]);
    assert_eq!(rust.content, "Only use Rust rules.\n");
    assert!(
        instruction_rules::load_if_matches(
            &rule,
            workspace.path(),
            &workspace.path().join("docs/readme.md")
        )
        .is_none()
    );

    std::fs::write(&rule, "---\npaths: docs/**\n---\nDocs only\n").expect("rewrite rule");
    assert!(
        instruction_rules::load_if_matches(
            &rule,
            workspace.path(),
            &workspace.path().join("src/lib.rs")
        )
        .is_none()
    );
    assert!(
        instruction_rules::load_if_matches(
            &rule,
            workspace.path(),
            &workspace.path().join("docs/readme.md")
        )
        .is_some()
    );
}
