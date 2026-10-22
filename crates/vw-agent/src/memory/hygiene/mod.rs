//! 内存与对话数据清理模块
//!
//! 本模块负责执行周期性的内存和会话数据清理任务（hygiene），包括：
//! - 将过期的内存文件归档
//! - 将过期的会话文件归档
//! - 清理过期的内存归档文件
//! - 清理过期的会话归档文件
//! - 清理数据库中过期的对话记录
//!
//! 清理任务采用尽力而为（best-effort）策略，失败时不会阻塞主流程。
//! 通过配置可以控制清理的启用状态以及各类数据保留的时间阈值。

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use crate::app::agent::config::MemoryConfig;
    use crate::app::agent::memory::paths;
    use anyhow::Result;
    use chrono::{DateTime, Duration, Local, NaiveDate, Utc};
    use rusqlite::{Connection, params};
    use serde::{Deserialize, Serialize};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::{Duration as StdDuration, SystemTime};

    /// 清理任务执行的间隔时间（小时）
    ///
    /// 防止过于频繁执行清理操作，默认每 12 小时检查一次
    const HYGIENE_INTERVAL_HOURS: i64 = 12;

    /// 清理状态文件名
    ///
    /// 用于记录上次清理的时间和清理报告
    const STATE_FILE: &str = "memory_hygiene_state.json";

    /// 清理报告结构体
    ///
    /// 记录一次清理任务中执行的所有操作及其计数
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    struct HygieneReport {
        /// 已归档的内存文件数量
        archived_memory_files: u64,
        /// 已归档的会话文件数量
        archived_session_files: u64,
        /// 已清理的内存归档文件数量
        purged_memory_archives: u64,
        /// 已清理的会话归档文件数量
        purged_session_archives: u64,
        /// 已从数据库删除的对话记录行数
        pruned_conversation_rows: u64,
    }

    impl HygieneReport {
        /// 计算本次清理任务的总操作数
        ///
        /// # 返回值
        ///
        /// 返回所有清理操作的计数总和
        fn total_actions(&self) -> u64 {
            self.archived_memory_files
                + self.archived_session_files
                + self.purged_memory_archives
                + self.purged_session_archives
                + self.pruned_conversation_rows
        }
    }

    /// 清理状态结构体
    ///
    /// 用于持久化保存清理任务的执行状态，包括上次执行时间和清理报告
    #[derive(Debug, Clone, Default, Serialize, Deserialize)]
    struct HygieneState {
        /// 上次清理执行的 UTC 时间（RFC3339 格式）
        last_run_at: Option<String>,
        /// 上次清理的报告详情
        last_report: HygieneReport,
    }

    /// 根据配置和时间间隔执行内存和会话数据清理
    ///
    /// 此函数采用尽力而为（best-effort）策略：即使失败也不应阻塞调用方，
    /// 调用方应该记录错误并继续执行。
    ///
    /// # 参数
    ///
    /// * `config` - 内存配置，包含启用标志和各类保留时间阈值
    /// * `workspace_dir` - 工作空间目录路径，包含内存文件、会话文件和数据库
    ///
    /// # 返回值
    ///
    /// 返回 `Ok(())` 表示清理任务成功执行或被跳过（未启用/未到时间）
    ///
    /// # 执行条件
    ///
    /// 只有当以下条件同时满足时才会执行清理：
    /// 1. 配置中启用了清理功能（`hygiene_enabled` 为 true）
    /// 2. 距离上次清理已超过 `HYGIENE_INTERVAL_HOURS` 小时
    ///
    /// # 示例
    ///
    /// ```ignore
    /// use std::path::Path;
    /// let config = MemoryConfig::default();
    /// let workspace = Path::new("/path/to/workspace");
    /// run_if_due(&config, workspace)?;
    /// ```
    pub fn run_if_due(config: &MemoryConfig, workspace_dir: &Path) -> Result<()> {
        // 检查是否启用清理功能
        if !config.hygiene_enabled {
            return Ok(());
        }

        // 检查是否到达执行时间
        if !should_run_now(workspace_dir)? {
            return Ok(());
        }

        let storage_dir = paths::project_data_dir(workspace_dir)?;

        // 执行各项清理任务并收集报告
        let report = HygieneReport {
            archived_memory_files: archive_daily_memory_files(
                &storage_dir,
                config.archive_after_days,
            )?,
            archived_session_files: archive_session_files(&storage_dir, config.archive_after_days)?,
            purged_memory_archives: purge_memory_archives(&storage_dir, config.purge_after_days)?,
            purged_session_archives: purge_session_archives(&storage_dir, config.purge_after_days)?,
            pruned_conversation_rows: prune_conversation_rows(
                &storage_dir,
                config.conversation_retention_days,
            )?,
        };

        // 持久化清理状态
        write_state(workspace_dir, &report)?;

        // 如果有实际清理操作，记录日志
        if report.total_actions() > 0 {
            tracing::info!(
                "memory hygiene complete: archived_memory={} archived_sessions={} purged_memory={} purged_sessions={} pruned_conversation_rows={}",
                report.archived_memory_files,
                report.archived_session_files,
                report.purged_memory_archives,
                report.purged_session_archives,
                report.pruned_conversation_rows,
            );
        }

        Ok(())
    }

    /// 检查是否应该立即执行清理任务
    ///
    /// 通过读取持久化的状态文件，判断距离上次清理是否已超过配置的间隔时间
    ///
    /// # 参数
    ///
    /// * `workspace_dir` - 工作空间目录路径
    ///
    /// # 返回值
    ///
    /// * `Ok(true)` - 应该立即执行清理
    /// * `Ok(false)` - 尚未到执行时间
    ///
    /// # 容错处理
    ///
    /// 以下情况会返回 `Ok(true)`，确保清理能够执行：
    /// - 状态文件不存在（首次运行）
    /// - 状态文件解析失败（文件损坏）
    /// - 时间戳解析失败（格式错误）
    /// - 缺少上次执行时间记录
    fn should_run_now(workspace_dir: &Path) -> Result<bool> {
        let path = state_path(workspace_dir)?;
        // 状态文件不存在，首次运行，应该执行清理
        if !path.exists() {
            return Ok(true);
        }

        // 读取并解析状态文件
        let raw = fs::read_to_string(&path)?;
        let state: HygieneState = match serde_json::from_str(&raw) {
            Ok(s) => s,
            Err(_) => return Ok(true), // 解析失败，执行清理
        };

        // 检查是否有上次执行时间记录
        let Some(last_run_at) = state.last_run_at else {
            return Ok(true);
        };

        // 解析上次执行时间
        let last = match DateTime::parse_from_rfc3339(&last_run_at) {
            Ok(ts) => ts.with_timezone(&Utc),
            Err(_) => return Ok(true), // 时间戳解析失败，执行清理
        };

        // 判断是否已超过清理间隔
        Ok(Utc::now().signed_duration_since(last) >= Duration::hours(HYGIENE_INTERVAL_HOURS))
    }

    /// 将清理状态写入持久化文件
    ///
    /// 保存当前清理时间和报告详情，供下次检查时使用
    ///
    /// # 参数
    ///
    /// * `workspace_dir` - 工作空间目录路径
    /// * `report` - 本次清理的报告
    ///
    /// # 返回值
    ///
    /// 返回 `Ok(())` 表示写入成功
    fn write_state(workspace_dir: &Path, report: &HygieneReport) -> Result<()> {
        let path = state_path(workspace_dir)?;
        // 确保父目录存在
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        // 构建状态结构并序列化为 JSON
        let state = HygieneState {
            last_run_at: Some(Utc::now().to_rfc3339()),
            last_report: report.clone(),
        };
        let json = serde_json::to_vec_pretty(&state)?;
        fs::write(path, json)?;
        Ok(())
    }

    /// 构建清理状态文件的完整路径
    ///
    /// # 参数
    ///
    /// * `workspace_dir` - 工作空间目录路径
    ///
    /// # 返回值
    ///
    /// 返回状态文件路径：`~/.shiju-agent/worktree/workspaces/<workspace-id>/state/memory_hygiene_state.json`
    fn state_path(workspace_dir: &Path) -> Result<PathBuf> {
        Ok(paths::workspace_data_dir(workspace_dir)?.join("state").join(STATE_FILE))
    }

    /// 归档过期的每日内存文件
    ///
    /// 扫描 `memory/` 目录中的 `.md` 文件，将超过指定天数的文件移动到
    /// `memory/archive/` 目录中
    ///
    /// # 参数
    ///
    /// * `workspace_dir` - 工作空间目录路径
    /// * `archive_after_days` - 归档阈值（天），0 表示禁用归档
    ///
    /// # 返回值
    ///
    /// 返回成功归档的文件数量
    ///
    /// # 文件命名约定
    ///
    /// 内存文件名应包含日期前缀，格式为 `YYYY-MM-DD_*.md`，
    /// 例如：`2024-01-15_notes.md`
    fn archive_daily_memory_files(workspace_dir: &Path, archive_after_days: u32) -> Result<u64> {
        // 归档天数为 0 表示禁用此功能
        if archive_after_days == 0 {
            return Ok(0);
        }

        let memory_dir = workspace_dir.join("memory");
        if !memory_dir.is_dir() {
            return Ok(0);
        }

        // 创建归档目录
        let archive_dir = memory_dir.join("archive");
        fs::create_dir_all(&archive_dir)?;

        // 计算截止日期
        let cutoff = Local::now().date_naive() - Duration::days(i64::from(archive_after_days));
        let mut moved = 0_u64;

        // 遍历内存目录中的所有文件
        for entry in fs::read_dir(&memory_dir)? {
            let entry = entry?;
            let path = entry.path();

            // 跳过子目录
            if path.is_dir() {
                continue;
            }
            // 只处理 .md 文件
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            let Some(filename) = path.file_name().and_then(|f| f.to_str()) else {
                continue;
            };

            // 从文件名中提取日期
            let Some(file_date) = memory_date_from_filename(filename) else {
                continue;
            };

            // 如果文件日期早于截止日期，归档该文件
            if file_date < cutoff {
                move_to_archive(&path, &archive_dir)?;
                moved += 1;
            }
        }

        Ok(moved)
    }

    /// 归档过期的会话文件
    ///
    /// 扫描 `sessions/` 目录中的文件，将超过指定天数的文件移动到
    /// `sessions/archive/` 目录中
    ///
    /// # 参数
    ///
    /// * `workspace_dir` - 工作空间目录路径
    /// * `archive_after_days` - 归档阈值（天），0 表示禁用归档
    ///
    /// # 返回值
    ///
    /// 返回成功归档的文件数量
    ///
    /// # 日期判断策略
    ///
    /// 1. 优先从文件名前缀提取日期（格式：`YYYY-MM-DD_*`）
    /// 2. 如果文件名不包含日期前缀，则使用文件修改时间判断
    fn archive_session_files(workspace_dir: &Path, archive_after_days: u32) -> Result<u64> {
        // 归档天数为 0 表示禁用此功能
        if archive_after_days == 0 {
            return Ok(0);
        }

        let sessions_dir = workspace_dir.join("sessions");
        if !sessions_dir.is_dir() {
            return Ok(0);
        }

        // 创建归档目录
        let archive_dir = sessions_dir.join("archive");
        fs::create_dir_all(&archive_dir)?;

        // 计算截止日期和截止时间（用于文件修改时间判断）
        let cutoff_date = Local::now().date_naive() - Duration::days(i64::from(archive_after_days));
        let cutoff_time = SystemTime::now()
            .checked_sub(StdDuration::from_secs(u64::from(archive_after_days) * 24 * 60 * 60))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let mut moved = 0_u64;
        // 遍历会话目录中的所有文件
        for entry in fs::read_dir(&sessions_dir)? {
            let entry = entry?;
            let path = entry.path();

            // 跳过子目录
            if path.is_dir() {
                continue;
            }

            let Some(filename) = path.file_name().and_then(|f| f.to_str()) else {
                continue;
            };

            // 判断文件是否过期：优先使用文件名日期，否则使用修改时间
            let is_old = if let Some(date) = date_prefix(filename) {
                date < cutoff_date
            } else {
                is_older_than(&path, cutoff_time)
            };

            if is_old {
                move_to_archive(&path, &archive_dir)?;
                moved += 1;
            }
        }

        Ok(moved)
    }

    /// 清理过期的内存归档文件
    ///
    /// 扫描 `memory/archive/` 目录，删除超过指定天数的归档文件
    ///
    /// # 参数
    ///
    /// * `workspace_dir` - 工作空间目录路径
    /// * `purge_after_days` - 清理阈值（天），0 表示禁用清理
    ///
    /// # 返回值
    ///
    /// 返回成功删除的文件数量
    ///
    /// # 注意
    ///
    /// 这是归档文件的最终清理操作，文件将被永久删除
    fn purge_memory_archives(workspace_dir: &Path, purge_after_days: u32) -> Result<u64> {
        // 清理天数为 0 表示禁用此功能
        if purge_after_days == 0 {
            return Ok(0);
        }

        let archive_dir = workspace_dir.join("memory").join("archive");
        if !archive_dir.is_dir() {
            return Ok(0);
        }

        // 计算截止日期
        let cutoff = Local::now().date_naive() - Duration::days(i64::from(purge_after_days));
        let mut removed = 0_u64;

        // 遍历归档目录中的所有文件
        for entry in fs::read_dir(&archive_dir)? {
            let entry = entry?;
            let path = entry.path();

            // 跳过子目录
            if path.is_dir() {
                continue;
            }

            let Some(filename) = path.file_name().and_then(|f| f.to_str()) else {
                continue;
            };

            // 从文件名中提取日期
            let Some(file_date) = memory_date_from_filename(filename) else {
                continue;
            };

            // 删除过期文件
            if file_date < cutoff {
                fs::remove_file(&path)?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// 清理过期的会话归档文件
    ///
    /// 扫描 `sessions/archive/` 目录，删除超过指定天数的归档文件
    ///
    /// # 参数
    ///
    /// * `workspace_dir` - 工作空间目录路径
    /// * `purge_after_days` - 清理阈值（天），0 表示禁用清理
    ///
    /// # 返回值
    ///
    /// 返回成功删除的文件数量
    ///
    /// # 日期判断策略
    ///
    /// 1. 优先从文件名前缀提取日期（格式：`YYYY-MM-DD_*`）
    /// 2. 如果文件名不包含日期前缀，则使用文件修改时间判断
    ///
    /// # 注意
    ///
    /// 这是归档文件的最终清理操作，文件将被永久删除
    fn purge_session_archives(workspace_dir: &Path, purge_after_days: u32) -> Result<u64> {
        // 清理天数为 0 表示禁用此功能
        if purge_after_days == 0 {
            return Ok(0);
        }

        let archive_dir = workspace_dir.join("sessions").join("archive");
        if !archive_dir.is_dir() {
            return Ok(0);
        }

        // 计算截止日期和截止时间（用于文件修改时间判断）
        let cutoff_date = Local::now().date_naive() - Duration::days(i64::from(purge_after_days));
        let cutoff_time = SystemTime::now()
            .checked_sub(StdDuration::from_secs(u64::from(purge_after_days) * 24 * 60 * 60))
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let mut removed = 0_u64;
        // 遍历归档目录中的所有文件
        for entry in fs::read_dir(&archive_dir)? {
            let entry = entry?;
            let path = entry.path();

            // 跳过子目录
            if path.is_dir() {
                continue;
            }

            let Some(filename) = path.file_name().and_then(|f| f.to_str()) else {
                continue;
            };

            // 判断文件是否过期：优先使用文件名日期，否则使用修改时间
            let is_old = if let Some(date) = date_prefix(filename) {
                date < cutoff_date
            } else {
                is_older_than(&path, cutoff_time)
            };

            if is_old {
                fs::remove_file(&path)?;
                removed += 1;
            }
        }

        Ok(removed)
    }

    /// 清理数据库中过期的对话记录
    ///
    /// 从 `memory/brain.db` 数据库中删除超过指定天数的对话记录
    ///
    /// # 参数
    ///
    /// * `workspace_dir` - 工作空间目录路径
    /// * `retention_days` - 保留天数，0 表示禁用清理
    ///
    /// # 返回值
    ///
    /// 返回成功删除的记录行数
    ///
    /// # 数据库配置
    ///
    /// 使用 WAL 模式以避免清理操作阻塞代理的读取操作
    /// 设置 `synchronous = NORMAL` 以平衡性能和安全性
    fn prune_conversation_rows(workspace_dir: &Path, retention_days: u32) -> Result<u64> {
        // 保留天数为 0 表示禁用此功能
        if retention_days == 0 {
            return Ok(0);
        }

        let db_path = workspace_dir.join("memory").join("brain.db");
        if !db_path.exists() {
            return Ok(0);
        }

        let conn = Connection::open(db_path)?;
        // 使用 WAL 模式，确保清理操作不会阻塞代理的读取操作
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
        let cutoff = (Local::now() - Duration::days(i64::from(retention_days))).to_rfc3339();

        // 删除过期的对话记录
        let affected = conn.execute(
            "DELETE FROM memories WHERE category = 'conversation' AND updated_at < ?1",
            params![cutoff],
        )?;

        Ok(u64::try_from(affected).unwrap_or(0))
    }

    /// 从内存文件名中提取日期
    ///
    /// 支持的文件名格式：`YYYY-MM-DD_*.md` 或 `YYYY-MM-DD.md`
    ///
    /// # 参数
    ///
    /// * `filename` - 文件名（不含路径）
    ///
    /// # 返回值
    ///
    /// 如果文件名包含有效日期，返回 `Some(date)`，否则返回 `None`
    ///
    /// # 示例
    ///
    /// ```ignore
    /// assert_eq!(
    ///     memory_date_from_filename("2024-01-15_notes.md"),
    ///     Some(NaiveDate::from_ymd_opt(2024, 1, 15).unwrap())
    /// );
    /// assert_eq!(memory_date_from_filename("invalid.md"), None);
    /// ```
    fn memory_date_from_filename(filename: &str) -> Option<NaiveDate> {
        let stem = filename.strip_suffix(".md")?;
        let date_part = stem.split('_').next().unwrap_or(stem);
        NaiveDate::parse_from_str(date_part, "%Y-%m-%d").ok()
    }

    /// 从文件名前缀中提取日期
    ///
    /// 尝试从文件名的前 10 个字符（`YYYY-MM-DD`）解析日期
    ///
    /// # 参数
    ///
    /// * `filename` - 文件名（不含路径）
    ///
    /// # 返回值
    ///
    /// 如果文件名前缀是有效日期，返回 `Some(date)`，否则返回 `None`
    ///
    /// # 安全处理
    ///
    /// 使用 `floor_utf8_char_boundary` 确保在 UTF-8 多字节字符边界处截断
    #[allow(clippy::incompatible_msrv)]
    fn date_prefix(filename: &str) -> Option<NaiveDate> {
        // 文件名长度不足，无法包含日期前缀
        if filename.len() < 10 {
            return None;
        }
        // 安全截断到 UTF-8 字符边界
        let prefix_len = crate::app::agent::util::floor_utf8_char_boundary(filename, 10);
        NaiveDate::parse_from_str(&filename[..prefix_len], "%Y-%m-%d").ok()
    }

    /// 检查文件是否早于指定时间
    ///
    /// 通过文件的修改时间（mtime）判断文件是否过期
    ///
    /// # 参数
    ///
    /// * `path` - 文件路径
    /// * `cutoff` - 截止时间
    ///
    /// # 返回值
    ///
    /// 如果文件的修改时间早于截止时间，返回 `true`
    /// 如果无法获取文件元数据或修改时间，返回 `false`（保守处理，不删除）
    fn is_older_than(path: &Path, cutoff: SystemTime) -> bool {
        fs::metadata(path)
            .and_then(|meta| meta.modified())
            .map(|modified| modified < cutoff)
            .unwrap_or(false)
    }

    /// 将文件移动到归档目录
    ///
    /// 使用 `rename` 系统调用移动文件，如果目标文件已存在，会生成唯一文件名
    ///
    /// # 参数
    ///
    /// * `src` - 源文件路径
    /// * `archive_dir` - 归档目录路径
    ///
    /// # 返回值
    ///
    /// 返回 `Ok(())` 表示移动成功
    fn move_to_archive(src: &Path, archive_dir: &Path) -> Result<()> {
        let Some(filename) = src.file_name().and_then(|f| f.to_str()) else {
            return Ok(());
        };

        // 生成唯一的归档目标路径
        let target = unique_archive_target(archive_dir, filename);
        fs::rename(src, target)?;
        Ok(())
    }

    /// 生成唯一的归档文件路径
    ///
    /// 如果目标文件已存在，会自动添加序号后缀（`_1`, `_2`, ...）直到找到可用文件名
    ///
    /// # 参数
    ///
    /// * `archive_dir` - 归档目录路径
    /// * `filename` - 原始文件名
    ///
    /// # 返回值
    ///
    /// 返回唯一的归档文件路径
    ///
    /// # 示例
    ///
    /// ```ignore
    /// // 如果 file.md 不存在
    /// unique_archive_target(dir, "file.md") // => dir/file.md
    ///
    /// // 如果 file.md 已存在
    /// unique_archive_target(dir, "file.md") // => dir/file_1.md
    ///
    /// // 如果 file_1.md 也已存在
    /// unique_archive_target(dir, "file.md") // => dir/file_2.md
    /// ```
    fn unique_archive_target(archive_dir: &Path, filename: &str) -> PathBuf {
        // 首先尝试直接使用原文件名
        let direct = archive_dir.join(filename);
        if !direct.exists() {
            return direct;
        }

        // 文件已存在，添加序号后缀
        let (stem, ext) = split_name(filename);
        for i in 1..10_000 {
            let candidate = if ext.is_empty() {
                archive_dir.join(format!("{stem}_{i}"))
            } else {
                archive_dir.join(format!("{stem}_{i}.{ext}"))
            };
            if !candidate.exists() {
                return candidate;
            }
        }

        // 极端情况：所有序号都被占用，返回原路径（后续操作会失败）
        direct
    }

    /// 分离文件名的主干和扩展名
    ///
    /// # 参数
    ///
    /// * `filename` - 文件名
    ///
    /// # 返回值
    ///
    /// 返回元组 `(stem, extension)`，如果没有扩展名则 extension 为空字符串
    ///
    /// # 示例
    ///
    /// ```ignore
    /// assert_eq!(split_name("file.txt"), ("file", "txt"));
    /// assert_eq!(split_name("noext"), ("noext", ""));
    /// assert_eq!(split_name("multi.dot.file"), ("multi.dot", "file"));
    /// ```
    fn split_name(filename: &str) -> (&str, &str) {
        match filename.rsplit_once('.') {
            Some((stem, ext)) => (stem, ext),
            None => (filename, ""),
        }
    }

    #[cfg(test)]
    mod tests {
        include!("tests.rs");

        include!("mod_tests.rs");
    }
}

/// 非 WASM 平台：导出完整实现
#[cfg(not(target_arch = "wasm32"))]
pub use imp::*;

/// WASM 平台：提供空实现
///
/// WASM 环境下不支持文件系统操作，因此清理功能被禁用
/// 该函数始终返回成功，不做任何操作
///
/// # 参数
///
/// * `_config` - 内存配置（未使用）
/// * `_workspace_dir` - 工作空间目录路径（未使用）
///
/// # 返回值
///
/// 始终返回 `Ok(())`
#[cfg(target_arch = "wasm32")]
pub fn run_if_due(
    _config: &super::super::config::MemoryConfig,
    _workspace_dir: &std::path::Path,
) -> anyhow::Result<()> {
    Ok(())
}
