use super::verify_file;
use std::{
    fs::{self, Permissions},
    os::unix::fs::{PermissionsExt, symlink},
};

#[test]
fn acp_platform_codex_contract_verifies_native_artifact_bytes_and_path() {
    let root = tempfile::tempdir().unwrap();
    let root = root.path().canonicalize().unwrap();
    let executable = root.join("codex");
    let expected = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad";
    fs::write(&executable, b"abc").unwrap();
    fs::set_permissions(&executable, Permissions::from_mode(0o600)).unwrap();
    assert!(verify_file(&executable, expected, true).is_err());

    fs::set_permissions(&executable, Permissions::from_mode(0o700)).unwrap();
    verify_file(&executable, expected, true).unwrap();
    let redirected = root.join("redirected");
    symlink(&executable, &redirected).unwrap();
    assert!(verify_file(&redirected, expected, true).is_err());

    fs::write(&executable, b"changed").unwrap();
    assert!(verify_file(&executable, expected, true).is_err());
    fs::remove_file(&executable).unwrap();
    assert!(verify_file(&executable, expected, true).is_err());
}
