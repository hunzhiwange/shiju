//! Firejail 沙箱隔离功能的单元测试模块
//!
//! 本模块包含针对 FirejailSandbox 实现的所有测试用例，
//! 验证沙箱的基本功能、命令包装机制以及安全标志的完整性。
//!
//! ## 测试覆盖范围
//!
//! - 沙箱名称与描述验证
//! - 安装检测与错误处理
//! - 命令包装功能
//! - 安全隔离标志完整性
//! - 原始命令参数保留

use super::*;

/// 测试模块内部定义
#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试 FirejailSandbox 的名称标识
    ///
    /// 验证沙箱实现返回的名称字符串是否为 "firejail"。
    /// 这个名称用于在系统中标识和区分不同的沙箱后端。
    #[test]
    fn firejail_sandbox_name() {
        assert_eq!(FirejailSandbox.name(), "firejail");
    }

    /// 测试沙箱描述中是否提及依赖项
    ///
    /// 验证 FirejailSandbox 的描述文本中包含 "firejail" 关键字，
    /// 确保用户能够清楚了解该沙箱依赖的外部工具。
    #[test]
    fn firejail_description_mentions_dependency() {
        let desc = FirejailSandbox.description();
        assert!(desc.contains("firejail"));
    }

    /// 测试在未安装 firejail 时的构造失败行为
    ///
    /// 当系统中未安装 firejail 可执行文件时，
    /// `FirejailSandbox::new()` 应返回适当的错误类型。
    ///
    /// ## 错误类型
    ///
    /// - `std::io::ErrorKind::NotFound`: firejail 可执行文件未找到
    /// - `std::io::ErrorKind::Unsupported`: 当前平台不支持
    #[test]
    fn firejail_new_fails_if_not_installed() {
        let result = FirejailSandbox::new();
        match result {
            Ok(_) => println!("Firejail is installed"),
            Err(e) => assert!(
                e.kind() == std::io::ErrorKind::NotFound
                    || e.kind() == std::io::ErrorKind::Unsupported
            ),
        }
    }

    /// 测试命令包装功能是否正确添加 firejail 前缀
    ///
    /// 验证 `wrap_command` 方法能够将普通命令包装为
    /// 以 firejail 开头的沙箱化命令。
    ///
    /// ## 行为说明
    ///
    /// - 如果 firejail 未安装，wrap_command 可能失败
    /// - 如果 firejail 已安装，命令程序应变为 "firejail"
    #[test]
    fn firejail_wrap_command_prepends_firejail() {
        let sandbox = FirejailSandbox;
        let mut cmd = Command::new("echo");
        cmd.arg("test");

        let _ = sandbox.wrap_command(&mut cmd);

        if sandbox.is_available() {
            assert_eq!(cmd.get_program().to_string_lossy(), "firejail");
        }
    }

    // ── §1.1 沙箱隔离标志测试 ─────────────────────────────────────

    /// 测试命令包装是否包含所有必需的安全隔离标志
    ///
    /// 验证经过 wrap_command 处理后的命令包含完整的
    /// 安全隔离参数集，确保沙箱提供预期的隔离级别。
    ///
    /// ## 预期的安全标志
    ///
    /// - `--private=home`: 私有主目录隔离
    /// - `--private-dev`: 私有设备文件系统
    /// - `--nosound`: 禁用音频设备访问
    /// - `--no3d`: 禁用 3D 硬件加速
    /// - `--novideo`: 禁用视频设备访问
    /// - `--nowheel`: 禁用滚轮/鼠标设备
    /// - `--notv`: 禁用电视调谐器设备
    /// - `--noprofile`: 不使用默认配置文件
    /// - `--quiet`: 静默模式运行
    #[test]
    fn firejail_wrap_command_includes_all_security_flags() {
        let sandbox = FirejailSandbox;
        let mut cmd = Command::new("echo");
        cmd.arg("test");
        sandbox.wrap_command(&mut cmd).unwrap();

        assert_eq!(
            cmd.get_program().to_string_lossy(),
            "firejail",
            "wrapped command should use firejail as program"
        );

        // 收集所有命令行参数用于验证
        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

        // 定义所有预期的安全隔离标志
        let expected_flags = [
            "--private=home",
            "--private-dev",
            "--nosound",
            "--no3d",
            "--novideo",
            "--nowheel",
            "--notv",
            "--noprofile",
            "--quiet",
        ];

        // 验证每个安全标志都存在于参数列表中
        for flag in &expected_flags {
            assert!(args.contains(&flag.to_string()), "must include security flag: {flag}");
        }
    }

    /// 测试命令包装是否正确保留原始命令及其参数
    ///
    /// 验证 wrap_command 在添加沙箱包装时，
    /// 不会丢失或篡改原始命令的程序名和参数。
    ///
    /// ## 保留内容
    ///
    /// - 原始程序名（如 "ls"）
    /// - 原始命令行参数（如 "-la", "/workspace"）
    #[test]
    fn firejail_wrap_command_preserves_original_command() {
        let sandbox = FirejailSandbox;
        let mut cmd = Command::new("ls");
        cmd.arg("-la");
        cmd.arg("/workspace");
        sandbox.wrap_command(&mut cmd).unwrap();

        // 提取所有参数用于断言检查
        let args: Vec<String> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

        // 验证原始命令和参数都被保留
        assert!(args.contains(&"ls".to_string()), "original program must be passed as argument");
        assert!(args.contains(&"-la".to_string()), "original args must be preserved");
        assert!(args.contains(&"/workspace".to_string()), "original args must be preserved");
    }
}
