//! 验证压缩包提取对不存在输入返回明确错误。

use super::archive::extract_zip;

#[test]
fn extract_zip_reports_missing_archive() {
    let dir = tempfile::tempdir().expect("temp dir");
    let err = extract_zip(dir.path().join("missing.zip"), dir.path()).expect_err("missing zip");

    assert!(!err.to_string().is_empty());
}
