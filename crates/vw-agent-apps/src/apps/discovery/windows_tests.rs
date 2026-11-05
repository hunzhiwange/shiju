use super::windows::{find_executable, merge_paths};

#[test]
fn cc_apps_windows_discovery_paths_merge_unicode_and_priority() {
    assert_eq!(
        merge_paths(&[r"C:\用户\bin;C:\Tools;;relative", r"c:\tools;D:\npm", r"E:\machine"]),
        vec![r"C:\用户\bin", r"C:\Tools", r"D:\npm", r"E:\machine"]
    );
}

#[test]
fn cc_apps_windows_discovery_paths_extensions_and_alias_rejection() {
    let root = tempfile::tempdir().unwrap();
    let alias = root.path().join("Microsoft/WindowsApps");
    let first = root.path().join("用户 npm");
    let second = root.path().join("second");
    for dir in [&alias, &first, &second] {
        std::fs::create_dir_all(dir).unwrap();
    }
    for path in [
        alias.join("tool.exe"),
        first.join("tool.cmd"),
        first.join("tool.exe"),
        second.join("tool.exe"),
    ] {
        std::fs::write(path, b"fixture").unwrap();
    }
    let dirs = vec![alias, first.clone(), second];
    assert_eq!(find_executable("tool", &dirs, Some(".CMD;.EXE")), Some(first.join("tool.cmd")));
    assert_eq!(find_executable("tool", &dirs, None), Some(first.join("tool.exe")));
    assert_eq!(find_executable("missing", &dirs, None), None);
    assert_eq!(find_executable("../tool", &dirs, None), None);
    assert_eq!(find_executable("tool", &dirs, Some(".PS1")), None);
}

#[test]
fn cc_apps_windows_discovery_paths_package_managers() {
    let paths = super::windows::search_directories(
        &[r"D:\override"],
        &[("APPDATA".into(), r"C:\用户\Roaming".into()), ("PNPM_HOME".into(), r"D:\pnpm".into())],
    );
    assert_eq!(paths[0], r"D:\override");
    assert!(paths.contains(&r"C:\用户\Roaming\npm".into()));
    assert!(paths.contains(&r"D:\pnpm".into()));
}
