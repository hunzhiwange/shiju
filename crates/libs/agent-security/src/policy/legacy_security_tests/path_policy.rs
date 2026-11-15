use super::*;

// 测试相对路径被允许
#[test]
fn relative_paths_allowed() {
    let p = default_policy();
    assert!(p.is_path_allowed("file.txt"));
    assert!(p.is_path_allowed("src/main.rs"));
    assert!(p.is_path_allowed("deep/nested/dir/file.txt"));
}

// 测试路径穿越攻击被阻止
#[test]
fn path_traversal_blocked() {
    let p = default_policy();
    assert!(!p.is_path_allowed("../etc/passwd"));
    assert!(!p.is_path_allowed("../../root/.ssh/id_rsa"));
    assert!(!p.is_path_allowed("foo/../../../etc/shadow"));
    assert!(!p.is_path_allowed(".."));
}

// 测试 workspace_only 模式下绝对路径被阻止
#[test]
fn absolute_paths_blocked_when_workspace_only() {
    let p = default_policy();
    assert!(!p.is_path_allowed("/etc/passwd"));
    assert!(!p.is_path_allowed("/root/.ssh/id_rsa"));
    assert!(!p.is_path_allowed("/tmp/file.txt"));
}

// 测试 workspace_only 模式下工作区内绝对路径被允许
#[test]
fn absolute_workspace_path_allowed_when_workspace_only() {
    let workspace = std::env::temp_dir().join("vibewindow_test_absolute_workspace_path");
    let nested = workspace.join("src").join("main.rs");
    let _ = std::fs::remove_dir_all(&workspace);
    std::fs::create_dir_all(nested.parent().expect("nested parent should exist"))
        .expect("workspace tree should be created");

    let policy = SecurityPolicy {
        workspace_dir: workspace.clone(),
        ..SecurityPolicy::default()
    };

    assert!(
        policy.is_path_allowed(workspace.to_string_lossy().as_ref()),
        "workspace root absolute path must be allowed"
    );
    assert!(
        policy.is_path_allowed(nested.to_string_lossy().as_ref()),
        "workspace child absolute path must be allowed"
    );

    let _ = std::fs::remove_dir_all(&workspace);
}

