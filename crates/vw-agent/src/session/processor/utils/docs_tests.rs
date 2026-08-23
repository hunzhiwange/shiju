//! 验证文档列表请求识别、目录遍历边界与稳定排序行为。

#[test]
fn is_docs_request_requires_docs_list_and_read_intent() {
    assert!(super::is_docs_request("请读取 docs 文件列表"));
    assert!(super::is_docs_request("DOCS 请查看文件 列表"));
    assert!(super::is_docs_request("帮我列出 docs 列表"));

    assert!(!super::is_docs_request("请读取文件列表"));
    assert!(!super::is_docs_request("docs"));
    assert!(!super::is_docs_request("docs 文件列表"));
    assert!(!super::is_docs_request("请读取 docs"));
}

#[test]
fn is_docs_request_accepts_each_supported_list_and_read_keyword_pair() {
    for list_keyword in ["文件列表", "文件 列表", "列表"] {
        for read_keyword in ["读取", "查看", "列出"] {
            let query = format!("请{read_keyword} docs {list_keyword}");
            assert!(super::is_docs_request(&query), "query should match: {query}");
        }
    }
}

#[test]
fn list_docs_returns_error_when_docs_directory_is_missing() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let root = workspace.path().to_string_lossy().to_string();

    assert_eq!(super::list_docs(Some(&root)), Err("docs 目录不存在".to_string()));
}

#[test]
fn list_docs_returns_empty_when_docs_directory_is_empty() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::create_dir(workspace.path().join("docs")).expect("docs dir");
    let root = workspace.path().to_string_lossy().to_string();

    assert_eq!(super::list_docs(Some(&root)), Ok(Vec::new()));
}

#[test]
fn list_docs_returns_empty_when_docs_path_is_a_file() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    std::fs::write(workspace.path().join("docs"), "not a directory").expect("docs file");
    let root = workspace.path().to_string_lossy().to_string();

    assert_eq!(super::list_docs(Some(&root)), Ok(Vec::new()));
}

#[test]
fn list_docs_walks_visible_files_skips_noise_and_sorts() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let docs = workspace.path().join("docs");
    std::fs::create_dir_all(docs.join("guide")).expect("guide dir");
    std::fs::create_dir_all(docs.join(".hidden")).expect("hidden dir");
    std::fs::create_dir_all(docs.join("node_modules/pkg")).expect("node_modules dir");
    std::fs::create_dir_all(docs.join("target/debug")).expect("target dir");
    std::fs::write(docs.join("zeta.md"), "z").expect("zeta");
    std::fs::write(docs.join("guide/alpha.md"), "a").expect("alpha");
    std::fs::write(docs.join(".hidden/secret.md"), "secret").expect("secret");
    std::fs::write(docs.join("node_modules/pkg/readme.md"), "noise").expect("noise");
    std::fs::write(docs.join("target/debug/out.md"), "noise").expect("target noise");

    let root = workspace.path().to_string_lossy().to_string();
    let docs = super::list_docs(Some(&root)).expect("docs should list");

    assert_eq!(docs, vec!["docs/guide/alpha.md", "docs/zeta.md"]);
}

#[test]
fn list_docs_skips_reserved_names_when_they_are_files() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let docs = workspace.path().join("docs");
    std::fs::create_dir(&docs).expect("docs dir");
    std::fs::write(docs.join("target"), "reserved").expect("target file");
    std::fs::write(docs.join("node_modules"), "reserved").expect("node_modules file");
    std::fs::write(docs.join("visible.md"), "visible").expect("visible file");
    let root = workspace.path().to_string_lossy().to_string();

    assert_eq!(super::list_docs(Some(&root)), Ok(vec!["docs/visible.md".to_string()]));
}

#[test]
fn list_docs_uses_current_dir_when_root_is_absent() {
    let cwd = std::env::current_dir().expect("current dir");
    let result = super::list_docs(None);

    if cwd.join("docs").exists() {
        let docs = result.expect("docs via current dir");
        assert!(docs.iter().all(|path| path.starts_with("docs/")));
    } else {
        assert_eq!(result, Err("docs 目录不存在".to_string()));
    }
}

#[test]
fn walk_docs_stops_after_depth_limit() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let docs = workspace.path().join("docs");
    let mut dir = docs.clone();
    std::fs::create_dir_all(&dir).expect("docs dir");
    for idx in 0..12 {
        dir = dir.join(format!("level-{idx}"));
        std::fs::create_dir_all(&dir).expect("nested dir");
    }
    std::fs::write(dir.join("visible-at-depth-12.md"), "visible").expect("visible doc");
    let too_deep = dir.join("level-12");
    std::fs::create_dir_all(&too_deep).expect("too deep dir");
    std::fs::write(too_deep.join("hidden-at-depth-13.md"), "hidden").expect("hidden doc");

    let mut out = Vec::new();
    super::walk_docs(workspace.path(), &docs, &mut out, 0);

    assert!(out.iter().any(|path| path.ends_with("visible-at-depth-12.md")));
    assert!(!out.iter().any(|path| path.ends_with("hidden-at-depth-13.md")));
}

#[test]
fn walk_docs_uses_absolute_path_when_base_is_not_an_ancestor() {
    let workspace = tempfile::tempdir().expect("temp workspace");
    let unrelated_base = tempfile::tempdir().expect("unrelated base");
    let docs = workspace.path().join("docs");
    std::fs::create_dir(&docs).expect("docs dir");
    let document = docs.join("guide.md");
    std::fs::write(&document, "guide").expect("guide file");
    let expected = document.to_string_lossy().replace('\\', "/");
    let mut out = Vec::new();

    super::walk_docs(unrelated_base.path(), &docs, &mut out, 0);

    assert_eq!(out, vec![expected]);
}
