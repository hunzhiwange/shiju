use super::posix::*;
use std::{
    collections::BTreeMap, ffi::OsString, fs, os::unix::fs::PermissionsExt, path::Path,
    time::Duration,
};

#[test]
fn cc_apps_posix_discovery_paths_managers_and_login_order() {
    let root = tempfile::tempdir().unwrap();
    let dirs = [
        ".local/bin",
        ".npm-global/bin",
        "n/bin",
        ".volta/bin",
        ".local/share/mise/shims",
        ".local/share/mise/installs/node/22/bin",
        ".local/state/fnm_multishells/session/bin",
        ".nvm/versions/node/v22/bin",
        "Library/pnpm",
        ".local/share/pnpm",
        "login",
        "inherited",
    ];
    for dir in dirs {
        fs::create_dir_all(root.path().join(dir)).unwrap();
    }
    let env = PosixEnvironment {
        home: root.path().into(),
        path: root.path().join("inherited").into_os_string(),
        login_path: Some(root.path().join("login").into_os_string()),
        variables: BTreeMap::new(),
    };
    let paths = build_tool_search_paths("codex", &env);
    for dir in dirs {
        assert!(paths.contains(&root.path().join(dir)), "missing {dir}");
    }
    assert!(paths.iter().all(|p| p.is_absolute() && p.is_dir()));
    assert!(
        paths.iter().position(|p| p.ends_with("login"))
            < paths.iter().position(|p| p.ends_with("inherited"))
    );
    assert_eq!(merge_path_segments("/a::/b:/a", "/b:/c:"), "/a:/b:/c");
}

#[test]
fn cc_apps_posix_discovery_paths_app_overrides_and_rejection() {
    let root = tempfile::tempdir().unwrap();
    let mut env = PosixEnvironment {
        home: root.path().into(),
        path: OsString::from(":relative:/does-not-exist"),
        login_path: None,
        variables: BTreeMap::new(),
    };
    for (tool, key, suffix) in [
        ("kimi", "KIMI_INSTALL_DIR", "bin"),
        ("grok", "GROK_BIN_DIR", ""),
        ("mcode", "MCODE_INSTALL_DIR", "bin"),
        ("opencode", "OPENCODE_INSTALL_DIR", ""),
    ] {
        let base = root.path().join(tool);
        let expected = base.join(suffix);
        fs::create_dir_all(&expected).unwrap();
        env.variables.insert(key.into(), base.into_os_string());
        assert!(build_tool_search_paths(tool, &env).contains(&expected));
        assert!(!build_tool_search_paths("gemini", &env).contains(&expected));
    }
    fs::write(root.path().join("file"), "").unwrap();
    env.path = root.path().join("file").into_os_string();
    assert!(!build_tool_search_paths("gemini", &env).contains(&root.path().join("file")));
}

#[test]
fn cc_apps_posix_discovery_paths_shell_success_failure_timeout() {
    let root = tempfile::tempdir().unwrap();
    let shell = root.path().join("zsh");
    fs::write(&shell, "#!/bin/sh\n[ \"$1\" = -lic ] || exit 9\n[ \"$2\" = '/usr/bin/printenv PATH' ] || exit 8\nprintf 'welcome\\n/custom/bin:/usr/bin\\n'\n").unwrap();
    fs::set_permissions(&shell, fs::Permissions::from_mode(0o700)).unwrap();
    assert_eq!(
        login_shell_path(&shell, Duration::from_secs(1)).unwrap(),
        OsString::from("/custom/bin:/usr/bin")
    );
    fs::write(&shell, "#!/bin/sh\nexit 3\n").unwrap();
    assert!(login_shell_path(&shell, Duration::from_secs(1)).is_err());
    fs::write(&shell, "#!/bin/sh\nsleep 10\n").unwrap();
    assert_eq!(
        login_shell_path(&shell, Duration::from_millis(30)).unwrap_err().kind(),
        std::io::ErrorKind::TimedOut
    );
    assert!(login_shell_path(Path::new("relative"), Duration::from_secs(1)).is_err());
}

#[test]
fn cc_apps_posix_discovery_paths_bun_go_and_real_shell() {
    let root = tempfile::tempdir().unwrap();
    for dir in [".bun/bin", ".opencode/bin", "go/bin", "go-extra/bin"] {
        fs::create_dir_all(root.path().join(dir)).unwrap();
    }
    let env = PosixEnvironment {
        home: root.path().into(),
        path: OsString::new(),
        login_path: None,
        variables: BTreeMap::from([(
            "GOPATH".into(),
            root.path().join("go-extra").into_os_string(),
        )]),
    };
    let paths = build_tool_search_paths("opencode", &env);
    for dir in [".bun/bin", ".opencode/bin", "go/bin", "go-extra/bin"] {
        assert!(paths.contains(&root.path().join(dir)));
    }
    assert!(build_tool_search_paths("omp", &env).contains(&root.path().join(".bun/bin")));
    assert!(!build_tool_search_paths("gemini", &env).contains(&root.path().join(".bun/bin")));
    let path = login_shell_path(Path::new("/bin/sh"), Duration::from_secs(2)).unwrap();
    assert!(!path.is_empty());
}