// 测试 workspace_only 模式下 allowed_roots 内绝对路径被允许
#[test]
fn absolute_allowed_root_path_allowed_when_workspace_only() {
    let root = std::env::temp_dir().join("vibewindow_test_absolute_allowed_root");
    let workspace = root.join("workspace");
    let project = root.join("project");
    let project_file = project.join("Cargo.toml");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&workspace).expect("workspace should be created");
    std::fs::create_dir_all(&project).expect("project should be created");

    let policy = SecurityPolicy {
        workspace_dir: workspace,
        allowed_roots: vec![project.clone()],
        ..SecurityPolicy::default()
    };

    assert!(
        policy.is_path_allowed(project.to_string_lossy().as_ref()),
        "allowed root absolute path must be allowed"
    );
    assert!(
        policy.is_path_allowed(project_file.to_string_lossy().as_ref()),
        "file inside allowed root absolute path must be allowed"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// 测试非 workspace_only 模式下绝对路径被允许
#[test]
fn absolute_paths_allowed_when_not_workspace_only() {
    let p = SecurityPolicy {
        workspace_only: false,
        forbidden_paths: vec![],
        ..SecurityPolicy::default()
    };
    assert!(p.is_path_allowed("/tmp/file.txt"));
}

// 测试禁止路径被阻止
#[test]
fn forbidden_paths_blocked() {
    let p = SecurityPolicy { workspace_only: false, ..SecurityPolicy::default() };
    assert!(!p.is_path_allowed("/etc/passwd"));
    assert!(!p.is_path_allowed("/root/.bashrc"));
    assert!(!p.is_path_allowed("~/.ssh/id_rsa"));
    assert!(!p.is_path_allowed("~/.gnupg/pubring.kbx"));
}

// 测试空路径被允许
#[test]
fn empty_path_allowed() {
    let p = default_policy();
    assert!(p.is_path_allowed(""));
}

// 测试工作区内的点文件被允许
#[test]
fn dotfile_in_workspace_allowed() {
    let p = default_policy();
    assert!(p.is_path_allowed(".gitignore"));
    assert!(p.is_path_allowed(".env"));
}

// 测试编码点号的路径穿越被阻止
#[test]
fn path_traversal_encoded_dots() {
    let p = default_policy();
    assert!(!p.is_path_allowed("foo/..%2f..%2fetc/passwd"));
}

// 测试文件名中的双点号处理
#[test]
fn path_traversal_double_dot_in_filename() {
    let p = default_policy();
    assert!(p.is_path_allowed("my..file.txt"));
    assert!(!p.is_path_allowed("../etc/passwd"));
    assert!(!p.is_path_allowed("foo/../etc/passwd"));
}

// 测试包含空字节的路径被阻止
#[test]
fn path_with_null_byte_blocked() {
    let p = default_policy();
    assert!(!p.is_path_allowed("file\0.txt"));
}

// 测试符号链接风格的绝对路径被阻止
#[test]
fn path_symlink_style_absolute() {
    let p = default_policy();
    assert!(!p.is_path_allowed("/proc/self/root/etc/passwd"));
}

// 测试主目录波浪号路径（如 .ssh）被阻止
#[test]
fn path_home_tilde_ssh() {
    let p = SecurityPolicy { workspace_only: false, ..SecurityPolicy::default() };
    assert!(!p.is_path_allowed("~/.ssh/id_rsa"));
    assert!(!p.is_path_allowed("~/.gnupg/secring.gpg"));
    assert!(!p.is_path_allowed("~root/.ssh/id_rsa"));
    assert!(!p.is_path_allowed("~nobody"));
}

// 测试 /var/run 路径被阻止
#[test]
fn path_var_run_blocked() {
    let p = SecurityPolicy { workspace_only: false, ..SecurityPolicy::default() };
    assert!(!p.is_path_allowed("/var/run/docker.sock"));
}

// 测试完全自治模式仍遵守禁止路径
#[test]
fn full_autonomy_still_respects_forbidden_paths() {
    let p = SecurityPolicy {
        autonomy: AutonomyLevel::Full,
        workspace_only: false,
        ..SecurityPolicy::default()
    };
    assert!(!p.is_path_allowed("/etc/shadow"));
    assert!(!p.is_path_allowed("/root/.bashrc"));
}

// 测试 workspace_only=false 时允许解析的工作区外路径
#[test]
fn workspace_only_false_allows_resolved_outside_workspace() {
    let workspace = std::env::temp_dir().join("vibewindow_test_ws_only_false");
    let _ = std::fs::create_dir_all(&workspace);
    let canonical_workspace = workspace.canonicalize().unwrap_or_else(|_| workspace.clone());

    let p = SecurityPolicy {
        workspace_dir: canonical_workspace.clone(),
        workspace_only: false,
        forbidden_paths: vec!["/etc".into(), "/var".into()],
        ..SecurityPolicy::default()
    };

    let outside = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/home"))
        .join("vibewindow_outside_ws");
    assert!(
        p.is_resolved_path_allowed(&outside),
        "workspace_only=false must allow resolved paths outside workspace"
    );

    assert!(
        !p.is_resolved_path_allowed(std::path::Path::new("/etc/passwd")),
        "forbidden paths must be blocked even when workspace_only=false"
    );
    assert!(
        !p.is_resolved_path_allowed(std::path::Path::new("/var/run/docker.sock")),
        "forbidden /var must be blocked even when workspace_only=false"
    );

    let _ = std::fs::remove_dir_all(&workspace);
}

// 测试 workspace_only=true 时阻止解析的工作区外路径
#[test]
fn workspace_only_true_blocks_resolved_outside_workspace() {
    let workspace = std::env::temp_dir().join("vibewindow_test_ws_only_true");
    let _ = std::fs::create_dir_all(&workspace);
    let canonical_workspace = workspace.canonicalize().unwrap_or_else(|_| workspace.clone());

    let p = SecurityPolicy {
        workspace_dir: canonical_workspace.clone(),
        workspace_only: true,
        ..SecurityPolicy::default()
    };

    let inside = canonical_workspace.join("subdir");
    assert!(p.is_resolved_path_allowed(&inside), "path inside workspace must be allowed");

    let outside = std::env::temp_dir()
        .canonicalize()
        .unwrap_or_else(|_| std::env::temp_dir())
        .join("vibewindow_outside_ws_true");
    assert!(
        !p.is_resolved_path_allowed(&outside),
        "workspace_only=true must block resolved paths outside workspace"
    );

    let _ = std::fs::remove_dir_all(&workspace);
}

// 测试根路径被阻止
#[test]
fn checklist_root_path_blocked() {
    let p = default_policy();
    assert!(!p.is_path_allowed("/"));
    assert!(!p.is_path_allowed("/anything"));
}

// 测试所有系统目录被阻止
#[test]
fn checklist_all_system_dirs_blocked() {
    let p = SecurityPolicy { workspace_only: false, ..SecurityPolicy::default() };
    for dir in [
        "/etc", "/root", "/home", "/usr", "/bin", "/sbin", "/lib", "/opt", "/boot", "/dev",
        "/proc", "/sys", "/var", "/tmp",
    ] {
        assert!(!p.is_path_allowed(dir), "System dir should be blocked: {dir}");
        assert!(
            !p.is_path_allowed(&format!("{dir}/subpath")),
            "Subpath of system dir should be blocked: {dir}/subpath"
        );
    }
}

// 测试敏感点文件被阻止
#[test]
fn checklist_sensitive_dotfiles_blocked() {
    let p = SecurityPolicy { workspace_only: false, ..SecurityPolicy::default() };
    for path in ["~/.ssh/id_rsa", "~/.gnupg/secring.gpg", "~/.aws/credentials", "~/.config/secrets"]
    {
        assert!(!p.is_path_allowed(path), "Sensitive dotfile should be blocked: {path}");
    }
}

// 测试空字节注入被阻止
#[test]
fn checklist_null_byte_injection_blocked() {
    let p = default_policy();
    assert!(!p.is_path_allowed("safe\0/../../../etc/passwd"));
    assert!(!p.is_path_allowed("\0"));
    assert!(!p.is_path_allowed("file\0"));
}

// 测试 workspace_only 阻止所有绝对路径
#[test]
fn checklist_workspace_only_blocks_all_absolute() {
    let p = SecurityPolicy { workspace_only: true, ..SecurityPolicy::default() };
    assert!(!p.is_path_allowed("/any/absolute/path"));
    assert!(p.is_path_allowed("relative/path.txt"));
}

// 测试解析路径必须在工作区内
#[test]
fn checklist_resolved_path_must_be_in_workspace() {
    let p = SecurityPolicy {
        workspace_dir: PathBuf::from("/home/user/project"),
        ..SecurityPolicy::default()
    };
    assert!(p.is_resolved_path_allowed(std::path::Path::new("/home/user/project/src/main.rs")));
    assert!(!p.is_resolved_path_allowed(std::path::Path::new("/etc/passwd")));
    assert!(!p.is_resolved_path_allowed(std::path::Path::new("/home/user/other_project/file")));
    assert!(!p.is_resolved_path_allowed(std::path::Path::new("/")));
}

// 测试默认策略是 workspace_only
#[test]
fn checklist_default_policy_is_workspace_only() {
    let p = SecurityPolicy::default();
    assert!(p.workspace_only, "Default policy must be workspace_only=true");
}

// 测试默认禁止路径列表的完整性
#[test]
fn checklist_default_forbidden_paths_comprehensive() {
    let p = SecurityPolicy::default();
    for dir in ["/etc", "/root", "/proc", "/sys", "/dev", "/var", "/tmp"] {
        assert!(
            p.forbidden_paths.iter().any(|f| f == dir),
            "Default forbidden_paths must include {dir}"
        );
    }
    for dot in ["~/.ssh", "~/.gnupg", "~/.aws"] {
        assert!(
            p.forbidden_paths.iter().any(|f| f == dot),
            "Default forbidden_paths must include {dot}"
        );
    }
}

// 测试解析路径阻止工作区外的路径
#[test]
fn resolved_path_blocks_outside_workspace() {
    let workspace = std::env::temp_dir().join("vibewindow_test_resolved_path");
    let _ = std::fs::create_dir_all(&workspace);

    let canonical_workspace = workspace.canonicalize().unwrap_or_else(|_| workspace.clone());

    let policy =
        SecurityPolicy { workspace_dir: canonical_workspace.clone(), ..SecurityPolicy::default() };

    let inside = canonical_workspace.join("subdir").join("file.txt");
    assert!(policy.is_resolved_path_allowed(&inside), "path inside workspace should be allowed");

    let canonical_temp =
        std::env::temp_dir().canonicalize().unwrap_or_else(|_| std::env::temp_dir());
    let outside = canonical_temp.join("outside_workspace_vibewindow");
    assert!(!policy.is_resolved_path_allowed(&outside), "path outside workspace must be blocked");

    let _ = std::fs::remove_dir_all(&workspace);
}

// 测试解析路径阻止根目录逃逸
#[test]
fn resolved_path_blocks_root_escape() {
    let policy = SecurityPolicy {
        workspace_dir: PathBuf::from("/home/vibewindow_user/project"),
        ..SecurityPolicy::default()
    };

    assert!(
        !policy.is_resolved_path_allowed(std::path::Path::new("/etc/passwd")),
        "resolved path to /etc/passwd must be blocked"
    );
    assert!(
        !policy.is_resolved_path_allowed(std::path::Path::new("/root/.bashrc")),
        "resolved path to /root/.bashrc must be blocked"
    );
}

// 测试解析路径阻止符号链接逃逸
#[cfg(unix)]
#[test]
fn resolved_path_blocks_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join("vibewindow_test_symlink_escape");
    let workspace = root.join("workspace");
    let outside = root.join("outside_target");

    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&outside).unwrap();

    let link_path = workspace.join("escape_link");
    symlink(&outside, &link_path).unwrap();

    let policy = SecurityPolicy { workspace_dir: workspace.clone(), ..SecurityPolicy::default() };

    let resolved = link_path.canonicalize().unwrap();
    assert!(
        !policy.is_resolved_path_allowed(&resolved),
        "symlink-resolved path outside workspace must be blocked"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// 测试 allowed_roots 允许工作区外的路径
#[cfg(unix)]
#[test]
fn allowed_roots_permits_paths_outside_workspace() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join("vibewindow_test_allowed_roots");
    let workspace = root.join("workspace");
    let extra = root.join("extra_root");
    let extra_file = extra.join("data.txt");

    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&workspace).unwrap();
    std::fs::create_dir_all(&extra).unwrap();
    std::fs::write(&extra_file, "test").unwrap();

    let link_path = workspace.join("link_to_extra");
    symlink(&extra, &link_path).unwrap();

    let resolved = link_path.join("data.txt").canonicalize().unwrap();

    let policy_without = SecurityPolicy {
        workspace_dir: workspace.clone(),
        allowed_roots: vec![],
        ..SecurityPolicy::default()
    };
    assert!(
        !policy_without.is_resolved_path_allowed(&resolved),
        "without allowed_roots, symlink target must be blocked"
    );

    let policy_with = SecurityPolicy {
        workspace_dir: workspace.clone(),
        allowed_roots: vec![extra.clone()],
        ..SecurityPolicy::default()
    };
    assert!(
        policy_with.is_resolved_path_allowed(&resolved),
        "with allowed_roots containing the target, symlink must be allowed"
    );

    let unrelated = root.join("unrelated");
    std::fs::create_dir_all(&unrelated).unwrap();
    assert!(
        !policy_with.is_resolved_path_allowed(&unrelated.canonicalize().unwrap()),
        "paths outside workspace and allowed_roots must still be blocked"
    );

    let _ = std::fs::remove_dir_all(&root);
}

// 测试 is_path_allowed 阻止空字节
#[test]
fn is_path_allowed_blocks_null_bytes() {
    let policy = default_policy();
    assert!(!policy.is_path_allowed("file\0.txt"), "paths with null bytes must be blocked");
}

// 测试 is_path_allowed 阻止 URL 编码的路径穿越
#[test]
fn is_path_allowed_blocks_url_encoded_traversal() {
    let policy = default_policy();
    assert!(
        !policy.is_path_allowed("..%2fetc%2fpasswd"),
        "URL-encoded path traversal must be blocked"
    );
    assert!(
        !policy.is_path_allowed("subdir%2f..%2f..%2fetc"),
        "URL-encoded parent dir traversal must be blocked"
    );
}
