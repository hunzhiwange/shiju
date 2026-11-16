//! 内存管理命令行接口模块
//!
//! 本模块提供内存子系统的命令行操作支持，包括列出、查询、统计和清除记忆条目。
//! 支持多种内存后端（SQLite、PostgreSQL、MariaDB 等），并提供分页、过滤等功能。
//!
//! # 主要功能
//!
//! - **列出条目**：按分类、会话过滤，支持分页
//! - **查询条目**：精确或前缀匹配获取单个条目
//! - **统计信息**：查看后端健康状态和分类统计
//! - **清除条目**：按键、分类或全部清除，带确认机制
//!
//! # 架构说明
//!
//! 该模块作为 CLI 入口，通过 trait 对象 `dyn Memory` 与具体后端解耦，
//! 实际的存储操作由 `super::traits::Memory` 的各实现类完成。

use super::traits::{Memory, MemoryCategory};
use super::{
    MemoryBackendKind, classify_memory_backend, create_memory_for_migration,
    effective_memory_backend_name,
};
use agent_config::runtime::Config;
#[cfg(any(feature = "memory-postgres", feature = "memory-mariadb"))]
use anyhow::Context;
use anyhow::{Result, bail};
use console::style;

/// 处理内存命令的入口函数
///
/// 根据命令类型路由到对应的处理函数，执行相应的内存操作。
///
/// # 参数
///
/// - `command`: 要执行的内存命令（List/Get/Stats/Clear）
/// - `config`: 应用配置，包含内存后端设置
///
/// # 返回值
///
/// 成功返回 `Ok(())`，失败返回错误信息。
///
/// # 错误
///
/// - 后端连接失败
/// - 数据库操作错误
/// - 配置无效（如后端为 none）
pub async fn handle_command(command: MemoryCommands, config: &Config) -> Result<()> {
    match command {
        MemoryCommands::List { category, session, limit, offset } => {
            handle_list(config, category, session, limit, offset).await
        }
        MemoryCommands::Get { key } => handle_get(config, &key).await,
        MemoryCommands::Stats => handle_stats(config).await,
        MemoryCommands::Clear { key, category, yes } => {
            handle_clear(config, key, category, yes).await
        }
    }
}

/// 为 CLI 操作创建内存后端实例
///
/// 根据配置创建适合命令行操作的内存后端。对于需要迁移的后端（如 SQLite），
/// 会使用迁移专用工厂；对于数据库后端（PostgreSQL、MariaDB），会从配置中
/// 提取连接参数并建立连接。
///
/// # 参数
///
/// - `config`: 应用配置，包含内存后端和存储提供商配置
///
/// # 返回值
///
/// 返回装箱的 trait 对象 `Box<dyn Memory>`。
///
/// # 错误
///
/// - 后端被禁用（none）
/// - 数据库连接参数缺失（如 db_url）
/// - 编译时未启用对应特性标志
fn create_cli_memory(config: &Config) -> Result<Box<dyn Memory>> {
    let backend = effective_memory_backend_name(
        &config.memory.backend,
        Some(&config.storage.provider.config),
    );

    match classify_memory_backend(&backend) {
        MemoryBackendKind::None => {
            bail!("Memory backend is 'none' (disabled). No entries to manage.");
        }
        #[cfg(feature = "memory-postgres")]
        MemoryBackendKind::Postgres => {
            #[cfg(feature = "memory-postgres")]
            {
                let sp = &config.storage.provider.config;
                let db_url =
                    sp.db_url.as_deref().map(str::trim).filter(|v| !v.is_empty()).context(
                        "memory backend 'postgres' requires db_url in [storage.provider.config]",
                    )?;
                let mem = super::PostgresMemory::new(
                    db_url,
                    &sp.schema,
                    &sp.table,
                    sp.connect_timeout_secs,
                    sp.tls,
                )?;
                Ok(Box::new(mem))
            }
            #[cfg(not(feature = "memory-postgres"))]
            {
                bail!(
                    "Memory backend 'postgres' requires the 'memory-postgres' feature to be enabled at compile time."
                );
            }
        }
        #[cfg(not(feature = "memory-postgres"))]
        MemoryBackendKind::Postgres => {
            bail!("memory backend 'postgres' requires the 'memory-postgres' feature to be enabled");
        }
        #[cfg(feature = "memory-mariadb")]
        MemoryBackendKind::Mariadb => {
            let sp = &config.storage.provider.config;
            let db_url =
                sp.db_url.as_deref().map(str::trim).filter(|v| !v.is_empty()).context(
                    "memory backend 'mariadb' requires db_url in [storage.provider.config]",
                )?;
            let mem = super::MariadbMemory::new(
                db_url,
                &sp.schema,
                &sp.table,
                sp.connect_timeout_secs,
                sp.tls,
            )?;
            Ok(Box::new(mem))
        }
        #[cfg(not(feature = "memory-mariadb"))]
        MemoryBackendKind::Mariadb => {
            bail!("memory backend 'mariadb' requires the 'memory-mariadb' feature to be enabled");
        }
        _ => create_memory_for_migration(&backend, &config.workspace_dir),
    }
}

