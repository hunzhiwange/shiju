//! 心跳引擎模块
//!
//! 该模块提供了心跳任务调度引擎，用于定期读取和执行 `HEARTBEAT.md` 文件中定义的周期性任务。
//!
//! # 功能特性
//!
//! - **周期性执行**: 按配置的时间间隔定期检查任务文件
//! - **任务收集**: 从 `HEARTBEAT.md` 文件中解析任务列表
//! - **事件监控**: 与观测系统集成，记录心跳事件和错误
//! - **平台兼容**: 支持 WASM 和原生平台的差异化实现
//!
//! # 使用示例
//!
//! ```rust,no_run
//! use std::sync::Arc;
//! use std::path::PathBuf;
//! use crate::engine::HeartbeatEngine;
//! use config_automation::HeartbeatConfig;
//! use observer_types::Observer;
//!
//! async fn run_heartbeat() {
//!     let config = HeartbeatConfig::default();
//!     let workspace = PathBuf::from("/path/to/workspace");
//!     let observer = Arc::new(/* your observer implementation */);
//!
//!     let engine = HeartbeatEngine::new(config, workspace, observer);
//!     engine.run().await.expect("Heartbeat failed");
//! }
//! ```

use observer_types::{Observer, ObserverEvent};
use config_automation::HeartbeatConfig;
use anyhow::Result;
use std::path::Path;
use std::sync::Arc;
use tokio::time::{self, Duration};
use tracing::{info, warn};

/// 心跳引擎
///
/// 负责定期读取 `HEARTBEAT.md` 文件并执行其中定义的周期性任务。
/// 该引擎会按照配置的时间间隔持续运行，直到被显式取消。
///
/// # 字段说明
///
/// - `config`: 心跳配置，包含启用状态和执行间隔等参数
/// - `workspace_dir`: 工作空间目录路径，`HEARTBEAT.md` 文件应位于此目录下
/// - `observer`: 观测器实例，用于记录心跳事件和错误信息
pub struct HeartbeatEngine {
    config: HeartbeatConfig,
    workspace_dir: std::path::PathBuf,
    observer: Arc<dyn Observer>,
}

impl HeartbeatEngine {
    /// 创建新的心跳引擎实例
    ///
    /// # 参数
    ///
    /// - `config`: 心跳配置对象，包含启用状态和执行间隔等设置
    /// - `workspace_dir`: 工作空间目录路径，用于定位 `HEARTBEAT.md` 文件
    /// - `observer`: 观测器实例，用于记录运行时事件和错误
    ///
    /// # 返回值
    ///
    /// 返回一个配置完成的心跳引擎实例
    ///
    /// # 示例
    ///
    /// ```rust,no_run
    /// use std::sync::Arc;
    /// use std::path::PathBuf;
    /// use crate::engine::HeartbeatEngine;
    /// use config_automation::HeartbeatConfig;
    ///
    /// let engine = HeartbeatEngine::new(
    ///     HeartbeatConfig::default(),
    ///     PathBuf::from("/workspace"),
    ///     observer,
    /// );
    /// ```
    pub fn new(
        config: HeartbeatConfig,
        workspace_dir: std::path::PathBuf,
        observer: Arc<dyn Observer>,
    ) -> Self {
        Self { config, workspace_dir, observer }
    }

    /// 启动心跳循环
    ///
    /// 该方法会启动一个持续运行的心跳循环，按照配置的时间间隔定期执行任务检查。
    /// 循环会一直运行直到被取消（通常通过 Tokio 的取消机制）。
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 当心跳被禁用或正常启动时返回
    /// - `Err(e)`: 如果发生运行时错误则返回错误信息
    ///
    /// # 执行流程
    ///
    /// 1. 检查心跳是否启用，如果未启用则直接返回
    /// 2. 确定执行间隔（最小为 5 分钟）
    /// 3. 启动定时器循环
    /// 4. 每次间隔到期时：
    ///    - 记录心跳事件
    ///    - 执行任务收集和处理
    ///    - 记录处理结果或错误
    ///
    /// # 错误处理
    ///
    /// 单次心跳失败不会中断整个循环，错误会被记录并继续下一次执行
    pub async fn run(&self) -> Result<()> {
        // 检查心跳功能是否启用
        if !self.config.enabled {
            info!("Heartbeat disabled");
            return Ok(());
        }

        // 确定执行间隔，最小间隔为 5 分钟
        let interval_mins = self.config.interval_minutes.max(5);
        info!("💓 Heartbeat started: every {} minutes", interval_mins);

        // 创建定时器，间隔时间转换为秒
        let mut interval = time::interval(Duration::from_secs(u64::from(interval_mins) * 60));

        // 主循环：持续运行直到被取消
        loop {
            // 等待下一个间隔周期
            interval.tick().await;

            // 记录心跳事件到观测系统
            self.observer.record_event(&ObserverEvent::HeartbeatTick);

            // 执行单次心跳任务处理
            match self.tick().await {
                Ok(tasks) => {
                    // 仅在有任务时记录日志
                    if tasks > 0 {
                        info!("💓 Heartbeat: processed {} tasks", tasks);
                    }
                }
                Err(e) => {
                    // 记录错误但不中断循环
                    warn!("💓 Heartbeat error: {}", e);
                    self.observer.record_event(&ObserverEvent::Error {
                        component: "heartbeat".into(),
                        message: e.to_string(),
                    });
                }
            }
        }
    }

