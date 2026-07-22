//! # 原生运行时单元测试模块
//!
//! 本模块包含 `NativeRuntime` 及其相关功能的单元测试，验证原生运行时的各项特性和行为。
//!
//! ## 测试范围
//!
//! - **运行时基础属性**：验证运行时名称、能力标识（Shell 访问、文件系统访问等）
//! - **Shell 检测逻辑**：Windows 和 Unix 系统下 Shell 的自动检测与优先级回退
//! - **WSL 启动器识别**：排除 Windows Subsystem for Linux 的 bash.exe（非真正的 bash）
//! - **命令构建**：验证不同 Shell 类型（PowerShell、CMD、Zsh、Bash）的命令构建正确性
//!
//! ## 测试策略
//!
//! 测试分为两类：
//! 1. **直接测试**：直接创建 `NativeRuntime` 实例并验证其属性
//! 2. **注入测试**：使用 `detect_native_shell_with` 函数注入模拟环境，测试特定场景

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    /// 测试原生运行时的名称标识
    ///
    /// 验证 `NativeRuntime::new()` 返回的运行时名称为 `"native"`，
    /// 用于在运行时工厂中识别和路由到原生运行时实现。
    #[test]
    fn native_name() {
        assert_eq!(NativeRuntime::new().name(), "native");
    }

    /// 测试 Shell 访问能力的检测
    ///
    /// 验证 `has_shell_access()` 返回值与系统实际 Shell 检测结果一致。
    /// 当系统存在可用 Shell 时返回 `true`，否则返回 `false`。
    #[test]
    fn native_has_shell_access() {
        assert_eq!(NativeRuntime::new().has_shell_access(), detect_native_shell().is_some());
    }

    /// 测试文件系统访问能力
    ///
    /// 原生运行时直接运行在宿主机上，因此始终具备完整的文件系统访问权限。
    /// 验证 `has_filesystem_access()` 始终返回 `true`。
    #[test]
    fn native_has_filesystem_access() {
        assert!(NativeRuntime::new().has_filesystem_access());
    }

    /// 测试长时间运行任务的支持
    ///
    /// 原生运行时不受容器或沙箱的时间/资源限制，可以执行长时间运行的任务。
    /// 验证 `supports_long_running()` 返回 `true`。
    #[test]
    fn native_supports_long_running() {
        assert!(NativeRuntime::new().supports_long_running());
    }

    /// 测试内存预算限制
    ///
    /// 原生运行时使用值 `0` 表示无内存限制（unlimited）。
    /// 与容器运行时不同，原生运行时直接使用宿主机内存资源。
    #[test]
    fn native_memory_budget_unlimited() {
        assert_eq!(NativeRuntime::new().memory_budget(), 0);
    }

    /// 测试存储路径包含项目标识
    ///
    /// 验证运行时的存储路径中包含 `"vibewindow"` 字符串，
    /// 确保应用数据存储在专用的目录下，避免与其他应用冲突。
    #[test]
    fn native_storage_path_contains_vibewindow() {
        let path = NativeRuntime::new().storage_path();
        assert!(path.to_string_lossy().contains("vibewindow"));
    }

    #[test]
    fn storage_path_from_home_dir_uses_home_when_available_or_relative_fallback() {
        let home = Path::new("/tmp/vibewindow-home");

        assert_eq!(
            storage_path_from_home_dir(Some(home)),
            vw_config_types::paths::home_config_dir(home)
        );
        assert_eq!(
            storage_path_from_home_dir(None),
            PathBuf::from(vw_config_types::paths::HOME_CONFIG_DIR_NAME)
        );
    }

    #[test]
    fn selected_shell_accessors_return_injected_shell_metadata() {
        let runtime = NativeRuntime::new_for_test(Some(ShellProgram {
            kind: ShellKind::Sh,
            program: PathBuf::from("/bin/sh"),
        }));

        assert_eq!(runtime.selected_shell_kind(), Some("sh"));
        assert_eq!(runtime.selected_shell_program(), Some(Path::new("/bin/sh")));
    }

    #[test]
    fn shell_kind_names_cover_every_supported_variant() {
        let cases = [
            (ShellKind::Zsh, "zsh"),
            (ShellKind::Sh, "sh"),
            (ShellKind::Bash, "bash"),
            (ShellKind::Pwsh, "pwsh"),
            (ShellKind::PowerShell, "powershell"),
            (ShellKind::Cmd, "cmd"),
        ];

        for (kind, expected) in cases {
            assert_eq!(kind.as_str(), expected);
        }
    }

    #[test]
    fn shell_kind_launch_styles_match_shell_argument_contracts() {
        let cases = [
            (ShellKind::Zsh, ShellLaunchStyle::ZshLoginInteractive),
            (ShellKind::Sh, ShellLaunchStyle::Posix),
            (ShellKind::Bash, ShellLaunchStyle::BashLogin),
            (ShellKind::Pwsh, ShellLaunchStyle::PowerShell),
            (ShellKind::PowerShell, ShellLaunchStyle::PowerShell),
            (ShellKind::Cmd, ShellLaunchStyle::Cmd),
        ];

        for (kind, expected) in cases {
            assert_eq!(kind.launch_style(), expected);
        }
    }

    #[test]
    fn selected_shell_accessors_return_none_without_shell() {
        let runtime = NativeRuntime::new_for_test(None);

        assert!(runtime.selected_shell_kind().is_none());
        assert!(runtime.selected_shell_program().is_none());
    }

    #[test]
    fn native_as_any_supports_native_runtime_downcast() {
        let runtime = NativeRuntime::new_for_test(None);

        assert!(runtime.as_any().downcast_ref::<NativeRuntime>().is_some());
    }

    /// 测试 Windows 下 Shell 检测优先选择 Git Bash
    ///
    /// 场景：系统中同时存在 Git Bash、PowerShell 和 CMD
    /// 预期：优先检测到 Git Bash（`ShellKind::Bash`）
    ///
    /// 这是 Windows 开发者的常见配置，Git Bash 提供更好的 Unix 兼容性。
    #[test]
    fn detect_shell_windows_prefers_git_bash() {
        // 模拟 Windows 环境下的可执行文件路径映射
        let mut map = HashMap::new();
        map.insert("bash", r"C:\Program Files\Git\bin\bash.exe");
        map.insert("powershell", r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");
        map.insert("cmd", r"C:\Windows\System32\cmd.exe");

        // 注入模拟环境进行测试
        let shell = detect_native_shell_with(
            true,
            |name| map.get(name).map(PathBuf::from),
            Some(PathBuf::from(r"C:\Windows\System32\cmd.exe")),
            None,
        )
        .expect("windows shell should be detected");

        // 应该优先选择 Git Bash
        assert_eq!(shell.kind, ShellKind::Bash);
    }

    #[test]
    fn shell_candidate_lists_preserve_platform_detection_priority() {
        assert_eq!(
            windows_shell_candidates().map(|(name, kind)| (name, kind.as_str())),
            [
                ("bash", "bash"),
                ("sh", "sh"),
                ("pwsh", "pwsh"),
                ("powershell", "powershell"),
                ("cmd", "cmd"),
                ("cmd.exe", "cmd"),
            ]
        );
        assert_eq!(
            unix_shell_candidates().map(|(name, kind)| (name, kind.as_str())),
            [("zsh", "zsh"), ("bash", "bash"), ("sh", "sh")]
        );
    }

    /// 测试 Windows 下 Shell 检测的回退顺序
    ///
    /// 场景 1：不存在 Bash，存在 PowerShell 和 CMD
    /// 预期：检测到 PowerShell（`ShellKind::PowerShell`）
    ///
    /// 场景 2：仅存在 CMD
    /// 预期：回退到 CMD（`ShellKind::Cmd`）
    ///
    /// Windows 下的 Shell 优先级为：Git Bash > PowerShell > CMD
    #[test]
    fn detect_shell_windows_falls_back_to_powershell_then_cmd() {
        // 场景 1：没有 Bash，有 PowerShell
        let mut map = HashMap::new();
        map.insert("powershell", r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");

        let shell = detect_native_shell_with(
            true,
            |name| map.get(name).map(PathBuf::from),
            Some(PathBuf::from(r"C:\Windows\System32\cmd.exe")),
            None,
        )
        .expect("windows shell should be detected");

        assert_eq!(shell.kind, ShellKind::PowerShell);

        // 场景 2：没有 Bash 和 PowerShell，仅有 CMD 作为最终回退
        let cmd_shell = detect_native_shell_with(
            true,
            |_name| None,
            Some(PathBuf::from(r"C:\Windows\System32\cmd.exe")),
            None,
        )
        .expect("cmd fallback should be detected");
        assert_eq!(cmd_shell.kind, ShellKind::Cmd);
    }

    #[test]
    fn detect_shell_windows_prefers_pwsh_over_windows_powershell() {
        let mut map = HashMap::new();
        map.insert("pwsh", r"C:\Program Files\PowerShell\7\pwsh.exe");
        map.insert("powershell", r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");

        let shell = detect_native_shell_with(
            true,
            |name| map.get(name).map(PathBuf::from),
            Some(PathBuf::from(r"C:\Windows\System32\cmd.exe")),
            None,
        )
        .expect("pwsh should be detected");

        assert_eq!(shell.kind, ShellKind::Pwsh);
    }

    #[test]
    fn detect_shell_windows_prefers_sh_over_powershell_variants() {
        let mut map = HashMap::new();
        map.insert("sh", r"C:\Program Files\Git\bin\sh.exe");
        map.insert("pwsh", r"C:\Program Files\PowerShell\7\pwsh.exe");
        map.insert("powershell", r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");

        let shell = detect_native_shell_with(
            true,
            |name| map.get(name).map(PathBuf::from),
            Some(PathBuf::from(r"C:\Windows\System32\cmd.exe")),
            None,
        )
        .expect("sh should be detected before PowerShell variants");

        assert_eq!(shell.kind, ShellKind::Sh);
        assert_eq!(shell.program, PathBuf::from(r"C:\Program Files\Git\bin\sh.exe"));
    }

    #[test]
    fn detect_shell_windows_uses_cmd_exe_alias_before_comspec() {
        let shell = detect_native_shell_with(
            true,
            |name| (name == "cmd.exe").then(|| PathBuf::from(r"C:\Tools\cmd.exe")),
            Some(PathBuf::from(r"C:\Windows\System32\cmd.exe")),
            None,
        )
        .expect("cmd.exe alias should be detected");

        assert_eq!(shell.kind, ShellKind::Cmd);
        assert_eq!(shell.program, PathBuf::from(r"C:\Tools\cmd.exe"));
    }

    #[test]
    fn detect_shell_windows_returns_none_without_candidates_or_comspec() {
        let shell = detect_native_shell_with(true, |_name| None, None, None);

        assert!(shell.is_none());
    }

    /// 测试 Windows 下排除 System32 中的 WSL bash.exe
    ///
    /// Windows System32 目录下的 `bash.exe` 是 WSL 的启动器，
    /// 而非真正的 bash shell。检测逻辑应该跳过它，选择其他可用 Shell。
    ///
    /// 场景：仅存在 WSL bash（System32）、PowerShell、CMD
    /// 预期：选择 PowerShell，而非 WSL bash
    #[test]
    fn detect_shell_windows_skips_system32_bash_wsl_launcher() {
        // 模拟仅有 WSL bash 的场景
        let mut map = HashMap::new();
        map.insert("bash", r"C:\Windows\System32\bash.exe"); // 这是 WSL 启动器，应被跳过
        map.insert("powershell", r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe");
        map.insert("cmd", r"C:\Windows\System32\cmd.exe");

        let shell = detect_native_shell_with(
            true,
            |name| map.get(name).map(PathBuf::from),
            Some(PathBuf::from(r"C:\Windows\System32\cmd.exe")),
            None,
        )
        .expect("windows shell should be detected");

        // 应该跳过 WSL bash，选择 PowerShell
        assert_eq!(shell.kind, ShellKind::PowerShell);
        assert_eq!(
            shell.program,
            PathBuf::from(r"C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe")
        );
    }

    /// 测试 Windows 下仅存在 WSL bash 时使用 CMD 作为回退
    ///
    /// 场景：仅存在 WSL bash（位于 Sysnative 目录）和 CMD
    /// 预期：由于 WSL bash 被识别为启动器而被跳过，最终选择 CMD
    ///
    /// Sysnative 目录是 32 位进程访问 64 位 System32 的重定向路径。
    #[test]
    fn detect_shell_windows_uses_cmd_when_only_wsl_bash_exists() {
        let mut map = HashMap::new();
        map.insert("bash", r"C:\Windows\Sysnative\bash.exe"); // WSL 启动器路径

        let shell = detect_native_shell_with(
            true,
            |name| map.get(name).map(PathBuf::from),
            Some(PathBuf::from(r"C:\Windows\System32\cmd.exe")),
            None,
        )
        .expect("cmd fallback should be detected");

        // 应该跳过 WSL bash，回退到 CMD
        assert_eq!(shell.kind, ShellKind::Cmd);
        assert_eq!(shell.program, PathBuf::from(r"C:\Windows\System32\cmd.exe"));
    }

    /// 测试 WSL 启动器路径识别函数
    ///
    /// `is_windows_wsl_bash_launcher` 函数用于判断给定的 bash 路径是否为 WSL 启动器。
    ///
    /// - 应识别为 WSL 启动器的路径：
    ///   - `C:\Windows\System32\bash.exe`
    ///   - `C:\Windows\Sysnative\bash.exe`
    /// - 不应识别为 WSL 启动器的路径：
    ///   - `C:\Program Files\Git\bin\bash.exe`（Git Bash，真正的 bash）
    #[test]
    fn wsl_launcher_detection_matches_known_paths() {
        // WSL 启动器路径应返回 true
        assert!(is_windows_wsl_bash_launcher(Path::new(r"C:\Windows\System32\bash.exe")));
        assert!(is_windows_wsl_bash_launcher(Path::new(r"C:\Windows\Sysnative\bash.exe")));
        // Git Bash 路径应返回 false
        assert!(!is_windows_wsl_bash_launcher(Path::new(r"C:\Program Files\Git\bin\bash.exe")));
    }

    #[test]
    fn wsl_launcher_detection_normalizes_case_and_path_separators() {
        assert!(is_windows_wsl_bash_launcher(Path::new("C:/WINDOWS/System32/BASH.EXE")));
        assert!(is_windows_wsl_bash_launcher(Path::new("D:/Windows/Sysnative/bash.exe")));
        assert!(!is_windows_wsl_bash_launcher(Path::new("C:/Windows/System32/bash.exe.backup")));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn resolve_shell_on_path_returns_none_for_missing_executable_name() {
        assert!(resolve_shell_on_path("__vibewindow_missing_shell_for_native_runtime__").is_none());
    }

    /// 测试 Unix 系统下优先使用用户默认 Shell
    ///
    /// Unix 系统通过环境变量 `$SHELL` 指定用户的默认 Shell。
    /// 检测逻辑应该优先使用这个值。
    ///
    /// 场景：用户默认 Shell 为 `/bin/zsh`
    /// 预期：检测到 Zsh（`ShellKind::Zsh`），程序路径为 `/bin/zsh`
    #[test]
    fn detect_shell_unix_prefers_user_shell() {
        let shell =
            detect_native_shell_with(false, |_name| None, None, Some(PathBuf::from("/bin/zsh")))
                .expect("unix user shell should be detected");

        assert_eq!(shell.kind, ShellKind::Zsh);
        assert_eq!(shell.program, PathBuf::from("/bin/zsh"));
    }

    #[test]
    fn detect_shell_unix_prefers_existing_supported_user_shell_over_resolver() {
        let dir = tempfile::tempdir().unwrap();
        let user_shell = dir.path().join("custom-bash");
        std::fs::write(&user_shell, b"").unwrap();

        let shell = detect_native_shell_with(
            false,
            |name| (name == "zsh").then(|| PathBuf::from("/fallback/zsh")),
            None,
            Some(user_shell.clone()),
        )
        .expect("existing supported user shell should be selected");

        assert_eq!(shell.kind, ShellKind::Bash);
        assert_eq!(shell.program, user_shell);
    }

    /// 测试 Unix 系统下 Shell 检测的优先级回退
    ///
    /// 当无法获取用户默认 Shell 时，按优先级搜索可用的 Shell。
    /// Unix 下的优先级为：zsh > bash > sh
    ///
    /// 场景：系统中存在 zsh、sh、bash（均通过 `which` 找到）
    /// 预期：按优先级选择 Zsh
    #[test]
    fn detect_shell_unix_falls_back_by_priority() {
        // 模拟 which 命令的查找结果
        let mut map = HashMap::new();
        map.insert("zsh", "/bin/zsh");
        map.insert("sh", "/bin/sh");
        map.insert("bash", "/usr/bin/bash");

        let shell =
            detect_native_shell_with(false, |name| map.get(name).map(PathBuf::from), None, None)
                .expect("unix shell should be detected");

        // 应该按优先级选择 zsh
        assert_eq!(shell.kind, ShellKind::Zsh);
    }

    #[test]
    fn detect_shell_unix_falls_back_to_bash_when_user_shell_is_missing() {
        let process_id = std::process::id();
        let missing_user_shell = std::env::temp_dir()
            .join(format!("vibewindow-native-runtime-{process_id}"))
            .join("zsh");

        let shell = detect_native_shell_with(
            false,
            |name| (name == "bash").then(|| PathBuf::from("/usr/bin/bash")),
            None,
            Some(missing_user_shell),
        )
        .expect("bash fallback should be detected");

        assert_eq!(shell.kind, ShellKind::Bash);
        assert_eq!(shell.program, PathBuf::from("/usr/bin/bash"));
    }

    #[test]
    fn detect_shell_unix_returns_none_without_user_or_system_shell() {
        let shell = detect_native_shell_with(false, |_name| None, None, None);

        assert!(shell.is_none());
    }

    #[test]
    fn classify_unix_shell_program_handles_case_wrappers_and_missing_names() {
        assert_eq!(
            classify_unix_shell_program(Path::new("/usr/local/bin/ZSH")),
            Some(ShellKind::Zsh)
        );
        assert_eq!(
            classify_unix_shell_program(Path::new("/opt/tools/bash-wrapper")),
            Some(ShellKind::Bash)
        );
        assert_eq!(classify_unix_shell_program(Path::new("/bin/sh")), Some(ShellKind::Sh));
        assert!(classify_unix_shell_program(Path::new("/bin/dash")).is_none());
        assert!(classify_unix_shell_program(Path::new("/")).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn classify_unix_shell_program_rejects_non_utf8_file_name() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;

        let program = PathBuf::from(OsString::from_vec(vec![b'z', b's', b'h', 0xff]));

        assert!(classify_unix_shell_program(&program).is_none());
    }

    #[test]
    fn unix_user_shell_is_ignored_when_unknown_or_missing() {
        let shell = detect_native_shell_with(
            false,
            |name| (name == "sh").then(|| PathBuf::from("/bin/sh")),
            None,
            Some(PathBuf::from("/tmp/not-a-supported-shell")),
        )
        .expect("fallback shell should be detected");

        assert_eq!(shell.kind, ShellKind::Sh);
        assert_eq!(classify_unix_shell_program(Path::new("/bin/fish")), None);
        assert_eq!(
            classify_unix_shell_program(Path::new("/usr/local/bin/bash")),
            Some(ShellKind::Bash)
        );
    }

    /// 测试无可用 Shell 时的行为
    ///
    /// 场景：创建一个没有配置 Shell 的测试用运行时实例
    /// 预期：
    /// - `has_shell_access()` 返回 `false`
    /// - 调用 `build_shell_command` 返回错误，错误信息包含 "could not find a usable shell"
    ///
    /// 这验证了运行时在没有 Shell 时的安全失败行为。
    #[test]
    fn native_without_shell_disables_shell_access() {
        let runtime = NativeRuntime::new_for_test(None);
        assert!(!runtime.has_shell_access());

        let err = runtime
            .build_shell_command("echo hello", Path::new("."))
            .expect_err("build should fail without available shell")
            .to_string();
        assert!(err.contains("could not find a usable shell"));
    }

    /// 测试 PowerShell 命令构建
    ///
    /// 验证为 PowerShell 构建的命令包含正确的参数：
    /// - 程序名：`powershell`
    /// - `-NoProfile`：不加载用户配置文件，加快启动速度
    /// - `-Command`：指定要执行的命令
    /// - 命令内容：`Get-Location`
    #[test]
    fn native_builds_powershell_command() {
        let runtime = NativeRuntime::new_for_test(Some(ShellProgram {
            kind: ShellKind::PowerShell,
            program: PathBuf::from("powershell"),
        }));

        let command = runtime
            .build_shell_command("Get-Location", Path::new("."))
            .expect("powershell command should build");
        let debug = format!("{command:?}");

        assert!(debug.contains("powershell"));
        assert!(debug.contains("-NoProfile"));
        assert!(debug.contains("-Command"));
        assert!(debug.contains("Get-Location"));
    }

    /// 测试 CMD 命令构建
    ///
    /// 验证为 CMD 构建的命令包含正确的参数：
    /// - 程序名：`cmd`
    /// - `/C`：执行字符串指定的命令然后终止
    /// - 命令内容：`echo hello`
    #[test]
    fn native_builds_cmd_command() {
        let runtime = NativeRuntime::new_for_test(Some(ShellProgram {
            kind: ShellKind::Cmd,
            program: PathBuf::from("cmd"),
        }));

        let command = runtime
            .build_shell_command("echo hello", Path::new("."))
            .expect("cmd command should build");
        let debug = format!("{command:?}");

        assert!(debug.contains("cmd"));
        assert!(debug.contains("/C"));
        assert!(debug.contains("echo hello"));
    }

    /// 测试 Zsh 命令构建（包含登录和交互标志）
    ///
    /// 验证为 Zsh 构建的命令包含正确的参数：
    /// - 程序路径：`/bin/zsh`
    /// - `-l`：作为登录 Shell 启动，加载 `.zprofile` 等配置
    /// - `-i`：作为交互式 Shell 启动，加载 `.zshrc` 等配置
    /// - 命令内容：`echo hello`
    ///
    /// 这确保 Zsh 环境下用户的别名、函数等配置可以被正确加载。
    #[test]
    fn native_builds_zsh_command_with_login_and_interactive_flags() {
        let runtime = NativeRuntime::new_for_test(Some(ShellProgram {
            kind: ShellKind::Zsh,
            program: PathBuf::from("/bin/zsh"),
        }));

        let command = runtime
            .build_shell_command("echo hello", Path::new("."))
            .expect("zsh command should build");
        let debug = format!("{command:?}");

        assert!(debug.contains("/bin/zsh"));
        assert!(debug.contains("-l"));
        assert!(debug.contains("-i"));
        assert!(debug.contains("echo hello"));
    }

    /// 测试 Bash 命令构建（包含登录标志）
    ///
    /// 验证为 Bash 构建的命令包含正确的参数：
    /// - 程序路径：`/bin/bash`
    /// - `-l`：作为登录 Shell 启动，加载 `.bash_profile` 等配置
    /// - `-c`：从字符串读取命令执行
    /// - 命令内容：`echo hello`
    ///
    /// 这确保 Bash 环境下用户的 PATH、别名等配置可以被正确加载。
    #[test]
    fn native_builds_bash_command_with_login_flag() {
        let runtime = NativeRuntime::new_for_test(Some(ShellProgram {
            kind: ShellKind::Bash,
            program: PathBuf::from("/bin/bash"),
        }));

        let command = runtime
            .build_shell_command("echo hello", Path::new("."))
            .expect("bash command should build");
        let debug = format!("{command:?}");

        assert!(debug.contains("/bin/bash"));
        assert!(debug.contains("-l"));
        assert!(debug.contains("-c"));
        assert!(debug.contains("echo hello"));
    }

    #[test]
    fn native_builds_sh_and_pwsh_commands() {
        let sh_runtime = NativeRuntime::new_for_test(Some(ShellProgram {
            kind: ShellKind::Sh,
            program: PathBuf::from("/bin/sh"),
        }));
        let sh = sh_runtime.build_shell_command("echo sh", Path::new(".")).unwrap();
        let sh_debug = format!("{sh:?}");
        assert!(sh_debug.contains("/bin/sh"));
        assert!(sh_debug.contains("-c"));
        assert!(sh_debug.contains("echo sh"));
        assert!(!sh_debug.contains("-l"));

        let pwsh_runtime = NativeRuntime::new_for_test(Some(ShellProgram {
            kind: ShellKind::Pwsh,
            program: PathBuf::from("pwsh"),
        }));
        let pwsh = pwsh_runtime.build_shell_command("Get-Date", Path::new(".")).unwrap();
        let pwsh_debug = format!("{pwsh:?}");
        assert!(pwsh_debug.contains("pwsh"));
        assert!(pwsh_debug.contains("-NoLogo"));
        assert!(pwsh_debug.contains("-NonInteractive"));
        assert!(pwsh_debug.contains("Get-Date"));
    }

    #[test]
    fn native_build_shell_command_preserves_program_arguments_and_workspace() {
        use std::ffi::OsStr;

        let runtime = NativeRuntime::new_for_test(Some(ShellProgram {
            kind: ShellKind::Sh,
            program: PathBuf::from("custom-sh"),
        }));
        let workspace = std::env::temp_dir().join("vibewindow-native-runtime-workspace");

        let command = runtime
            .build_shell_command("printf 'hello world'", &workspace)
            .expect("sh command should build");
        let command = command.as_std();

        assert_eq!(command.get_program(), OsStr::new("custom-sh"));
        let args = command.get_args().collect::<Vec<_>>();
        assert_eq!(args.len(), 2);
        assert_eq!(args[0], OsStr::new("-c"));
        assert_eq!(args[1], OsStr::new("printf 'hello world'"));
        assert_eq!(command.get_current_dir(), Some(workspace.as_path()));
    }

    /// 测试 Shell 命令构建（集成测试）
    ///
    /// 使用真实的运行时实例构建命令，验证命令字符串中包含预期的命令内容。
    /// 如果当前环境没有可用的 Shell，测试会直接跳过。
    ///
    /// 这是一个轻量级的集成测试，验证命令构建流程端到端可用。
    #[test]
    fn native_builds_shell_command() {
        let runtime = NativeRuntime::new();
        // 无 Shell 时跳过测试
        if !runtime.has_shell_access() {
            return;
        }

        let cwd = std::env::temp_dir();
        let command = runtime.build_shell_command("echo hello", &cwd).unwrap();
        let debug = format!("{command:?}");
        assert!(debug.contains("echo hello"));
    }
}
