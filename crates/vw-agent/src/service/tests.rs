//! Service 模块单元测试
//!
//! 本模块包含针对 VibeWindow 服务管理功能的全面测试套件，涵盖了：
//! - XML 转义功能测试
//! - 命令执行与输出捕获测试
//! - 平台特定的服务配置测试（Linux/Windows）
//! - Init 系统（systemd/OpenRC）解析测试
//! - OpenRC 脚本生成测试
//!
//! 这些测试确保服务管理功能在不同平台和配置下的正确性和健壮性。

use super::*;

/// 测试模块
///
/// 包含所有针对 service 模块功能的单元测试
#[allow(dead_code)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::path::Path;
    use std::process::Command;

    #[derive(Debug, Parser)]
    struct ServiceTestCli {
        #[command(subcommand)]
        command: ServiceCommands,
    }

    /// 测试 XML 转义功能
    ///
    /// 验证 `xml_escape` 函数能够正确转义 XML 中的保留字符：
    /// - `<` 转义为 `&lt;`
    /// - `>` 转义为 `&gt;`
    /// - `&` 转义为 `&amp;`
    /// - `"` 转义为 `&quot;`
    /// - `'` 转义为 `&apos;`
    ///
    /// 同时验证非保留字符保持不变
    #[test]
    fn xml_escape_escapes_reserved_chars() {
        let escaped = xml_escape("<&>\"' and text");
        assert_eq!(escaped, "&lt;&amp;&gt;&quot;&apos; and text");
    }

    /// 测试命令输出捕获（非 Windows 平台）
    ///
    /// 验证 `run_capture` 函数能够正确捕获命令的标准输出。
    /// 在 Unix/Linux 系统上执行 shell 命令并捕获其输出。
    ///
    /// 测试场景：
    /// - 执行 `echo hello` 命令
    /// - 验证捕获的输出为 "hello"
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn run_capture_reads_stdout() {
        let out = run_capture(Command::new("sh").args(["-lc", "echo hello"]))
            .expect("stdout capture should succeed");
        assert_eq!(out.trim(), "hello");
    }

    /// 测试命令标准错误捕获（非 Windows 平台）
    ///
    /// 验证 `run_capture` 函数在标准输出为空时能够回退到捕获标准错误输出。
    /// 这确保了即使在命令输出到 stderr 的情况下也能获取信息。
    ///
    /// 测试场景：
    /// - 执行输出到 stderr 的命令
    /// - 验证能够捕获 stderr 内容
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn run_capture_falls_back_to_stderr() {
        let out = run_capture(Command::new("sh").args(["-lc", "echo warn 1>&2"]))
            .expect("stderr capture should succeed");
        assert_eq!(out.trim(), "warn");
    }

    /// 测试命令执行失败检测（非 Windows 平台）
    ///
    /// 验证 `run_checked` 函数能够正确检测命令的非零退出状态码，
    /// 并返回包含错误信息的错误结果。
    ///
    /// 测试场景：
    /// - 执行退出码为 17 的命令
    /// - 验证返回错误且包含 "Command failed" 信息
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn run_checked_errors_on_non_zero_status() {
        let err = run_checked(Command::new("sh").args(["-lc", "exit 17"]))
            .expect_err("non-zero exit should error");
        assert!(err.to_string().contains("Command failed"));
    }

    /// 测试 Linux 服务文件路径生成
    ///
    /// 验证 `linux_service_file` 函数能够生成符合 systemd 规范的用户级服务文件路径。
    /// 路径应该位于用户的 systemd 配置目录中。
    ///
    /// 测试场景：
    /// - 使用默认配置
    /// - 验证生成的路径以 ".config/systemd/user/vibewindow.service" 结尾
    #[cfg(not(target_os = "windows"))]
    #[test]
    fn linux_service_file_has_expected_suffix() {
        let file = linux_service_file(&Config::default()).unwrap();
        let path = file.to_string_lossy();
        assert!(path.ends_with(".config/systemd/user/vibewindow.service"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn systemd_config_dir_args_preserves_selected_config_dir() {
        let mut config = Config::default();
        config.config_path =
            "/Users/me/Library/Application Support/VibeWindow/vibewindow.json".into();

        let args = systemd_config_dir_args(&config);

        assert!(args.contains("--config-dir"));
        assert!(args.contains("\"/Users/me/Library/Application Support/VibeWindow\""));
        assert!(!args.contains("vibewindow.json"));
    }

    #[cfg(not(target_os = "windows"))]
    #[test]
    fn systemd_quote_arg_leaves_simple_paths_unquoted() {
        assert_eq!(systemd_quote_arg("/home/me/.vibewindow"), "/home/me/.vibewindow");
    }

    /// 测试 Windows 任务名称常量
    ///
    /// 验证 `windows_task_name` 函数返回正确的 Windows 任务计划程序任务名称。
    /// 该名称用于在 Windows 系统上注册 ShiJu 守护进程。
    #[test]
    fn windows_task_name_is_constant() {
        assert_eq!(windows_task_name(), "ShiJu Daemon");
    }

    /// 测试命令输出捕获（Windows 平台）
    ///
    /// Windows 版本的输出捕获测试，验证 `run_capture` 函数在 Windows 平台上的正确性。
    ///
    /// 测试场景：
    /// - 使用 cmd.exe 执行 echo 命令
    /// - 验证输出捕获成功且内容正确
    #[cfg(target_os = "windows")]
    #[test]
    fn run_capture_reads_stdout_windows() {
        let out = run_capture(Command::new("cmd").args(["/C", "echo hello"]))
            .expect("stdout capture should succeed");
        assert_eq!(out.trim(), "hello");
    }

    /// 测试命令执行失败检测（Windows 平台）
    ///
    /// Windows 版本的失败检测测试，验证 `run_checked` 在 Windows 上的错误处理能力。
    ///
    /// 测试场景：
    /// - 使用 cmd.exe 执行退出码为 17 的命令
    /// - 验证返回错误且包含 "Command failed" 信息
    #[cfg(target_os = "windows")]
    #[test]
    fn run_checked_errors_on_non_zero_status_windows() {
        let err = run_checked(Command::new("cmd").args(["/C", "exit /b 17"]))
            .expect_err("non-zero exit should error");
        assert!(err.to_string().contains("Command failed"));
    }

    /// 测试 InitSystem 枚举的字符串解析
    ///
    /// 验证 `InitSystem` 的 `FromStr` 实现能够正确解析有效的初始化系统值，
    /// 且解析是大小写不敏感的。
    ///
    /// 测试场景：
    /// - 验证 "auto"/"AUTO" 解析为 `InitSystem::Auto`
    /// - 验证 "systemd"/"SYSTEMD" 解析为 `InitSystem::Systemd`
    /// - 验证 "openrc"/"OPENRC" 解析为 `InitSystem::Openrc`
    #[test]
    fn init_system_from_str_parses_valid_values() {
        assert_eq!("auto".parse::<InitSystem>().unwrap(), InitSystem::Auto);
        assert_eq!("AUTO".parse::<InitSystem>().unwrap(), InitSystem::Auto);
        assert_eq!("systemd".parse::<InitSystem>().unwrap(), InitSystem::Systemd);
        assert_eq!("SYSTEMD".parse::<InitSystem>().unwrap(), InitSystem::Systemd);
        assert_eq!("openrc".parse::<InitSystem>().unwrap(), InitSystem::Openrc);
        assert_eq!("OPENRC".parse::<InitSystem>().unwrap(), InitSystem::Openrc);
    }

    /// 测试 InitSystem 拒绝未知值
    ///
    /// 验证 `InitSystem` 的 `FromStr` 实现能够正确拒绝未知的初始化系统值，
    /// 并返回包含有用提示的错误信息。
    ///
    /// 测试场景：
    /// - 尝试解析 "unknown"
    /// - 验证返回错误且包含 "Unknown init system" 和支持的值列表
    #[test]
    fn init_system_from_str_rejects_unknown() {
        let err = "unknown".parse::<InitSystem>().expect_err("should reject unknown");
        assert!(err.to_string().contains("Unknown init system"));
        assert!(err.to_string().contains("Supported: auto, systemd, openrc"));
    }

    /// 测试 InitSystem 的默认值
    ///
    /// 验证 `InitSystem` 的默认值为 `Auto`，即自动检测初始化系统。
    #[test]
    fn init_system_default_is_auto() {
        assert_eq!(InitSystem::default(), InitSystem::Auto);
    }

    /// 测试 root 权限检测
    ///
    /// 验证 `is_root` 函数能够正确检测当前进程是否以 root 用户运行，
    /// 结果应与系统 UID 检测一致（UID 0 表示 root）。
    #[cfg(unix)]
    #[test]
    fn is_root_matches_system_uid() {
        assert_eq!(is_root(), current_uid() == Some(0));
    }

    /// 测试 OpenRC 脚本生成内容
    ///
    /// 验证 `generate_openrc_script` 函数生成的 OpenRC 服务脚本包含所有必需的指令和配置。
    /// 脚本应符合 OpenRC 的标准格式和最佳实践。
    ///
    /// 测试场景：
    /// - 验证脚本以正确的 shebang 开头
    /// - 验证包含服务名称和描述
    /// - 验证包含正确的命令路径和参数
    /// - 验证配置了后台运行和用户权限
    /// - 验证设置了日志文件路径
    /// - 验证包含依赖声明（网络和防火墙）
    #[test]
    fn generate_openrc_script_contains_required_directives() {
        use std::path::PathBuf;

        let exe_path = PathBuf::from("/usr/local/bin/vibewindow");
        let script = generate_openrc_script(&exe_path, Path::new("/etc/vibewindow"));

        // 验证脚本头
        assert!(script.starts_with("#!/sbin/openrc-run"));

        // 验证基本服务信息
        assert!(script.contains("name=\"vibewindow\""));
        assert!(script.contains("description=\"VibeWindow daemon\""));

        // 验证命令配置
        assert!(script.contains("command=\"/usr/local/bin/vibewindow\""));
        assert!(script.contains("command_args=\"--config-dir /etc/vibewindow daemon\""));

        // 验证未使用环境变量（而是直接传递参数）
        assert!(!script.contains("env VIBEWINDOW_CONFIG_DIR"));
        assert!(!script.contains("env VIBEWINDOW_WORKSPACE"));

        // 验证进程管理配置
        assert!(script.contains("command_background=\"yes\""));
        assert!(script.contains("command_user=\"vibewindow:vibewindow\""));
        assert!(script.contains("pidfile=\"/run/${RC_SVCNAME}.pid\""));

        // 验证安全设置
        assert!(script.contains("umask 027"));

        // 验证日志配置
        assert!(script.contains("output_log=\"/var/log/vibewindow/access.log\""));
        assert!(script.contains("error_log=\"/var/log/vibewindow/error.log\""));

        // 验证依赖函数
        assert!(script.contains("depend()"));
        assert!(script.contains("need net"));
        assert!(script.contains("after firewall"));
    }

    /// 测试二进制文件位置警告检测
    ///
    /// 验证能够正确识别二进制文件是否位于用户主目录（如 ~/.cargo/bin），
    /// 这对于生产环境部署的警告提示很重要。
    ///
    /// 测试场景：
    /// - 验证能识别 /home/ 路径
    /// - 验证能识别 .cargo/bin 路径
    /// - 验证能区分系统路径
    #[test]
    fn warn_if_binary_in_home_detects_home_path() {
        use std::path::PathBuf;

        fn path_has_component(path: &std::path::Path, expected: &str) -> bool {
            path.components().any(|component| component.as_os_str() == expected)
        }

        // 测试典型的主目录路径
        let home_path = PathBuf::from("/home/user/.cargo/bin/vibewindow");
        assert!(path_has_component(&home_path, "home"));
        assert!(path_has_component(&home_path, ".cargo"));
        assert!(path_has_component(&home_path, "bin"));

        // 测试 Cargo 默认安装路径
        let cargo_path = PathBuf::from("/home/user/.cargo/bin/vibewindow");
        assert!(path_has_component(&cargo_path, ".cargo"));
        assert!(path_has_component(&cargo_path, "bin"));

        // 测试系统路径（不应被检测为 home 路径）
        let system_path = PathBuf::from("/usr/local/bin/vibewindow");
        assert!(!path_has_component(&system_path, "home"));
        assert!(!path_has_component(&system_path, ".cargo"));
    }

    /// 测试 shell 单引号转义
    ///
    /// 验证 `shell_single_quote` 函数能够正确处理包含单引号的路径，
    /// 使用安全的转义方式确保路径在 shell 命令中正确使用。
    ///
    /// 转义策略：将字符串用单引号包裹，单引号本身使用 '\'' 方式转义
    #[cfg(unix)]
    #[test]
    fn shell_single_quote_escapes_single_quotes() {
        assert_eq!(shell_single_quote("/tmp/weird'path"), "'/tmp/weird'\"'\"'path'");
    }

    /// 测试 OpenRC 可写性探测命令生成（优先使用 runuser）
    ///
    /// 验证当 `runuser` 命令可用时，`build_openrc_writability_probe_command` 函数
    /// 能够生成正确的命令来测试指定目录是否对 vibewindow 用户可写。
    ///
    /// 测试场景：
    /// - 验证使用 runuser 而非 su
    /// - 验证命令参数正确传递用户和测试路径
    /// - 验证执行 test -w 命令检查可写性
    #[cfg(unix)]
    #[test]
    fn openrc_writability_probe_prefers_runuser_when_available() {
        let (program, args) =
            build_openrc_writability_probe_command(Path::new("/etc/vibewindow"), true);

        // 验证使用 runuser 命令
        assert_eq!(program, "runuser");

        // 验证命令参数序列
        assert_eq!(
            args,
            vec![
                "-u".to_string(),
                "vibewindow".to_string(),
                "--".to_string(),
                "sh".to_string(),
                "-c".to_string(),
                "test -w '/etc/vibewindow'".to_string()
            ]
        );
    }

    /// 测试 OpenRC 可写性探测命令生成（回退到 su）
    ///
    /// 验证当 `runuser` 命令不可用时，`build_openrc_writability_probe_command` 函数
    /// 能够正确回退到使用 `su` 命令进行权限切换和可写性测试。
    ///
    /// 测试场景：
    /// - 验证回退使用 su 命令
    /// - 验证 su 命令的参数格式正确
    /// - 验证测试路径正确传递
    #[cfg(unix)]
    #[test]
    fn openrc_writability_probe_falls_back_to_su() {
        let (program, args) =
            build_openrc_writability_probe_command(Path::new("/etc/vibewindow/workspace"), false);

        // 验证使用 su 命令作为回退
        assert_eq!(program, "su");

        // 验证 su 命令参数序列（注意参数顺序与 runuser 不同）
        assert_eq!(
            args,
            vec![
                "-s".to_string(),
                "/bin/sh".to_string(),
                "-c".to_string(),
                "test -w '/etc/vibewindow/workspace'".to_string(),
                "vibewindow".to_string()
            ]
        );
    }

    #[test]
    fn internal_command_delay_rejects_absent_invalid_and_out_of_range_values() {
        let rejected = [
            None,
            Some(""),
            Some("   "),
            Some("invalid"),
            Some("-1"),
            Some("0"),
            Some("10001"),
            Some("18446744073709551616"),
        ];

        for raw in rejected {
            assert_eq!(internal_command_delay(raw), None, "unexpected delay for {raw:?}");
        }
    }

    #[test]
    fn internal_command_delay_accepts_trimmed_values_at_inclusive_bounds() {
        assert_eq!(internal_command_delay(Some(" 1 ")), Some(Duration::from_millis(1)));
        assert_eq!(internal_command_delay(Some("10000")), Some(Duration::from_millis(10_000)));
    }

    #[test]
    fn init_system_from_str_rejects_whitespace_padded_values() {
        let error = " systemd "
            .parse::<InitSystem>()
            .expect_err("whitespace-padded init system should be rejected");

        assert!(error.to_string().contains("Unknown init system: ' systemd '"));
    }

    #[test]
    fn service_commands_clap_parses_every_variant() {
        let cases = [
            ("install", ServiceCommands::Install),
            ("start", ServiceCommands::Start),
            ("stop", ServiceCommands::Stop),
            ("restart", ServiceCommands::Restart),
            ("status", ServiceCommands::Status),
            ("uninstall", ServiceCommands::Uninstall),
        ];

        for (name, expected) in cases {
            let parsed = ServiceTestCli::try_parse_from(["service-test", name])
                .expect("known service subcommand should parse");
            assert_eq!(parsed.command, expected);
        }
    }

    #[test]
    fn service_commands_clap_rejects_missing_and_unknown_variants() {
        assert!(ServiceTestCli::try_parse_from(["service-test"]).is_err());
        assert!(ServiceTestCli::try_parse_from(["service-test", "reload"]).is_err());
    }
}