/// 处理列表命令，显示内存条目
///
/// 从内存后端获取条目列表，支持按分类和会话过滤，并提供分页功能。
/// 输出格式为每个条目显示键、分类和内容预览（截断到指定长度）。
///
/// # 参数
///
/// - `config`: 应用配置
/// - `category`: 可选的分类过滤器
/// - `session`: 可选的会话 ID 过滤器
/// - `limit`: 每页显示的最大条目数
/// - `offset`: 跳过的条目数（用于分页）
///
/// # 返回值
///
/// 成功返回 `Ok(())`，结果直接输出到标准输出。
///
/// # 示例输出
///
/// ```text
/// Memory entries (150 total, showing 1-50):
///
/// - user_preferences [core]
///     {"theme": "dark", "language": "zh-CN"}
/// - project_notes [daily]
///     Remember to update documentation...
/// ```
async fn handle_list(
    config: &Config,
    category: Option<String>,
    session: Option<String>,
    limit: usize,
    offset: usize,
) -> Result<()> {
    let mem = create_cli_memory(config)?;
    // 解析分类参数（如果提供）
    let cat = category.as_deref().map(parse_category);
    let entries = mem.list(cat.as_ref(), session.as_deref()).await?;

    // 空结果处理
    if entries.is_empty() {
        println!("No memory entries found.");
        return Ok(());
    }

    let total = entries.len();
    // 客户端分页：跳过 offset 条，最多取 limit 条
    let page: Vec<_> = entries.into_iter().skip(offset).take(limit).collect();

    // 边界情况：请求的偏移量超出范围
    if page.is_empty() {
        println!("No entries at offset {offset} (total: {total}).");
        return Ok(());
    }

    // 输出分页信息
    println!("Memory entries ({total} total, showing {}-{}):\n", offset + 1, offset + page.len(),);

    // 遍历当前页的条目并格式化输出
    for entry in &page {
        println!("- {} [{}]", style(&entry.key).white().bold(), entry.category,);
        println!("    {}", truncate_content(&entry.content, 80));
    }

    // 如果还有更多条目，提示下一页的偏移量
    if offset + page.len() < total {
        println!("\n  Use --offset {} to see the next page.", offset + limit);
    }

    Ok(())
}

/// 处理获取命令，查询指定键的内存条目
///
/// 首先尝试精确匹配键名。如果未找到，则尝试前缀匹配。
/// 前缀匹配到 1 个条目时直接显示，匹配到多个时列出所有匹配项供用户选择。
///
/// # 参数
///
/// - `config`: 应用配置
/// - `key`: 要查询的键名或键前缀
///
/// # 返回值
///
/// 成功返回 `Ok(())`，结果直接输出到标准输出。
///
/// # 示例
///
/// ```text
/// Key:       user_preferences
/// Category:  core
/// Timestamp: 2024-01-15T10:30:00Z
/// Session:   abc123
///
/// {"theme": "dark", "language": "zh-CN"}
/// ```
async fn handle_get(config: &Config, key: &str) -> Result<()> {
    let mem = create_cli_memory(config)?;

    // 首先尝试精确匹配
    if let Some(entry) = mem.get(key).await? {
        print_entry(&entry);
        return Ok(());
    }

    // 精确匹配失败，尝试前缀匹配
    let all = mem.list(None, None).await?;
    let matches: Vec<_> = all.iter().filter(|e| e.key.starts_with(key)).collect();

    match matches.len() {
        0 => println!("No memory entry found for key: {key}"),
        1 => print_entry(matches[0]), // 唯一匹配，直接显示
        n => {
            // 多个匹配，列出所有候选项
            println!("Prefix '{key}' matched {n} entries:\n");
            for entry in matches {
                println!("- {} [{}]", style(&entry.key).white().bold(), entry.category);
            }
            println!("\nSpecify a longer prefix to narrow the match.");
        }
    }

    Ok(())
}