    /// 执行单次心跳任务
    ///
    /// 读取 `HEARTBEAT.md` 文件并返回找到的任务数量。
    /// 这是心跳循环的核心处理方法，每次心跳间隔都会被调用。
    ///
    /// # 返回值
    ///
    /// - `Ok(usize)`: 成功返回找到的任务数量
    /// - `Err(e)`: 如果读取或解析文件时发生错误
    async fn tick(&self) -> Result<usize> {
        Ok(self.collect_tasks().await?.len())
    }

    /// 收集心跳任务
    ///
    /// 读取工作空间中的 `HEARTBEAT.md` 文件，并解析其中定义的所有任务。
    /// 任务以 `- ` 前缀标识，每行一个任务。
    ///
    /// # 返回值
    ///
    /// - `Ok(Vec<String>)`: 成功返回任务列表，如果没有文件则为空列表
    /// - `Err(e)`: 如果读取文件时发生 I/O 错误
    ///
    /// # 平台差异
    ///
    /// 该实现仅用于非 WASM 平台（native）。WASM 平台使用单独的实现。
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn collect_tasks(&self) -> Result<Vec<String>> {
        // 构建 HEARTBEAT.md 文件路径
        let heartbeat_path = self.workspace_dir.join("HEARTBEAT.md");

        // 如果文件不存在，返回空任务列表
        if !heartbeat_path.exists() {
            return Ok(Vec::new());
        }

        // 异步读取文件内容
        let content = tokio::fs::read_to_string(&heartbeat_path).await?;

        // 解析并返回任务列表
        Ok(Self::parse_tasks(&content))
    }

    /// 解析 HEARTBEAT.md 文件内容
    ///
    /// 从文件内容中提取所有任务定义。任务的识别规则：
    /// - 每行以 `- ` 开头的文本被视为一个任务
    /// - 空白字符会被自动去除
    /// - 不符合格式的行会被忽略
    ///
    /// # 参数
    ///
    /// - `content`: HEARTBEAT.md 文件的文本内容
    ///
    /// # 返回值
    ///
    /// 返回解析后的任务字符串列表
    ///
    /// # 示例
    ///
    /// ```rust
    /// let content = r#"
    /// # Periodic Tasks
    /// - Check email
    /// - Review calendar
    /// # Comment line
    /// - Check weather
    /// "#;
    ///
    /// let tasks = HeartbeatEngine::parse_tasks(content);
    /// assert_eq!(tasks, vec!["Check email", "Review calendar", "Check weather"]);
    /// ```
    fn parse_tasks(content: &str) -> Vec<String> {
        content
            .lines()
            .filter_map(|line| {
                // 去除行首尾空白
                let trimmed = line.trim();
                // 检查是否以 "- " 开头，如果是则提取任务内容
                trimmed.strip_prefix("- ").map(ToString::to_string)
            })
            .collect()
    }

    /// 收集心跳任务（WASM 版本）
    ///
    /// 在 WASM 平台上，心跳文件功能不可用，因此总是返回空列表。
    ///
    /// # 返回值
    ///
    /// 总是返回 `Ok(Vec::new())`
    #[cfg(target_arch = "wasm32")]
    pub async fn collect_tasks(&self) -> Result<Vec<String>> {
        Ok(Vec::new())
    }

    /// 确保心跳文件存在
    ///
    /// 检查工作空间中是否存在 `HEARTBEAT.md` 文件，如果不存在则创建一个包含说明和示例的默认文件。
    /// 这确保了用户能够快速了解如何使用心跳功能。
    ///
    /// # 参数
    ///
    /// - `workspace_dir`: 工作空间目录路径，心跳文件将在此目录下创建
    ///
    /// # 返回值
    ///
    /// - `Ok(())`: 文件已存在或成功创建
    /// - `Err(e)`: 如果创建文件时发生 I/O 错误
    ///
    /// # 默认文件内容
    ///
    /// 创建的默认文件包含：
    /// - 标题和说明文字
    /// - 任务格式说明（使用 `- ` 前缀）
    /// - 示例任务（检查邮件、查看日历、查询天气等）
    ///
    /// # 平台差异
    ///
    /// 该实现仅用于非 WASM 平台（native）。WASM 平台使用单独的空实现。
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn ensure_heartbeat_file(workspace_dir: &Path) -> Result<()> {
        let path = workspace_dir.join("HEARTBEAT.md");

        // 仅在文件不存在时创建
        if !path.exists() {
            // 默认文件模板，包含使用说明和示例
            let default = "# Periodic Tasks\n\n\
                           # Add tasks below (one per line, starting with `- `)\n\
                           # The agent will check this file on each heartbeat tick.\n\
                           #\n\
                           # Examples:\n\
                           # - Check my email for important messages\n\
                           # - Review my calendar for upcoming events\n\
                           # - Check the weather forecast\n";

            // 异步写入文件
            tokio::fs::write(&path, default).await?;
        }
        Ok(())
    }

    /// 确保心跳文件存在（WASM 版本）
    ///
    /// 在 WASM 平台上，心跳文件功能不可用，因此不执行任何操作。
    ///
    /// # 参数
    ///
    /// - `_workspace_dir`: 工作空间目录路径（未使用）
    ///
    /// # 返回值
    ///
    /// 总是返回 `Ok(())`
    #[cfg(target_arch = "wasm32")]
    pub async fn ensure_heartbeat_file(_workspace_dir: &Path) -> Result<()> {
        Ok(())
    }
}

/// 单元测试模块
///
/// 包含心跳引擎的各项测试用例，涵盖任务解析、文件处理等功能
#[cfg(test)]
#[path = "tests.rs"]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
