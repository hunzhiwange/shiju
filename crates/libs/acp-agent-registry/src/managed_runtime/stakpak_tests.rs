use super::{CliContract, LINUX_CLI_CONTRACT, cli_local_command};
use std::{
    fs,
    os::unix::fs::{PermissionsExt, symlink},
};

#[test]
fn acp_platform_stakpak_contract_rejects_invalid_installed_artifacts() {
    let root = tempfile::tempdir().unwrap();
    assert!(cli_local_command(root.path()).unwrap().is_none());

    let contract: CliContract = serde_json::from_str(LINUX_CLI_CONTRACT).unwrap();
    let directory = root.path().join(contract.directory);
    let redirected = root.path().join("redirected");
    fs::create_dir(&redirected).unwrap();
    symlink(&redirected, &directory).unwrap();
    assert_eq!(
        cli_local_command(root.path()).unwrap_err(),
        "Stakpak installation directory is invalid"
    );
    fs::remove_file(&directory).unwrap();
    fs::create_dir(&directory).unwrap();

    let entry = directory.join(contract.entry);
    assert_eq!(cli_local_command(root.path()).unwrap_err(), "Missing Stakpak Linux CLI");
    let foreign = redirected.join("stakpak");
    fs::write(&foreign, b"unreviewed executable").unwrap();
    fs::set_permissions(&foreign, fs::Permissions::from_mode(0o700)).unwrap();
    symlink(&foreign, &entry).unwrap();
    assert_eq!(
        cli_local_command(root.path()).unwrap_err(),
        "Stakpak Linux CLI must be a regular executable"
    );
    fs::remove_file(&entry).unwrap();
    fs::copy(&foreign, &entry).unwrap();
    fs::set_permissions(&entry, fs::Permissions::from_mode(0o600)).unwrap();
    assert_eq!(
        cli_local_command(root.path()).unwrap_err(),
        "Stakpak Linux CLI must be a regular executable"
    );
    fs::set_permissions(&entry, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(cli_local_command(root.path()).unwrap_err(), "Stakpak Linux CLI checksum mismatch");
}
