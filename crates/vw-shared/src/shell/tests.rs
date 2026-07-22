//! Shell 管理测试模块，覆盖环境解析、命令构建和安全进程终止行为。

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;
    use std::sync::{LazyLock, Mutex, MutexGuard};

    static SHELL_ENV_TEST_LOCK: LazyLock<Mutex<()>> = LazyLock::new(|| Mutex::new(()));

    fn shell_env_test_lock() -> MutexGuard<'static, ()> {
        SHELL_ENV_TEST_LOCK.lock().expect("shell env test lock should acquire")
    }

    struct EnvGuard {
        key: &'static str,
        original: Option<std::ffi::OsString>,
    }

    impl EnvGuard {
        fn set_os(key: &'static str, value: &std::ffi::OsStr) -> Self {
            let original = std::env::var_os(key);
            unsafe { std::env::set_var(key, value) };
            Self { key, original }
        }

        fn remove(key: &'static str) -> Self {
            let original = std::env::var_os(key);
            unsafe { std::env::remove_var(key) };
            Self { key, original }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.original {
                Some(value) => unsafe { std::env::set_var(self.key, value) },
                None => unsafe { std::env::remove_var(self.key) },
            }
        }
    }

    #[test]
    fn basename_returns_file_name_only_for_valid_paths() {
        assert_eq!(basename("/usr/local/bin/bash"), Some("bash".to_string()));
        assert_eq!(basename("nu"), Some("nu".to_string()));
        assert_eq!(basename("/"), None);
    }

    #[test]
    fn user_home_dir_prefers_home_and_falls_back_to_userprofile() {
        let _env_lock = shell_env_test_lock();
        let home = tempfile::TempDir::new().expect("home dir should be created");
        let userprofile = tempfile::TempDir::new().expect("userprofile dir should be created");
        let _home_guard = EnvGuard::set_os("HOME", home.path().as_os_str());
        let _userprofile_guard = EnvGuard::set_os("USERPROFILE", userprofile.path().as_os_str());

        assert_eq!(user_home_dir(), Some(home.path().to_path_buf()));

        drop(_home_guard);
        let _home_removed = EnvGuard::remove("HOME");

        assert_eq!(user_home_dir(), Some(userprofile.path().to_path_buf()));
    }

    #[test]
    fn user_home_dir_returns_none_when_home_variables_are_missing() {
        let _env_lock = shell_env_test_lock();
        let _home_guard = EnvGuard::remove("HOME");
        let _userprofile_guard = EnvGuard::remove("USERPROFILE");

        assert_eq!(user_home_dir(), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn shell_profile_paths_follow_shell_startup_precedence() {
        let home = Path::new("/home/demo");

        assert_eq!(
            shell_profile_paths(home),
            vec![
                PathBuf::from("/home/demo/.zshenv"),
                PathBuf::from("/home/demo/.zprofile"),
                PathBuf::from("/home/demo/.zshrc"),
                PathBuf::from("/home/demo/.bash_profile"),
                PathBuf::from("/home/demo/.bashrc"),
                PathBuf::from("/home/demo/.profile"),
            ]
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn strip_matching_quotes_only_removes_balanced_outer_quotes() {
        assert_eq!(strip_matching_quotes("\"/tmp/bin\""), "/tmp/bin");
        assert_eq!(strip_matching_quotes("'/tmp/bin'"), "/tmp/bin");
        assert_eq!(strip_matching_quotes("\"/tmp/bin'"), "\"/tmp/bin'");
        assert_eq!(strip_matching_quotes("/tmp/bin"), "/tmp/bin");
    }

    #[cfg(not(windows))]
    #[test]
    fn replace_shell_home_and_path_vars_expands_supported_tokens() {
        assert_eq!(
            replace_shell_home_and_path_vars("\"$HOME/bin:${PATH}\"", "/home/demo", "/usr/bin"),
            "/home/demo/bin:/usr/bin"
        );
        assert_eq!(replace_shell_home_and_path_vars("~", "/home/demo", "/usr/bin"), "/home/demo");
        assert_eq!(
            replace_shell_home_and_path_vars("~/tools", "/home/demo", "/usr/bin"),
            "/home/demo/tools"
        );
        assert_eq!(
            replace_shell_home_and_path_vars(" '${HOME}/bin:${PATH}' ", "/home/demo", "/usr/bin"),
            "/home/demo/bin:/usr/bin"
        );
        assert_eq!(
            replace_shell_home_and_path_vars("~other/tools", "/home/demo", "/usr/bin"),
            "~other/tools"
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn extract_path_assignment_rhs_supports_export_and_plain_assignments() {
        assert_eq!(
            extract_path_assignment_rhs("export PATH=/opt/homebrew/bin:$PATH"),
            Some("/opt/homebrew/bin:$PATH")
        );
        assert_eq!(
            extract_path_assignment_rhs("PATH=/usr/local/bin:$PATH"),
            Some("/usr/local/bin:$PATH")
        );
        assert_eq!(extract_path_assignment_rhs("export HOME=/tmp/demo"), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn extract_env_assignment_rhs_supports_custom_variable_name() {
        assert_eq!(
            extract_env_assignment_rhs("export CLAUDE_BIN=/opt/homebrew/bin/claude", "CLAUDE_BIN"),
            Some("/opt/homebrew/bin/claude")
        );
        assert_eq!(
            extract_env_assignment_rhs("CLAUDE_BIN=/opt/homebrew/bin/claude", "CLAUDE_BIN"),
            Some("/opt/homebrew/bin/claude")
        );
        assert_eq!(extract_env_assignment_rhs("export PATH=/usr/bin", "CLAUDE_BIN"), None);
        assert_eq!(extract_env_assignment_rhs("CLAUDE_BIN", "CLAUDE_BIN"), None);
        assert_eq!(extract_env_assignment_rhs("export CLAUDE_BIN=/tmp/bin", " CLAUDE_BIN "), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn brew_shellenv_dirs_detects_homebrew_prefixes() {
        let dirs = brew_shellenv_dirs("eval \"$(/opt/homebrew/bin/brew shellenv)\"")
            .expect("should detect /opt/homebrew brew shellenv");
        assert_eq!(
            dirs,
            vec![PathBuf::from("/opt/homebrew/bin"), PathBuf::from("/opt/homebrew/sbin")]
        );

        let dirs = brew_shellenv_dirs("eval \"$(/usr/local/bin/brew shellenv)\"")
            .expect("should detect /usr/local brew shellenv");
        assert_eq!(dirs, vec![PathBuf::from("/usr/local/bin"), PathBuf::from("/usr/local/sbin")]);

        assert_eq!(brew_shellenv_dirs("echo no brew here"), None);
        assert_eq!(brew_shellenv_dirs("eval \"$(/custom/bin/brew shellenv)\""), None);
    }

    #[test]
    fn which_finds_program_from_process_path() {
        let _env_lock = shell_env_test_lock();
        let dir = tempfile::TempDir::new().expect("temp dir should be created");
        let program_path = dir.path().join("demo-which-cli");
        std::fs::write(&program_path, b"#!/bin/sh\nexit 0\n").expect("program should be written");
        let _path_guard = EnvGuard::set_os("PATH", dir.path().as_os_str());

        assert_eq!(which("demo-which-cli"), Some(program_path));
    }

    #[test]
    fn which_returns_none_when_process_path_is_missing() {
        let _env_lock = shell_env_test_lock();
        let _path_guard = EnvGuard::remove("PATH");

        assert_eq!(which("definitely-missing-vw-shared-shell-test"), None);
    }

    #[test]
    fn which_returns_none_when_program_is_absent_from_present_path() {
        let _env_lock = shell_env_test_lock();
        let dir = tempfile::TempDir::new().expect("temp PATH dir should be created");
        let _path_guard = EnvGuard::set_os("PATH", dir.path().as_os_str());

        assert_eq!(which("definitely-missing-vw-shared-shell-test"), None);
    }

    #[test]
    fn push_unique_path_keeps_existing_dirs_once() {
        let dir = tempfile::TempDir::new().expect("temp dir should be created");
        let missing = dir.path().join("missing");
        let mut paths = Vec::new();

        push_unique_path(&mut paths, missing);
        push_unique_path(&mut paths, dir.path().to_path_buf());
        push_unique_path(&mut paths, dir.path().to_path_buf());

        assert_eq!(paths, vec![dir.path().to_path_buf()]);
    }

    #[test]
    fn push_known_path_keeps_missing_dir_and_dedupes() {
        let dir = tempfile::TempDir::new().expect("temp dir should be created");
        let missing = dir.path().join("missing");
        let mut paths = Vec::new();

        // 兜底目录即使不存在也必须保留，避免漏掉 homebrew 这类关键目录；重复项仍去重。
        push_known_path(&mut paths, missing.clone());
        push_known_path(&mut paths, missing.clone());
        push_known_path(&mut paths, dir.path().to_path_buf());

        assert_eq!(paths, vec![missing, dir.path().to_path_buf()]);
    }

    #[cfg(not(windows))]
    #[test]
    fn split_path_value_skips_empty_missing_and_duplicate_entries() {
        let dir = tempfile::TempDir::new().expect("temp dir should be created");
        let path_value =
            format!(":{}::{}:/definitely/missing:", dir.path().display(), dir.path().display());

        assert_eq!(split_path_value(&path_value), vec![dir.path().to_path_buf()]);
    }

    #[cfg(not(windows))]
    #[test]
    fn infer_path_from_shell_profiles_returns_none_without_assignments() {
        let home = tempfile::TempDir::new().expect("temp home should be created");
        std::fs::write(home.path().join(".zshrc"), "echo hello\n")
            .expect("profile should be written");

        assert_eq!(infer_path_from_shell_profiles(home.path()), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn infer_path_from_shell_profiles_reads_path_exports() {
        let home = tempfile::TempDir::new().expect("temp home should be created");
        let custom_bin = home.path().join("custom-bin");
        let local_bin = home.path().join(".local/bin");
        let cargo_bin = home.path().join(".cargo/bin");
        let bun_bin = home.path().join(".bun/bin");
        std::fs::create_dir_all(&custom_bin).expect("custom bin dir should be created");
        std::fs::create_dir_all(&local_bin).expect("local bin dir should be created");
        std::fs::create_dir_all(&cargo_bin).expect("cargo bin dir should be created");
        std::fs::create_dir_all(&bun_bin).expect("bun bin dir should be created");
        std::fs::write(
            home.path().join(".zshrc"),
            format!("export PATH={}:$PATH\nPATH=~/.local/bin:$PATH\n", custom_bin.display()),
        )
        .expect("profile should be written");

        let inferred = infer_path_from_shell_profiles(home.path())
            .expect("path should be inferred from profile");

        assert!(inferred.contains(&custom_bin.to_string_lossy().to_string()));
        assert!(inferred.contains(&local_bin.to_string_lossy().to_string()));
    }

    #[cfg(not(windows))]
    #[test]
    fn infer_path_from_shell_profiles_applies_profiles_in_startup_order() {
        let _env_lock = shell_env_test_lock();
        let home = tempfile::TempDir::new().expect("temp home should be created");
        std::fs::write(home.path().join(".zshenv"), "PATH=$HOME/first:$PATH\n")
            .expect("zshenv should be written");
        std::fs::write(home.path().join(".profile"), "export PATH=${HOME}/last:${PATH}\n")
            .expect("profile should be written");
        let _path_guard = EnvGuard::set_os("PATH", std::ffi::OsStr::new("/system/bin"));

        assert_eq!(
            infer_path_from_shell_profiles(home.path()),
            Some(format!(
                "{}/last:{}/first:/system/bin",
                home.path().display(),
                home.path().display()
            ))
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn infer_env_var_from_shell_profiles_trims_name_and_uses_last_assignment() {
        let _env_lock = shell_env_test_lock();
        let home = tempfile::TempDir::new().expect("temp home should be created");
        let first = home.path().join("first");
        let second = home.path().join("second");
        std::fs::write(
            home.path().join(".zshrc"),
            format!("\nCLAUDE_BIN={}\nexport CLAUDE_BIN={}\n", first.display(), second.display()),
        )
        .expect("profile should be written");
        let _path_guard = EnvGuard::set_os("PATH", std::ffi::OsStr::new("/usr/bin"));

        assert_eq!(
            infer_env_var_from_shell_profiles(home.path(), " CLAUDE_BIN "),
            Some(second.to_string_lossy().to_string())
        );
        assert_eq!(infer_env_var_from_shell_profiles(home.path(), " "), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn infer_env_var_from_shell_profiles_filters_blank_result() {
        let home = tempfile::TempDir::new().expect("temp home should be created");
        std::fs::write(home.path().join(".profile"), "export CLAUDE_BIN=   \n")
            .expect("profile should be written");

        assert_eq!(infer_env_var_from_shell_profiles(home.path(), "CLAUDE_BIN"), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn infer_env_var_from_shell_profiles_expands_home_and_path_tokens() {
        let _env_lock = shell_env_test_lock();
        let home = tempfile::TempDir::new().expect("temp home should be created");
        std::fs::write(home.path().join(".bashrc"), "export TOOL_PATH=\"$HOME/bin:${PATH}\"\n")
            .expect("profile should be written");
        let _path_guard = EnvGuard::set_os("PATH", std::ffi::OsStr::new("/system/bin"));

        assert_eq!(
            infer_env_var_from_shell_profiles(home.path(), "TOOL_PATH"),
            Some(format!("{}/bin:/system/bin", home.path().display()))
        );
    }

    #[cfg(not(windows))]
    #[test]
    fn effective_path_env_uses_profile_path_entries() {
        let _env_lock = shell_env_test_lock();
        let home = tempfile::TempDir::new().expect("temp home should be created");
        let profile_bin = home.path().join("profile-bin");
        let local_bin = home.path().join(".local/bin");
        let cargo_bin = home.path().join(".cargo/bin");
        let bun_bin = home.path().join(".bun/bin");
        std::fs::create_dir_all(&profile_bin).expect("profile bin dir should be created");
        std::fs::create_dir_all(&local_bin).expect("local bin dir should be created");
        std::fs::create_dir_all(&cargo_bin).expect("cargo bin dir should be created");
        std::fs::create_dir_all(&bun_bin).expect("bun bin dir should be created");
        std::fs::write(
            home.path().join(".zshrc"),
            format!("export PATH={}:$PATH\n", profile_bin.display()),
        )
        .expect("profile should be written");

        let _home_guard = EnvGuard::set_os("HOME", home.path().as_os_str());
        let _path_guard = EnvGuard::set_os("PATH", std::ffi::OsStr::new("/usr/bin:/bin"));

        let path = effective_path_env().expect("effective PATH should be available");
        assert!(path.contains(&profile_bin.to_string_lossy().to_string()));
        assert!(path.contains(&local_bin.to_string_lossy().to_string()));
        assert!(path.contains(&cargo_bin.to_string_lossy().to_string()));
        assert!(path.contains(&bun_bin.to_string_lossy().to_string()));
    }

    #[cfg(not(windows))]
    #[test]
    fn effective_path_env_keeps_common_dirs_under_minimal_daemon_path() {
        let _env_lock = shell_env_test_lock();
        let home = tempfile::TempDir::new().expect("temp home should be created");

        // 守护进程常以最小 PATH 启动（无 homebrew）。子进程必须仍能拿到常见命令目录，
        // 否则 `npx` 这类 shebang 脚本会因找不到 `node` 而以 ENOENT 失败。
        let _home_guard = EnvGuard::set_os("HOME", home.path().as_os_str());
        let _path_guard = EnvGuard::set_os("PATH", std::ffi::OsStr::new("/usr/bin:/bin"));

        let path = effective_path_env().expect("effective PATH should be available");
        for dir in COMMON_COMMAND_DIRS {
            assert!(
                path.split(':').any(|entry| entry == *dir),
                "missing common dir {dir} in {path}"
            );
        }
    }

    #[test]
    fn build_effective_path_uses_process_path_entries() {
        let process_path = std::env::var_os("PATH").expect("test process PATH should be present");
        let expected = std::env::split_paths(&process_path)
            .find(|entry| entry.is_dir())
            .expect("test process PATH should contain an existing directory");

        let path = build_effective_path().expect("process PATH should produce effective PATH");

        assert!(std::env::split_paths(&path).any(|entry| entry == expected));
    }

    #[cfg(windows)]
    #[test]
    fn build_effective_path_returns_none_without_process_path() {
        let _env_lock = shell_env_test_lock();
        let _path_guard = EnvGuard::remove("PATH");

        assert_eq!(build_effective_path(), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn shell_profile_env_var_reads_from_profile_when_process_env_missing() {
        let _env_lock = shell_env_test_lock();
        let home = tempfile::TempDir::new().expect("temp home should be created");
        let expected = home.path().join("tools/claude");
        std::fs::create_dir_all(expected.parent().expect("parent dir should exist"))
            .expect("parent dir should be created");
        std::fs::write(
            home.path().join(".zshrc"),
            format!("export CLAUDE_BIN={}\n", expected.display()),
        )
        .expect("profile should be written");

        let _home_guard = EnvGuard::set_os("HOME", home.path().as_os_str());
        let _claude_bin_guard = {
            let original = std::env::var_os("CLAUDE_BIN");
            unsafe { std::env::remove_var("CLAUDE_BIN") };
            EnvGuard { key: "CLAUDE_BIN", original }
        };

        let value = shell_profile_env_var("CLAUDE_BIN").expect("CLAUDE_BIN should be inferred");
        assert_eq!(value, expected.to_string_lossy().to_string());
    }

    #[cfg(not(windows))]
    #[test]
    fn shell_profile_env_var_falls_back_when_process_env_is_blank() {
        let _env_lock = shell_env_test_lock();
        let home = tempfile::TempDir::new().expect("temp home should be created");
        std::fs::write(home.path().join(".profile"), "CLAUDE_BIN=from-profile\n")
            .expect("profile should be written");
        let _home_guard = EnvGuard::set_os("HOME", home.path().as_os_str());
        let _value_guard = EnvGuard::set_os("CLAUDE_BIN", std::ffi::OsStr::new("   "));

        assert_eq!(shell_profile_env_var("CLAUDE_BIN"), Some("from-profile".to_string()));
    }

    #[test]
    fn shell_profile_env_var_prefers_non_empty_process_env() {
        let _env_lock = shell_env_test_lock();
        let _guard =
            EnvGuard::set_os("VIBEWINDOW_SHELL_TEST_VALUE", std::ffi::OsStr::new("from-env"));

        assert_eq!(
            shell_profile_env_var("VIBEWINDOW_SHELL_TEST_VALUE"),
            Some("from-env".to_string())
        );
    }

    #[test]
    fn shell_profile_env_var_returns_none_without_env_or_home() {
        let _env_lock = shell_env_test_lock();
        let _value_guard = EnvGuard::remove("VIBEWINDOW_MISSING_SHELL_TEST_VALUE");
        let _home_guard = EnvGuard::remove("HOME");
        let _userprofile_guard = EnvGuard::remove("USERPROFILE");

        assert_eq!(shell_profile_env_var("VIBEWINDOW_MISSING_SHELL_TEST_VALUE"), None);
    }

    #[test]
    fn resolve_executable_accepts_existing_absolute_path() {
        let file = tempfile::NamedTempFile::new().expect("temp file should be created");
        let resolved = resolve_executable(file.path().to_string_lossy().as_ref())
            .expect("absolute file path should resolve");
        assert_eq!(resolved, file.path());
    }

    #[test]
    fn resolve_executable_accepts_existing_relative_path() {
        let current_dir = std::env::current_dir().expect("current dir should be available");
        let dir = tempfile::Builder::new()
            .prefix("vw-shared-shell-relative-")
            .tempdir_in(&current_dir)
            .expect("temp dir should be created in current dir");
        let program = dir.path().join("demo-relative-cli");
        std::fs::write(&program, b"not executed").expect("program should be written");
        let relative_program = PathBuf::from(".").join(
            program.strip_prefix(&current_dir).expect("temp program should be below current dir"),
        );

        assert_eq!(
            resolve_executable(relative_program.to_string_lossy().as_ref()),
            Some(relative_program)
        );
    }

    #[test]
    fn resolve_executable_rejects_empty_and_missing_path_inputs() {
        assert_eq!(resolve_executable("  "), None);
        assert_eq!(resolve_executable("./definitely-missing-vw-shared-shell-test"), None);
        assert_eq!(resolve_executable("/definitely/missing/vw-shared-shell-test"), None);
    }

    #[test]
    fn resolve_executable_finds_program_from_effective_path() {
        let _env_lock = shell_env_test_lock();
        let dir = tempfile::TempDir::new().expect("temp dir should be created");
        let program_path = dir.path().join("demo-effective-cli");
        std::fs::write(&program_path, b"#!/bin/sh\nexit 0\n").expect("program should be written");
        let _path_guard = EnvGuard::set_os("PATH", dir.path().as_os_str());

        assert_eq!(resolve_executable("demo-effective-cli"), Some(program_path));
    }

    #[cfg(not(windows))]
    #[test]
    fn resolve_executable_finds_program_from_profile_augmented_path() {
        let _env_lock = shell_env_test_lock();
        let home = tempfile::TempDir::new().expect("temp home should be created");
        let local_bin = home.path().join(".local/bin");
        std::fs::create_dir_all(&local_bin).expect("local bin should be created");
        let program_path = local_bin.join("demo-profile-cli");
        std::fs::write(&program_path, b"#!/bin/sh\nexit 0\n").expect("program should be written");
        std::fs::write(home.path().join(".zshrc"), "PATH=$PATH\n")
            .expect("profile should be written");

        let _home_guard = EnvGuard::set_os("HOME", home.path().as_os_str());
        let _path_guard = EnvGuard::set_os("PATH", std::ffi::OsStr::new("/usr/bin:/bin"));

        assert_eq!(resolve_executable("demo-profile-cli"), Some(program_path));
    }

    #[test]
    fn resolved_program_or_original_keeps_unresolved_program_name() {
        assert_eq!(
            resolved_program_or_original("definitely-missing-vw-shared-shell-test"),
            PathBuf::from("definitely-missing-vw-shared-shell-test")
        );
    }

    #[test]
    fn which_in_path_value_finds_program_in_custom_path() {
        let dir = tempfile::TempDir::new().expect("temp dir should be created");
        let program_name = if cfg!(windows) { "demo-cli.exe" } else { "demo-cli" };
        let program_path = dir.path().join(program_name);
        std::fs::write(&program_path, b"#!/bin/sh\nexit 0\n").expect("program should be written");

        let found = which_in_path_value(program_name, &std::ffi::OsString::from(dir.path()))
            .expect("program should be found in custom path");
        assert_eq!(found, program_path);
    }

    #[test]
    fn which_in_path_value_returns_none_when_program_is_absent() {
        let dir = tempfile::TempDir::new().expect("temp dir should be created");

        assert_eq!(which_in_path_value("missing-cli", &std::ffi::OsString::from(dir.path())), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn std_command_uses_profile_augmented_path() {
        let _env_lock = shell_env_test_lock();
        let home = tempfile::TempDir::new().expect("temp home should be created");
        let bin_dir = home.path().join("profile-bin");
        std::fs::create_dir_all(&bin_dir).expect("profile bin dir should be created");
        std::fs::write(
            home.path().join(".zshrc"),
            format!("export PATH={}:$PATH\n", bin_dir.display()),
        )
        .expect("profile should be written");

        let _home_guard = EnvGuard::set_os("HOME", home.path().as_os_str());
        let _path_guard = EnvGuard::set_os("PATH", std::ffi::OsStr::new("/usr/bin:/bin"));

        let command = std_command("git");
        let path_value = command
            .get_envs()
            .find(|(key, _)| *key == std::ffi::OsStr::new("PATH"))
            .and_then(|(_, value)| value)
            .and_then(|value| value.to_str())
            .expect("PATH should be set on std_command");
        assert!(path_value.contains(&bin_dir.to_string_lossy().to_string()));
    }

    #[test]
    fn apply_augmented_path_std_command_sets_path_when_available() {
        let mut command = std::process::Command::new("demo");
        apply_augmented_path_std_command(&mut command);

        assert!(
            command
                .get_envs()
                .any(|(key, value)| { key == std::ffi::OsStr::new("PATH") && value.is_some() })
        );
    }

    #[cfg(feature = "shell-tokio")]
    #[test]
    fn tokio_command_uses_resolved_program_and_augmented_path() {
        let command = tokio_command("git");
        let debug = format!("{command:?}");

        assert!(debug.contains("PATH"));
    }

    #[test]
    fn std_system_command_does_not_inject_augmented_path() {
        let command = std_system_command("open");

        assert!(command.get_envs().next().is_none());
        assert_eq!(command.get_program(), std::ffi::OsStr::new("open"));
    }

    #[cfg(feature = "shell-tokio")]
    #[test]
    fn tokio_system_command_uses_plain_program() {
        let command = tokio_system_command("open");
        let debug = format!("{command:?}");

        assert!(debug.contains("\"open\""));
        assert!(!debug.contains("PATH"));
    }

    #[test]
    fn git_std_command_applies_git_default_environment() {
        let command = git_std_command();

        for &(expected_key, expected_value) in GIT_DEFAULT_ENV {
            let actual = command
                .get_envs()
                .find(|(key, _)| *key == std::ffi::OsStr::new(expected_key))
                .and_then(|(_, value)| value);
            assert_eq!(actual, Some(std::ffi::OsStr::new(expected_value)));
        }
    }

    #[cfg(feature = "shell-tokio")]
    #[test]
    fn git_tokio_command_applies_git_default_environment() {
        let command = git_tokio_command();
        let debug = format!("{command:?}");

        assert!(debug.contains("GIT_PAGER"));
        assert!(debug.contains("GIT_MERGE_AUTOEDIT"));
        assert!(debug.contains("GIT_EDITOR"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn fallback_uses_macos_default_shell() {
        assert_eq!(fallback(), "/bin/zsh");
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    #[test]
    fn fallback_uses_bin_sh_when_bash_is_not_on_path() {
        let _env_lock = shell_env_test_lock();
        let _path_guard = EnvGuard::remove("PATH");

        assert_eq!(fallback(), "/bin/sh");
    }

    #[cfg(all(not(windows), not(target_os = "macos")))]
    #[test]
    fn fallback_uses_bash_found_on_path() {
        let _env_lock = shell_env_test_lock();
        let bin = tempfile::TempDir::new().expect("temp bin dir should be created");
        let bash = bin.path().join("bash");
        std::fs::write(&bash, b"not executed").expect("bash file should be written");
        let _path_guard = EnvGuard::set_os("PATH", bin.path().as_os_str());

        assert_eq!(fallback(), bash.to_string_lossy().to_string());
    }

    #[cfg(windows)]
    #[test]
    fn fallback_prefers_configured_git_bash_file() {
        let _env_lock = shell_env_test_lock();
        let bash = tempfile::NamedTempFile::new().expect("temp bash file should be created");
        let _bash_guard = EnvGuard::set_os("VIBEWINDOW_GIT_BASH_PATH", bash.path().as_os_str());

        assert_eq!(fallback(), bash.path().to_string_lossy().to_string());
    }

    #[cfg(windows)]
    #[test]
    fn fallback_finds_git_bash_relative_to_git_on_path() {
        let _env_lock = shell_env_test_lock();
        let git_root = tempfile::TempDir::new().expect("temp git root should be created");
        let cmd_dir = git_root.path().join("cmd");
        let bin_dir = git_root.path().join("bin");
        std::fs::create_dir_all(&cmd_dir).expect("git cmd dir should be created");
        std::fs::create_dir_all(&bin_dir).expect("git bin dir should be created");
        std::fs::write(cmd_dir.join("git.exe"), b"not executed")
            .expect("git file should be written");
        let bash = bin_dir.join("bash.exe");
        std::fs::write(&bash, b"not executed").expect("bash file should be written");
        let _configured_bash_guard = EnvGuard::remove("VIBEWINDOW_GIT_BASH_PATH");
        let _path_guard = EnvGuard::set_os("PATH", cmd_dir.as_os_str());

        assert_eq!(fallback(), bash.to_string_lossy().to_string());
    }

    #[cfg(windows)]
    #[test]
    fn fallback_uses_comspec_when_git_bash_is_unavailable() {
        let _env_lock = shell_env_test_lock();
        let _configured_bash_guard = EnvGuard::remove("VIBEWINDOW_GIT_BASH_PATH");
        let _path_guard = EnvGuard::remove("PATH");
        let _comspec_guard = EnvGuard::set_os("COMSPEC", std::ffi::OsStr::new("custom-cmd.exe"));

        assert_eq!(fallback(), "custom-cmd.exe");
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn preferred_and_acceptable_shells_initialize_from_environment() {
        let _env_lock = shell_env_test_lock();
        let _shell_guard = EnvGuard::set_os("SHELL", std::ffi::OsStr::new("/usr/local/bin/bash"));

        assert_eq!(&**PREFERRED, "/usr/local/bin/bash");
        assert_eq!(&**ACCEPTABLE, "/usr/local/bin/bash");
    }

    #[cfg(not(windows))]
    #[test]
    fn kill_tree_returns_without_killing_when_exited_callback_is_true() {
        let mut child = std::process::Command::new("sh")
            .args(["-c", "sleep 5"])
            .spawn()
            .expect("sleep child should start");

        kill_tree(&mut child, Some(&|| true));

        assert!(child.try_wait().expect("child status should be readable").is_none());
        child.kill().expect("child should be killed for cleanup");
        let _ = child.wait();
    }

    #[cfg(not(windows))]
    #[test]
    fn kill_tree_kills_child_when_process_group_signal_fails() {
        let mut child = std::process::Command::new("sh")
            .args(["-c", "sleep 5"])
            .spawn()
            .expect("sleep child should start");

        kill_tree(&mut child, None);

        assert!(!child.wait().expect("child should report status").success());
    }

    #[cfg(not(windows))]
    #[test]
    fn kill_tree_uses_process_group_signal_when_available() {
        use std::os::unix::process::CommandExt;

        let mut command = std::process::Command::new("sh");
        command.args(["-c", "trap '' TERM; sleep 5"]);
        unsafe {
            command.pre_exec(|| {
                if libc::setsid() == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let mut child = command.spawn().expect("process group child should start");

        kill_tree(&mut child, None);

        assert!(!child.wait().expect("child should report status").success());
    }
}
