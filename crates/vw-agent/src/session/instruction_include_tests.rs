use super::instruction_include;
use crate::app::agent::security::SecurityPolicy;
use std::path::Path;

#[test]
fn agents_include_expands_safe_text_before_parent_without_cycles() {
    let workspace = tempfile::tempdir().expect("workspace");
    let agents = workspace.path().join("AGENTS.md");
    std::fs::write(
        &agents,
        "parent start\n@include.md\n@missing.md\n```markdown\n@ignored.md\n```\nparent end\n",
    )
    .expect("write AGENTS");
    std::fs::write(workspace.path().join("include.md"), "included\n@child.txt\n@AGENTS.md\n")
        .expect("write included file");
    std::fs::write(workspace.path().join("child.txt"), "child").expect("write child file");
    std::fs::write(workspace.path().join("ignored.md"), "must not load")
        .expect("write ignored file");
    std::fs::write(workspace.path().join("binary.png"), [0_u8, 159, 146, 150])
        .expect("write binary file");

    let policy =
        SecurityPolicy { workspace_dir: workspace.path().to_path_buf(), ..Default::default() };
    let expanded = instruction_include::expand(&agents, &policy);

    let child = expanded.content.find("child").expect("child included");
    let included = expanded.content.find("included").expect("include inserted");
    let parent = expanded.content.find("parent start").expect("parent retained");
    assert!(child < included && included < parent);
    assert!(expanded.content.contains("@ignored.md"));
    assert!(!expanded.content.contains("must not load"));
    assert!(expanded.diagnostics.iter().any(|diagnostic| diagnostic.contains("cycle")));
    assert!(
        expanded.diagnostics.iter().any(|diagnostic| diagnostic.contains("missing or not a file"))
    );
    assert!(expanded.sources.iter().any(|source| {
        source.filepath.ends_with("child.txt")
            && source.parent.as_deref().is_some_and(|parent| parent.ends_with("include.md"))
    }));

    let binary = instruction_include::expand(&workspace.path().join("binary.png"), &policy);
    assert!(binary.content.is_empty());
    assert!(
        binary.diagnostics.iter().any(|diagnostic| diagnostic.contains("unsupported text type"))
    );

    let outside = tempfile::tempdir().expect("external directory");
    std::fs::write(outside.path().join("external.md"), "external").expect("write external file");
    std::fs::write(&agents, format!("@{}", outside.path().join("external.md").display()))
        .expect("replace AGENTS");
    let denied = instruction_include::expand(&agents, &policy);
    assert!(denied.content.contains("@"));
    assert!(
        denied
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.contains("escapes workspace allowlist"))
    );
}