/// 格式化并打印单个内存条目的详细信息
///
/// 以结构化格式输出条目的所有字段，包括键、分类、时间戳、会话 ID 和完整内容。
///
/// # 参数
///
/// - `entry`: 要打印的内存条目引用
fn print_entry(entry: &super::traits::MemoryEntry) {
    println!("Key:       {}", style(&entry.key).white().bold());
    println!("Category:  {}", entry.category);
    println!("Timestamp: {}", entry.timestamp);
    if let Some(sid) = &entry.session_id {
        println!("Session:   {sid}");
    }
    println!("\n{}", entry.content);
}

/// 处理统计命令，显示内存后端的统计信息
///
/// 收集并显示以下信息：
/// - 后端类型和名称
/// - 健康检查状态
/// - 条目总数
/// - 按分类的条目统计（按数量降序排列）
///
/// # 参数
///
/// - `config`: 应用配置
///
/// # 返回值
///
/// 成功返回 `Ok(())`，统计信息输出到标准输出。
///
/// # 示例输出
///
/// ```text
/// Memory Statistics:
///
///   Backend:  sqlite
///   Health:   healthy
///   Total:    1234
///
///   By category:
///     daily              567
///     conversation       432
///     core               235
/// ```
async fn handle_stats(config: &Config) -> Result<()> {
    let mem = create_cli_memory(config)?;
    // 执行健康检查
    let healthy = mem.health_check().await;
    // 获取条目总数（失败时默认为 0）
    let total = mem.count().await.unwrap_or(0);

    println!("Memory Statistics:\n");
    println!("  Backend:  {}", style(mem.name()).white().bold());
    // 根据健康状态显示不同颜色的标识
    println!(
        "  Health:   {}",
        if healthy {
            style("healthy").green().bold().to_string()
        } else {
            style("unhealthy").yellow().bold().to_string()
        }
    );
    println!("  Total:    {total}");

    // 获取所有条目并按分类统计
    let all = mem.list(None, None).await.unwrap_or_default();
    if !all.is_empty() {
        // 使用 HashMap 按分类计数
        let mut counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for entry in &all {
            *counts.entry(entry.category.to_string()).or_default() += 1;
        }

        println!("\n  By category:");
        // 按数量降序排序
        let mut sorted: Vec<_> = counts.into_iter().collect();
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        for (cat, count) in sorted {
            println!("    {cat:<20} {count}");
        }
    }

    Ok(())
}

/// 处理清除命令，删除内存条目
///
/// 支持两种清除模式：
/// 1. 按键清除：删除指定键的条目（委托给 handle_clear_key）
/// 2. 按分类清除：删除指定分类的所有条目（默认清除所有分类）
///
/// 除非指定 --yes 标志，否则会提示用户确认。
///
/// # 参数
///
/// - `config`: 应用配置
/// - `key`: 可选的键名（优先级高于分类）
/// - `category`: 可选的分类名
/// - `yes`: 跳过确认提示
///
/// # 返回值
///
/// 成功返回 `Ok(())`，操作结果输出到标准输出。
///
/// # 平台差异
///
/// - 非 WASM 平台：使用 dialoguer 进行交互式确认
/// - WASM 平台：必须使用 --yes 标志，否则无法确认
async fn handle_clear(
    config: &Config,
    key: Option<String>,
    category: Option<String>,
    yes: bool,
) -> Result<()> {
    let mem = create_cli_memory(config)?;

    // 如果指定了键，优先按键清除
    if let Some(key) = key {
        return handle_clear_key(&*mem, &key, yes).await;
    }

    // 获取要清除的条目列表
    let cat = category.as_deref().map(parse_category);
    let entries = mem.list(cat.as_ref(), None).await?;

    if entries.is_empty() {
        println!("No entries to clear.");
        return Ok(());
    }

    // 显示清除范围
    let scope = category.as_deref().unwrap_or("all categories");
    println!("Found {} entries in '{scope}'.", entries.len());

    // 确认提示（非 WASM 平台）
    if !yes {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let confirmed = dialoguer::Confirm::new()
                .with_prompt(format!("  Delete {} entries?", entries.len()))
                .default(false)
                .interact()?;
            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }
        }
        // WASM 平台不支持交互式确认
        #[cfg(target_arch = "wasm32")]
        {
            println!("Cannot confirm interactively in WASM. Use --yes to force.");
            return Ok(());
        }
    }

    // 批量删除条目
    let mut deleted = 0usize;
    for entry in &entries {
        if mem.forget(&entry.key).await? {
            deleted += 1;
        }
    }

    println!("{} Cleared {deleted}/{} entries.", style("✓").green().bold(), entries.len(),);

    Ok(())
}

