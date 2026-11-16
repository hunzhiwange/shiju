//! 验证文件链接块的解析、压缩、路径解析与元数据构造行为。
//! 测试将展示路径与真实绝对路径分开校验，避免链接压缩掩盖路径逃逸。

use crate::app::agent::tools::ToolRuntimeContext;

fn ctx_for(path: &std::path::Path) -> ToolRuntimeContext {
    ToolRuntimeContext::new("file-link-tests", Some(path.to_string_lossy().to_string()))
}

#[test]
fn resolve_full_path_uses_absolute_root_or_relative_path() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let ctx = ctx_for(workspace.path());
    let absolute = workspace.path().join("already.txt");

    assert_eq!(super::resolve_full_path(&ctx, absolute.to_str().unwrap()), absolute);
    assert_eq!(super::resolve_full_path(&ctx, "docs/a.md"), workspace.path().join("docs/a.md"));

    let no_root = ToolRuntimeContext::new("file-link-no-root", None);
    assert_eq!(
        super::resolve_full_path(&no_root, "relative.md"),
        std::path::PathBuf::from("relative.md")
    );
}

#[test]
fn resolve_full_path_trims_relative_input_before_joining_root() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let ctx = ctx_for(workspace.path());

    assert_eq!(
        super::resolve_full_path(&ctx, "  docs/trimmed.md  "),
        workspace.path().join("docs/trimmed.md")
    );
}

#[test]
fn build_file_link_reports_relative_path_open_url_and_size() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let docs = workspace.path().join("docs");
    std::fs::create_dir_all(&docs).expect("docs dir");
    std::fs::write(docs.join("a.md"), "hello").expect("doc");
    let ctx = ctx_for(workspace.path());

    let link = super::build_file_link(&ctx, "docs/a.md").expect("link");

    assert!(link.contains("<file_link>"));
    assert!(link.contains("path: docs/a.md"));
    assert!(link.contains("open: file:///"));
    assert!(link.contains("size_bytes: 5"));
}

#[test]
fn build_file_link_reports_zero_size_for_missing_relative_file() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let ctx = ctx_for(workspace.path());
    let missing = workspace.path().join("missing.md");

    let link = super::build_file_link(&ctx, "missing.md").expect("link");

    assert!(link.contains("path: missing.md"));
    assert!(link.contains(&format!("open: file:///{}", missing.to_string_lossy())));
    assert!(link.contains("size_bytes: 0"));
}

#[test]
fn build_file_link_without_root_reports_open_url_and_size() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let file = workspace.path().join("absolute.md");
    std::fs::write(&file, "content").expect("doc");
    let ctx = ToolRuntimeContext::new("file-link-no-root", None);

    let link = super::build_file_link(&ctx, file.to_str().expect("utf-8 path")).expect("link");

    assert!(link.contains(&format!("open: file:///{}", file.to_string_lossy())));
    assert!(link.contains("size_bytes: 7"));
}

#[test]
fn build_file_link_keeps_absolute_display_path_outside_root() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let outside = tempfile::tempdir().expect("outside workspace");
    let file = outside.path().join("outside.md");
    std::fs::write(&file, "outside").expect("doc");
    let ctx = ctx_for(workspace.path());

    let link = super::build_file_link(&ctx, file.to_str().expect("utf-8 path")).expect("link");
    let display_path = file.to_string_lossy().replace('\\', "/");

    assert!(link.contains(&format!("path: {display_path}")));
    assert!(link.contains("size_bytes: 7"));
}

#[test]
// 自动注入仅适用于文件类工具，已有链接时保持原输出以避免重复展示入口。
fn maybe_inject_file_link_respects_eligibility_and_existing_links() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("a.md"), "hello").expect("doc");
    let ctx = ctx_for(workspace.path());

    let injected = super::maybe_inject_file_link("read", r#"{"path":"a.md"}"#, &ctx, "body");
    assert!(injected.starts_with("<file_link>"));
    assert!(injected.ends_with("body"));

    let link_only = super::maybe_inject_file_link("read", r#"{"path":"a.md"}"#, &ctx, "   ");
    assert!(link_only.contains("path: a.md"));
    assert!(!link_only.contains("body"));

    let existing = "<file_link>\npath: old\n</file_link>\nbody";
    assert_eq!(super::maybe_inject_file_link("read", "a.md", &ctx, existing), existing);
    assert_eq!(super::maybe_inject_file_link("bash", "a.md", &ctx, "body"), "body");
    assert_eq!(super::maybe_inject_file_link("read", "   ", &ctx, "body"), "body");
}

#[test]
fn maybe_inject_file_link_supports_every_file_tool_alias() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let ctx = ctx_for(workspace.path());

    for name in ["read", "file_read", "pdf_read", "write", "file_write", "file_edit", "apply_patch"]
    {
        let output =
            super::maybe_inject_file_link(name, r#"{"filePath":"missing.md"}"#, &ctx, "body");

        assert!(output.starts_with("<file_link>"), "tool alias: {name}");
        assert!(output.contains("path: missing.md"), "tool alias: {name}");
        assert!(output.ends_with("body"), "tool alias: {name}");
    }
}
