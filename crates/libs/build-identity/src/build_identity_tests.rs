//! 可执行文件构建身份的确定性测试。
//!
//! 使用固定字节夹具核对摘要格式与 SHA-256 标准结果。

use super::build_identity::executable_build_id;

#[test]
fn executable_build_id_returns_sha256_of_file_bytes() {
    let temp = tempfile::tempdir().expect("create tempdir");
    let executable = temp.path().join("vibewindow");
    std::fs::write(&executable, b"abc").expect("write executable fixture");

    assert_eq!(
        executable_build_id(executable).expect("hash executable"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