/// 清除指定键的内存条目
///
/// 首先尝试精确匹配键名。如果未找到，则尝试前缀匹配。
/// 前缀匹配到 1 个条目时直接删除，匹配到多个时列出所有匹配项供用户细化查询。
///
/// # 参数
///
/// - `mem`: 内存后端 trait 对象引用
/// - `key`: 要删除的键名或键前缀
/// - `yes`: 跳过确认提示
///
/// # 返回值
///
/// 成功返回 `Ok(())`，操作结果输出到标准输出。
async fn handle_clear_key(mem: &dyn Memory, key: &str, yes: bool) -> Result<()> {
    // 确定要删除的目标键
    let target = if mem.get(key).await?.is_some() {
        // 精确匹配成功
        key.to_string()
    } else {
        // 精确匹配失败，尝试前缀匹配
        let all = mem.list(None, None).await?;
        let matches: Vec<_> = all.iter().filter(|e| e.key.starts_with(key)).collect();
        match matches.len() {
            0 => {
                println!("No memory entry found for key: {key}");
                return Ok(());
            }
            1 => matches[0].key.clone(), // 唯一匹配
            n => {
                // 多个匹配，提示用户细化查询
                println!("Prefix '{key}' matched {n} entries:\n");
                for entry in matches {
                    println!("- {} [{}]", style(&entry.key).white().bold(), entry.category);
                }
                println!("\nSpecify a longer prefix to narrow the match.");
                return Ok(());
            }
        }
    };

    // 确认提示（非 WASM 平台）
    if !yes {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let confirmed = dialoguer::Confirm::new()
                .with_prompt(format!("  Delete '{target}'?"))
                .default(false)
                .interact()?;
            if !confirmed {
                println!("Aborted.");
                return Ok(());
            }
        }
        // WASM 平台不支持交互式确认
        #[cfg(target_arch = "wasm32")]
        {
            println!("Cannot confirm interactively in WASM. Use --yes to force.");
            return Ok(());
        }
    }

    // 执行删除并显示结果
    if mem.forget(&target).await? {
        println!("{} Deleted key: {target}", style("✓").green().bold());
    }

    Ok(())
}

/// 将字符串解析为内存分类枚举
///
/// 支持内置分类（core、daily、conversation）和自定义分类。
/// 解析时不区分大小写，会自动去除首尾空白。
///
/// # 参数
///
/// - `s`: 分类名称字符串
///
/// # 返回值
///
/// 对应的 MemoryCategory 枚举值。
///
/// # 示例
///
/// ```
/// parse_category("Core")     // -> MemoryCategory::Core
/// parse_category("DAILY")    // -> MemoryCategory::Daily
/// parse_category("my_custom") // -> MemoryCategory::Custom("my_custom")
/// ```
fn parse_category(s: &str) -> MemoryCategory {
    match s.trim().to_ascii_lowercase().as_str() {
        "core" => MemoryCategory::Core,
        "daily" => MemoryCategory::Daily,
        "conversation" => MemoryCategory::Conversation,
        other => MemoryCategory::Custom(other.to_string()),
    }
}

/// 截断内容字符串以便在终端显示
///
/// 只取第一行内容，如果超过最大长度则截断并添加省略号。
/// 用于在列表视图中显示内容的简短预览。
///
/// # 参数
///
/// - `s`: 原始内容字符串
/// - `max_len`: 最大显示长度（包含省略号）
///
/// # 返回值
///
/// 截断后的字符串，长度不超过 max_len。
///
/// # 示例
///
/// ```
/// truncate_content("Short", 10)       // -> "Short"
/// truncate_content("Very long content here", 10) // -> "Very lo..."
/// truncate_content("Line1\nLine2", 80) // -> "Line1" (只取第一行)
/// ```
fn truncate_content(s: &str, max_len: usize) -> String {
    // 只取第一行
    let line = s.lines().next().unwrap_or(s);
    if line.len() <= max_len {
        return line.to_string();
    }
    // 截断并保留 3 个字符用于省略号
    let truncated: String = line.chars().take(max_len.saturating_sub(3)).collect();
    format!("{truncated}...")
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;


/// 内存管理命令枚举
///
/// 定义所有支持的内存操作命令，作为 clap 子命令使用。
/// 每个变体对应一个具体的内存管理功能。
pub use agent_cli_types::MemoryCommands;
