use super::*;
use std::path::PathBuf;
use windows_sys::Win32::System::ApplicationInstallationAndServicing::{
    MSIDBOPEN_CREATE, MSIDBOPEN_TRANSACT, MsiDatabaseCommit,
};

fn execute(database: &MsiHandle, sql: &str) {
    let query: Vec<_> = sql.encode_utf16().chain(Some(0)).collect();
    let mut raw = 0;
    check(unsafe { MsiDatabaseOpenViewW(database.0, query.as_ptr(), &mut raw) }).unwrap();
    let view = MsiHandle(raw);
    check(unsafe { MsiViewExecute(view.0, 0) }).unwrap();
}

fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf) {
    let root = tempfile::tempdir().unwrap();
    let directory = root.path().join("中文 cache");
    std::fs::create_dir(&directory).unwrap();
    let installer = directory.join("ShiJu update.msi");
    let path = wide_path(&installer).unwrap();
    let mut raw = 0;
    check(unsafe { MsiOpenDatabaseW(path.as_ptr(), MSIDBOPEN_CREATE, &mut raw) }).unwrap();
    let database = MsiHandle(raw);
    execute(
        &database,
        "CREATE TABLE `Property` (`Property` CHAR(72) NOT NULL, \
         `Value` CHAR(0) LOCALIZABLE PRIMARY KEY `Property`)",
    );
    for (name, value) in [
        ("ProductVersion", "1.2.3"),
        ("ProductCode", "{1DB11907-F231-4382-BAFC-4EDD725CC8FF}"),
        ("UpgradeCode", "{C8512FCC-FA63-4C65-9458-B5363A71E371}"),
    ] {
        execute(
            &database,
            &format!("INSERT INTO `Property` (`Property`, `Value`) VALUES ('{name}', '{value}')"),
        );
    }
    execute(
        &database,
        "CREATE TABLE `MsiFileHash` (`File_` CHAR(72) NOT NULL, `Options` SHORT NOT NULL, \
         `HashPart1` LONG NOT NULL, `HashPart2` LONG NOT NULL, \
         `HashPart3` LONG NOT NULL, `HashPart4` LONG NOT NULL PRIMARY KEY `File_`)",
    );
    for (id, name) in BUNDLED_FILES {
        let path = directory.join(name);
        std::fs::write(&path, format!("fixture {name}")).unwrap();
        let hash = file_hash(&path).unwrap();
        execute(
            &database,
            &format!(
                "INSERT INTO `MsiFileHash` \
                 (`File_`, `Options`, `HashPart1`, `HashPart2`, `HashPart3`, `HashPart4`) \
                 VALUES ('{id}', 0, {}, {}, {}, {})",
                hash[0] as i32, hash[1] as i32, hash[2] as i32, hash[3] as i32
            ),
        );
    }
    check(unsafe { MsiDatabaseCommit(database.0) }).unwrap();
    drop(database);
    (root, installer, directory.join("shi-ju.exe"))
}

#[test]
fn msi_inspection_verifies_payload_and_rejects_old_desktop_or_daemon() {
    let (_root, installer, executable) = fixture();
    let package = Package::inspect(&installer, "1.2.3").unwrap();
    assert!(!package.is_installed());
    package.verify_installed(&executable).unwrap();
    for name in ["shi-ju.exe", "shiju.exe"] {
        let path = executable.with_file_name(name);
        let original = std::fs::read(&path).unwrap();
        std::fs::write(&path, b"old executable").unwrap();
        let error = package.verify_installed(&executable).unwrap_err();
        assert!(error.to_string().contains(name));
        assert!(error.to_string().contains("升级尚未完成"));
        std::fs::write(path, original).unwrap();
    }
    package.verify_installed(&executable).unwrap();
}

#[test]
fn msi_inspection_rejects_manifest_version_mismatch_and_missing_file_hash() {
    let (_root, installer, _) = fixture();
    let error = Package::inspect(&installer, "1.2.4").err().unwrap();
    assert!(error.to_string().contains("1.2.3"));
    assert!(error.to_string().contains("1.2.4"));

    let path = wide_path(&installer).unwrap();
    let mut raw = 0;
    check(unsafe { MsiOpenDatabaseW(path.as_ptr(), MSIDBOPEN_TRANSACT, &mut raw) }).unwrap();
    let database = MsiHandle(raw);
    execute(&database, "DELETE FROM `MsiFileHash` WHERE `File_` = 'DesktopExecutableFile'");
    check(unsafe { MsiDatabaseCommit(database.0) }).unwrap();
    drop(database);
    let error = Package::inspect(&installer, "1.2.3").err().unwrap();
    assert!(error.to_string().contains("DesktopExecutableFile"));
}

#[test]
fn msi_inspection_rejects_files_that_are_not_installer_databases() {
    let root = tempfile::tempdir().unwrap();
    let installer = root.path().join("invalid.msi");
    std::fs::write(&installer, b"not an MSI database").unwrap();
    let error = Package::inspect(&installer, "1.2.3").err().unwrap();
    assert!(error.to_string().contains("无法读取安装包数据库"));
}

#[test]
fn updater_stops_installed_processes_and_preserves_same_named_processes_elsewhere() {
    use std::os::windows::process::CommandExt;
    use std::process::{Child, Command, Stdio};

    struct Running(Child);
    impl Drop for Running {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }

    let root = tempfile::tempdir().unwrap();
    let installed = root.path().join("ShiJu 安装");
    let unrelated = root.path().join("other");
    std::fs::create_dir(&installed).unwrap();
    std::fs::create_dir(&unrelated).unwrap();
    let shell = PathBuf::from(std::env::var_os("SystemRoot").unwrap()).join("System32/cmd.exe");
    let start = |path: &Path| {
        std::fs::copy(&shell, path).unwrap();
        let mut process = Command::new(path)
            .args(["/D", "/Q"])
            .creation_flags(0x08000000)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        assert!(process.try_wait().unwrap().is_none());
        Running(process)
    };
    let executable = installed.join("shi-ju.exe");
    let mut desktop = start(&executable);
    let mut daemon = start(&installed.join("shiju.exe"));
    let mut other = start(&unrelated.join("shiju.exe"));
    stop_installed_processes(&executable).unwrap();
    assert!(desktop.0.try_wait().unwrap().is_some());
    assert!(daemon.0.try_wait().unwrap().is_some());
    assert!(other.0.try_wait().unwrap().is_none());
}
