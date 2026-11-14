//! Landlock 沙箱模块的单元测试
//!
//! 本模块包含对 `LandlockSandbox` 实现的全面测试用例，覆盖以下场景：
//!
//! - 沙箱名称验证
//! - 平台可用性检测
//! - 无工作空间目录时的行为
//! - 存根（stub）实现在不支持平台上的错误处理
//!
//! ## 测试条件编译
//!
//! 测试根据平台和特性标志进行条件编译：
//! - `#[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]`：仅在 Linux 平台且启用
//!   `sandbox-landlock` 特性时运行
//! - `#[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]`：在非 Linux 平台或
//!   未启用特性时运行存根测试

use super::*;

#[allow(dead_code)]
mod tests {
    use super::*;

    /// 测试 Landlock 沙箱的名称标识
    ///
    /// 验证沙箱实例返回正确的名称字符串 "landlock"。
    /// 此测试仅在 Linux 平台且启用 `sandbox-landlock` 特性时执行。
    ///
    /// # 行为说明
    ///
    /// - 如果沙箱创建成功，断言其名称为 "landlock"
    /// - 如果创建失败（例如内核不支持），测试会静默通过
    ///
    /// # 示例
    ///
    /// ```ignore
    /// let sandbox = LandlockSandbox::new()?;
    /// assert_eq!(sandbox.name(), "landlock");
    /// ```
    #[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
    #[test]
    fn landlock_sandbox_name() {
        if let Ok(sandbox) = LandlockSandbox::new() {
            assert_eq!(sandbox.name(), "landlock");
        }
    }

    /// 测试 Landlock 沙箱在非 Linux 平台上的不可用性
    ///
    /// 验证在非 Linux 平台或未启用 `sandbox-landlock` 特性时，
    /// 沙箱正确报告自身不可用，同时名称标识保持一致。
    ///
    /// # 断言
    ///
    /// - `is_available()` 返回 `false`
    /// - `name()` 仍返回 "landlock"（名称与可用性解耦）
    #[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
    #[test]
    fn landlock_not_available_on_non_linux() {
        assert!(!LandlockSandbox.is_available());
        assert_eq!(LandlockSandbox.name(), "landlock");
    }

    /// 测试在无工作空间目录时创建 Landlock 沙箱的行为
    ///
    /// 验证 `with_workspace(None)` 在没有指定工作空间目录时的处理逻辑。
    /// 此测试在所有平台上运行，根据配置预期不同的结果。
    ///
    /// # 预期行为
    ///
    /// - **成功时**：沙箱应报告为可用（`is_available()` 返回 `true`）
    /// - **失败时**：仅在非 Linux 平台或未启用特性时允许失败
    ///
    /// # 设计考量
    ///
    /// 即使没有工作空间目录，沙箱也应能正常初始化。
    /// 工作空间主要用于限制文件系统访问范围，而非沙箱存在的先决条件。
    #[test]
    fn landlock_with_none_workspace() {
        let result = LandlockSandbox::with_workspace(None);
        match result {
            Ok(sandbox) => assert!(sandbox.is_available()),
            Err(_) => assert!(!cfg!(all(feature = "sandbox-landlock", target_os = "linux"))),
        }
    }

    #[test]
    fn landlock_with_workspace_follows_platform_availability() {
        let dir = tempfile::tempdir().unwrap();
        let result = LandlockSandbox::with_workspace(Some(dir.path().to_path_buf()));

        match result {
            Ok(sandbox) => assert_eq!(sandbox.name(), "landlock"),
            Err(err) => assert!(
                matches!(err.kind(), std::io::ErrorKind::Unsupported | std::io::ErrorKind::Other),
                "unexpected landlock error kind: {:?}",
                err.kind()
            ),
        }
    }

    #[test]
    fn landlock_description_mentions_platform_state() {
        #[cfg(all(feature = "sandbox-landlock", target_os = "linux"))]
        if let Ok(sandbox) = LandlockSandbox::new() {
            assert!(sandbox.description().contains("Linux"));
            return;
        }

        #[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
        {
            let sandbox = LandlockSandbox;
            assert!(sandbox.description().contains("不可用"));
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    // §1.1 Landlock 存根测试
    // ═══════════════════════════════════════════════════════════════════════
    // 以下测试验证在非 Linux 平台或未启用 sandbox-landlock 特性时，
    // 存根实现正确返回 Unsupported 错误，防止在不受支持的环境中误用沙箱功能。

    /// 测试存根实现的 `wrap_command` 方法返回不支持错误
    ///
    /// 在不支持 Landlock 的环境中，尝试包装命令应返回 `io::ErrorKind::Unsupported` 错误，
    /// 明确指示当前环境不支持此操作。
    ///
    /// # 错误类型
    ///
    /// 返回 `std::io::Error`，其 kind 为 `std::io::ErrorKind::Unsupported`。
    /// 这允许调用方通过错误类型区分"不支持"与其他类型的失败。
    #[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
    #[test]
    fn landlock_stub_wrap_command_returns_unsupported() {
        let sandbox = LandlockSandbox;
        let mut cmd = std::process::Command::new("echo");
        let result = sandbox.wrap_command(&mut cmd);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
    }

    /// 测试存根实现的 `new` 构造函数返回不支持错误
    ///
    /// 在不支持 Landlock 的环境中，直接调用 `new()` 创建沙箱实例应失败，
    /// 并返回 `io::ErrorKind::Unsupported` 错误。
    ///
    /// # 与 `probe` 的区别
    ///
    /// - `new()`：尝试创建实际的沙箱实例，失败时返回错误
    /// - `probe()`：仅检测可用性，不创建实例
    ///
    /// # 使用建议
    ///
    /// 在调用 `new()` 之前，建议先使用 `probe()` 或 `is_available()` 检查可用性，
    /// 以提供更友好的错误信息。
    #[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
    #[test]
    fn landlock_stub_new_returns_unsupported() {
        let result = LandlockSandbox::new();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
    }

    /// 测试存根实现的 `probe` 方法返回错误
    ///
    /// `probe()` 是一个静态方法，用于检测系统是否支持 Landlock 沙箱。
    /// 在不支持的环境中，应返回错误而非 `Ok(false)`，以便调用方区分
    /// "明确不支持"与"检测失败"两种情况。
    ///
    /// # 返回值语义
    ///
    /// - `Ok(true)`：系统支持 Landlock
    /// - `Ok(false)`：系统不支持 Landlock（但检测成功）
    /// - `Err(...)`：检测过程本身失败
    ///
    /// 存根实现直接返回错误，表示在不支持的平台上无法完成检测。
    #[cfg(not(all(feature = "sandbox-landlock", target_os = "linux")))]
    #[test]
    fn landlock_stub_probe_returns_unsupported() {
        let result = LandlockSandbox::probe();
        assert!(result.is_err());
    }
}
