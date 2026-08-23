// 验证身份文件加载、缺省回退与无效内容的错误报告。

// 相对路径以工作区为基准，绝对路径优先于内联来源，读取错误保留路径上下文。
use super::*;
use crate::app::agent::config::IdentityConfig;

fn config(format: &str, path: Option<&str>, inline: Option<&str>) -> IdentityConfig {
    IdentityConfig {
        format: format.to_string(),
        aieos_path: path.map(ToString::to_string),
        aieos_inline: inline.map(ToString::to_string),
    }
}

#[test]
fn is_aieos_configured_requires_aieos_format_and_source() {
    assert!(is_aieos_configured(&config("aieos", Some("identity.json"), None)));
    assert!(!is_aieos_configured(&config("local", Some("identity.json"), None)));
    assert!(!is_aieos_configured(&config("aieos", None, None)));
}

#[test]
fn load_ignores_non_aieos_format() {
    let temp = tempfile::tempdir().expect("temp dir");
    let loaded = load_aieos_identity(
        &config("openclaw", Some("missing.json"), Some("{not-json")),
        temp.path(),
    )
    .expect("non-aieos format should not load");

    assert!(loaded.is_none());
}

#[test]
fn load_reads_relative_file_from_workspace() {
    let temp = tempfile::tempdir().expect("temp dir");
    std::fs::write(
        temp.path().join("identity.json"),
        r#"{"identity":{"names":{"first":"Ada","last":"Lovelace"}}}"#,
    )
    .expect("write identity");

    let loaded = load_aieos_identity(&config("aieos", Some("identity.json"), None), temp.path())
        .expect("load identity")
        .expect("identity");
    let names = loaded.identity.expect("identity section").names.expect("names");

    assert_eq!(names.first.as_deref(), Some("Ada"));
    assert_eq!(names.full.as_deref(), Some("Ada Lovelace"));
}

#[test]
fn load_reads_absolute_file_before_inline() {
    let temp = tempfile::tempdir().expect("temp dir");
    let path = temp.path().join("identity.json");
    std::fs::write(&path, r#"{"identity":{"names":{"full":"File Identity"}}}"#)
        .expect("write identity");

    let loaded = load_aieos_identity(
        &config("aieos", Some(path.to_str().expect("utf8 path")), Some("{not-json")),
        temp.path(),
    )
    .expect("load identity")
    .expect("identity");

    assert_eq!(loaded.identity.unwrap().names.unwrap().full.as_deref(), Some("File Identity"));
}

#[test]
fn load_parses_inline_identity_when_path_is_missing() {
    let temp = tempfile::tempdir().expect("temp dir");

    let loaded = load_aieos_identity(
        &config("aieos", None, Some(r#"{"identity":{"names":{"nickname":"Inline"}}}"#)),
        temp.path(),
    )
    .expect("load inline")
    .expect("identity");

    assert_eq!(loaded.identity.unwrap().names.unwrap().nickname.as_deref(), Some("Inline"));
}

#[test]
fn load_errors_when_aieos_source_is_missing() {
    let temp = tempfile::tempdir().expect("temp dir");
    let error =
        load_aieos_identity(&config("aieos", None, None), temp.path()).expect_err("missing source");

    assert!(error.to_string().contains("未配置 aieos_path 或 aieos_inline"));
}

#[test]
fn load_file_read_error_includes_path_context() {
    let temp = tempfile::tempdir().expect("temp dir");
    let error = load_aieos_identity(&config("aieos", Some("missing.json"), None), temp.path())
        .expect_err("missing file");
    let rendered = format!("{error:#}");

    assert!(rendered.contains("读取 AIEOS 文件失败"));
    assert!(rendered.contains("missing.json"));
}

#[test]
fn load_file_parse_error_includes_path_context() {
    let temp = tempfile::tempdir().expect("temp dir");
    std::fs::write(temp.path().join("identity.json"), "[]").expect("write invalid identity");

    let error = load_aieos_identity(&config("aieos", Some("identity.json"), None), temp.path())
        .expect_err("invalid file payload");
    let rendered = format!("{error:#}");

    assert!(rendered.contains("解析 AIEOS JSON 失败"));
    assert!(rendered.contains("identity.json"));
    assert!(rendered.contains("JSON 对象"));
}

#[test]
fn load_inline_parse_error_includes_context() {
    let temp = tempfile::tempdir().expect("temp dir");
    let error = load_aieos_identity(&config("aieos", None, Some("not-json")), temp.path())
        .expect_err("invalid inline payload");
    let rendered = format!("{error:#}");

    assert!(rendered.contains("解析内联 AIEOS JSON 失败"));
    assert!(rendered.contains("无效的 AIEOS JSON"));
}

#[test]
fn parse_aieos_identity_rejects_non_object_payload() {
    let error = parse_aieos_identity("[]").expect_err("array is not a valid identity object");

    assert!(error.to_string().contains("JSON 对象"));
}

#[test]
fn parse_aieos_identity_rejects_invalid_json() {
    let error = parse_aieos_identity("not-json").expect_err("invalid JSON");

    assert!(format!("{error:#}").contains("无效的 AIEOS JSON"));
}
