//! 验证压缩包提取对不存在输入返回明确错误。
//! 该失败契约确保调用方不会把缺失归档误判为空提取结果。

use super::archive::extract_zip;

#[test]
fn extract_zip_reports_missing_archive() {
    let dir = tempfile::tempdir().expect("temp dir");
    let err = extract_zip(dir.path().join("missing.zip"), dir.path()).expect_err("missing zip");

    assert!(!err.to_string().is_empty());
}
