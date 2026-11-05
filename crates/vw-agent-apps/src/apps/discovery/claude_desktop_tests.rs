use super::claude_desktop::{DesktopPlatform, probe};
use std::{fs, path::Path};

#[test]
fn cc_apps_claude_desktop_presence_bundle_is_independent_of_cli_and_config() {
    let root = tempfile::tempdir().unwrap();
    let home = root.path().join("home");
    let apps = root.path().join("Applications");
    fs::create_dir_all(home.join(".local/bin")).unwrap();
    fs::write(home.join(".local/bin/claude"), "cli only").unwrap();
    assert_eq!(probe(DesktopPlatform::Macos, &home, &apps).unwrap().app_bundles, Some(vec![]));
    let config = home.join("Library/Application Support/Claude");
    fs::create_dir_all(&config).unwrap();
    let status = probe(DesktopPlatform::Macos, &home, &apps).unwrap();
    assert!(status.supported);
    assert_eq!(status.app_bundles, Some(vec![]));
    assert_eq!(status.config_directories, vec![config.clone()]);
    for bundle in [apps.join("Claude.app"), home.join("Applications/Claude.app")] {
        fs::create_dir_all(&bundle).unwrap();
    }
    let status = probe(DesktopPlatform::Macos, &home, &apps).unwrap();
    assert_eq!(
        status.app_bundles,
        Some(vec![apps.join("Claude.app"), home.join("Applications/Claude.app")])
    );
    assert!(config.is_dir());
    assert!(!config.join("claude_desktop_config.json").exists());
}

#[test]
fn cc_apps_claude_desktop_presence_platforms_do_not_report_false_absence() {
    let root = tempfile::tempdir().unwrap();
    for platform in [DesktopPlatform::Windows, DesktopPlatform::Linux] {
        fs::create_dir_all(root.path().join("Claude-3p")).unwrap();
        let status = probe(platform, root.path(), root.path()).unwrap();
        assert!(status.supported);
        assert_eq!(status.app_bundles, None);
        assert_eq!(status.config_directories, vec![root.path().join("Claude-3p")]);
    }
    let status = probe(DesktopPlatform::Unsupported, Path::new(""), Path::new("")).unwrap();
    assert!(!status.supported);
    assert_eq!(status.app_bundles, None);
}

#[test]
fn cc_apps_claude_desktop_presence_rejects_relative_roots_and_io_errors() {
    assert_eq!(
        probe(DesktopPlatform::Macos, Path::new("relative"), Path::new("/Applications"))
            .unwrap_err()
            .kind(),
        std::io::ErrorKind::InvalidInput
    );
    let root = tempfile::tempdir().unwrap();
    fs::write(root.path().join("Claude.app"), "not a bundle").unwrap();
    assert_eq!(
        probe(DesktopPlatform::Macos, root.path(), root.path()).unwrap().app_bundles,
        Some(vec![])
    );
    fs::write(root.path().join("file"), "not a directory").unwrap();
    assert!(probe(DesktopPlatform::Macos, root.path(), &root.path().join("file")).is_err());
}

#[test]
fn cc_apps_claude_desktop_presence_windows_versioned_config_is_deterministic() {
    let root = tempfile::tempdir().unwrap();
    for name in ["Claude-z", "Claude-a", "Claude-3p-z", "Claude-3p-a"] {
        fs::create_dir(root.path().join(name)).unwrap();
    }
    let status = probe(DesktopPlatform::Windows, root.path(), Path::new("")).unwrap();
    assert_eq!(
        status.config_directories,
        vec![root.path().join("Claude-a"), root.path().join("Claude-3p-a")]
    );
    fs::create_dir(root.path().join("Claude")).unwrap();
    let status = probe(DesktopPlatform::Windows, root.path(), Path::new("")).unwrap();
    assert_eq!(status.config_directories[0], root.path().join("Claude"));
    assert_eq!(status.app_bundles, None);
}
