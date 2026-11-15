//! 心跳模块的单元测试
//!
//! 本模块包含心跳引擎 (`HeartbeatEngine`) 的测试用例，用于验证：
//! - 心跳引擎的构造和初始化
//! - 心跳文件的创建和路径正确性
//!
//! # 测试策略
//!
//! 测试使用临时目录 (`tempfile::tempdir`) 来隔离文件系统操作，
//! 确保测试之间互不影响，且不会污染实际的工作目录。

use super::*;

/// 心跳模块的测试套件
///
/// 包含心跳引擎的核心功能验证测试。
#[allow(dead_code)]
mod tests {
    use super::*;

    use config_automation::HeartbeatConfig;
    use crate::engine::HeartbeatEngine;
    use agent_observability::NoopObserver;
    use std::sync::Arc;

    /// 测试心跳引擎是否可以通过模块导出正确构造
    ///
    /// # 测试目的
    ///
    /// 验证 `HeartbeatEngine::new` 构造函数能够使用默认配置、
    /// 临时工作目录和空观察器成功创建引擎实例。
    ///
    /// # 测试步骤
    ///
    /// 1. 创建临时目录作为工作空间
    /// 2. 使用默认心跳配置构造引擎实例
    /// 3. 验证构造过程不会 panic 或返回错误
    #[test]
    fn heartbeat_engine_is_constructible_via_module_export() {
        // 创建临时目录，用于隔离测试环境
        let temp = tempfile::tempdir().unwrap();

        // 使用默认配置、临时路径和空观察器构造心跳引擎
        let engine = HeartbeatEngine::new(
            HeartbeatConfig::default(),
            temp.path().to_path_buf(),
            Arc::new(NoopObserver),
        );

        // 验证引擎实例已成功创建
        let _ = engine;
    }

    /// 测试心跳文件是否在预期路径创建
    ///
    /// # 测试目的
    ///
    /// 验证 `HeartbeatEngine::ensure_heartbeat_file` 方法能够
    /// 在指定的工作目录下创建 `HEARTBEAT.md` 文件。
    ///
    /// # 测试步骤
    ///
    /// 1. 创建临时目录作为工作空间
    /// 2. 调用 `ensure_heartbeat_file` 方法
    /// 3. 验证 `HEARTBEAT.md` 文件确实存在于工作目录中
    ///
    /// # 预期结果
    ///
    /// 工作目录下应存在名为 `HEARTBEAT.md` 的文件。
    #[tokio::test]
    async fn ensure_heartbeat_file_creates_expected_file() {
        // 创建临时目录作为工作空间
        let temp = tempfile::tempdir().unwrap();
        let workspace = temp.path();

        // 确保心跳文件被创建
        HeartbeatEngine::ensure_heartbeat_file(workspace).await.unwrap();

        // 构造预期的文件路径：{workspace}/HEARTBEAT.md
        let heartbeat_path = workspace.join("HEARTBEAT.md");

        // 断言文件存在
        assert!(heartbeat_path.exists());
    }
}
