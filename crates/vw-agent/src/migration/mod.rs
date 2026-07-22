//! # 数据迁移模块
//!
//! 本模块提供从外部系统（如 OpenClaw）向 VibeWindow 迁移数据的功能。
//!
//! ## 主要功能
//!
//! - **OpenClaw 记忆迁移**：将 OpenClaw 的记忆数据（SQLite 数据库和 Markdown 文件）
//!   迁移到 VibeWindow 的记忆系统中
//! - **智能冲突处理**：当目标键已存在时，自动重命名以避免数据覆盖
//! - **增量迁移**：跳过内容未变化的条目，避免重复导入
//! - **安全备份**：在迁移前自动备份目标工作区的现有记忆数据
//!
//! ## 支持的源数据格式
//!
//! 1. **SQLite 数据库**：`memory/brain.db` 中的 `memories` 表
//! 2. **核心记忆文件**：工作区根目录的 `MEMORY.md`
//! 3. **日常记忆文件**：`memory/` 目录下的 `.md` 文件
//!
//! ## 使用示例
//!
//! ```bash
//! # 从默认 OpenClaw 工作区迁移
//! vibe-agent migrate openclaw
//!
//! # 从指定路径迁移
//! vibe-agent migrate openclaw --source /path/to/openclaw/workspace
//!
//! # 预览迁移结果（不实际执行）
//! vibe-agent migrate openclaw --dry-run
//! ```

use crate::app::agent::config::Config;
use crate::memory::{self, Memory, MemoryCategory};
use anyhow::{Context, Result, bail};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

/// 源系统中的单条记忆条目
///
/// 表示从源系统（如 OpenClaw）读取的一条记忆数据，
/// 包含键、内容和分类信息。
#[derive(Debug, Clone)]
struct SourceEntry {
    /// 记忆条目的唯一标识键
    key: String,
    /// 记忆条目的文本内容
    content: String,
    /// 记忆条目的分类（核心、日常、对话或自定义）
    category: MemoryCategory,
}

/// 迁移统计信息
///
/// 记录迁移过程中各项操作的计数，用于在迁移完成后
/// 向用户展示详细的迁移报告。
#[derive(Debug, Default)]
struct MigrationStats {
    /// 从 SQLite 数据库读取的条目数
    from_sqlite: usize,
    /// 从 Markdown 文件读取的条目数
    from_markdown: usize,
    /// 成功导入到目标系统的条目数
    imported: usize,
    /// 因内容未变化而跳过的条目数
    skipped_unchanged: usize,
    /// 因键冲突而重命名的条目数
    renamed_conflicts: usize,
}

/// 源工作区类型
///
/// 区分用户显式指定的工作区路径和系统自动推断的默认路径，
/// 用于在源工作区不存在时提供不同的错误处理策略。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceWorkspaceKind {
    /// 用户通过 --source 参数显式指定的工作区
    Explicit,
    /// 系统根据环境变量或默认路径推断的工作区
    Default,
}

/// 源工作区信息
///
/// 包含源工作区的路径及其类型（显式指定或默认推断）。
#[derive(Debug, Clone)]
struct SourceWorkspace {
    /// 工作区的文件系统路径
    path: PathBuf,
    /// 工作区类型
    kind: SourceWorkspaceKind,
}

/// 处理迁移命令的入口函数
///
/// 根据命令类型路由到对应的迁移处理逻辑。
/// 当前仅支持 OpenClaw 迁移命令。
///
/// # 参数
///
/// * `command` - 迁移命令枚举，包含具体的迁移参数
/// * `config` - VibeWindow 的配置对象
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 示例
///
/// ```ignore
/// let command = MigrateCommands::Openclaw { source: None, dry_run: false };
/// handle_command(command, &config).await?;
/// ```
pub async fn handle_command(command: crate::MigrateCommands, config: &Config) -> Result<()> {
    match command {
        crate::MigrateCommands::Openclaw { source, dry_run } => {
            migrate_openclaw_memory(config, source, dry_run).await
        }
    }
}

/// 执行 OpenClaw 记忆迁移
///
/// 从 OpenClaw 工作区读取记忆数据并导入到 VibeWindow 工作区。
/// 支持从 SQLite 数据库和 Markdown 文件两种源格式读取数据。
///
/// # 迁移流程
///
/// 1. 解析并验证源工作区路径
/// 2. 检查源工作区是否存在
/// 3. 防止自迁移（源和目标为同一工作区）
/// 4. 收集源工作区中的所有记忆条目
/// 5. 如果是 dry-run 模式，仅显示预览并返回
/// 6. 备份目标工作区的现有记忆数据
/// 7. 逐条导入记忆，处理键冲突和内容去重
/// 8. 输出迁移统计报告
///
/// # 参数
///
/// * `config` - VibeWindow 配置对象，包含目标工作区路径和记忆后端配置
/// * `source_workspace` - 可选的源工作区路径，为 None 时使用默认 OpenClaw 工作区
/// * `dry_run` - 是否为预览模式，true 时仅显示将要执行的操作而不实际迁移
///
/// # 返回值
///
/// 成功时返回 `Ok(())`，失败时返回错误信息
///
/// # 错误
///
/// - 源工作区与目标工作区相同时返回错误
/// - 显式指定的源工作区不存在时返回错误
/// - 数据库读取或文件操作失败时返回错误
async fn migrate_openclaw_memory(
    config: &Config,
    source_workspace: Option<PathBuf>,
    dry_run: bool,
) -> Result<()> {
    // 解析源工作区路径（显式指定或使用默认值）
    let source_workspace = resolve_openclaw_workspace(config, source_workspace);

    // 如果源工作区不存在，根据工作区类型进行不同处理
    if !source_workspace.path.exists() {
        return handle_missing_source_workspace(config, &source_workspace, dry_run);
    }

    // 防止自迁移：源和目标不能是同一个工作区
    if paths_equal(&source_workspace.path, &config.workspace_dir) {
        bail!("Source workspace matches current VibeWindow workspace; refusing self-migration");
    }

    // 初始化迁移统计信息
    let mut stats = MigrationStats::default();

    // 从源工作区收集所有可迁移的记忆条目
    let entries = collect_source_entries(&source_workspace.path, &mut stats)?;

    // 如果没有找到任何可迁移的条目，提前返回
    if entries.is_empty() {
        println!("No importable memory found in {}", source_workspace.path.display());
        println!("Checked for: memory/brain.db, MEMORY.md, memory/*.md");
        return Ok(());
    }

    // dry-run 模式：仅显示预览，不执行实际迁移
    if dry_run {
        print_dry_run_preview(
            &source_workspace.path,
            &config.workspace_dir,
            entries.len(),
            &stats,
            false,
        );
        return Ok(());
    }

    // 在迁移前备份目标工作区的现有记忆数据
    if let Some(backup_dir) = backup_target_memory(&config.workspace_dir)? {
        println!("🛟 Backup created: {}", backup_dir.display());
    }

    // 获取目标记忆后端
    let memory = target_memory_backend(config)?;

    // 逐条处理并导入记忆条目
    for (idx, entry) in entries.into_iter().enumerate() {
        // 规范化键名，处理空键情况
        let mut key = entry.key.trim().to_string();
        if key.is_empty() {
            key = format!("openclaw_{idx}");
        }

        // 检查目标位置是否已存在相同键的条目
        if let Some(existing) = memory.get(&key).await? {
            // 如果内容完全相同，跳过此条目
            if existing.content.trim() == entry.content.trim() {
                stats.skipped_unchanged += 1;
                continue;
            }

            // 内容不同时，为新条目生成一个新的唯一键名
            let renamed = next_available_key(memory.as_ref(), &key).await?;
            key = renamed;
            stats.renamed_conflicts += 1;
        }

        // 存储记忆条目到目标后端
        memory.store(&key, &entry.content, entry.category, None).await?;
        stats.imported += 1;
    }

    // 输出迁移完成报告
    println!("✅ OpenClaw memory migration complete");
    println!("  Source: {}", source_workspace.path.display());
    println!("  Target: {}", config.workspace_dir.display());
    println!("  Imported:         {}", stats.imported);
    println!("  Skipped unchanged:{}", stats.skipped_unchanged);
    println!("  Renamed conflicts:{}", stats.renamed_conflicts);
    println!("  Source sqlite rows:{}", stats.from_sqlite);
    println!("  Source markdown:   {}", stats.from_markdown);

    Ok(())
}

/// 创建目标记忆后端实例
///
/// 根据配置中的记忆后端类型，创建对应的记忆存储实例。
/// 该实例用于存储迁移过来的记忆数据。
///
/// # 参数
///
/// * `config` - VibeWindow 配置对象
///
/// # 返回值
///
/// 返回装箱的记忆后端 trait 对象
fn target_memory_backend(config: &Config) -> Result<Box<dyn Memory>> {
    memory::create_memory_for_migration(&config.memory.backend, &config.workspace_dir)
}

/// 从源工作区收集所有记忆条目
///
/// 扫描源工作区中的 SQLite 数据库和 Markdown 文件，
/// 提取所有可迁移的记忆条目，并更新统计信息。
///
/// # 收集来源
///
/// 1. `memory/brain.db` - SQLite 数据库中的 memories 表
/// 2. `MEMORY.md` - 核心记忆文件
/// 3. `memory/*.md` - 日常记忆 Markdown 文件
///
/// # 参数
///
/// * `source_workspace` - 源工作区路径
/// * `stats` - 迁移统计信息，会被更新
///
/// # 返回值
///
/// 成功时返回去重后的记忆条目列表
fn collect_source_entries(
    source_workspace: &Path,
    stats: &mut MigrationStats,
) -> Result<Vec<SourceEntry>> {
    let mut entries = Vec::new();

    // 读取 SQLite 数据库中的记忆条目
    let sqlite_path = source_workspace.join("memory").join("brain.db");
    let sqlite_entries = read_openclaw_sqlite_entries(&sqlite_path)?;
    stats.from_sqlite = sqlite_entries.len();
    entries.extend(sqlite_entries);

    // 读取 Markdown 文件中的记忆条目
    let markdown_entries = read_openclaw_markdown_entries(source_workspace)?;
    stats.from_markdown = markdown_entries.len();
    entries.extend(markdown_entries);

    // 去除完全重复的条目，确保重复运行时的行为确定性
    // 使用键、内容和分类的组合作为唯一性签名
    let mut seen = HashSet::new();
    entries.retain(|entry| {
        let sig = format!("{}\u{0}{}\u{0}{}", entry.key, entry.content, entry.category);
        seen.insert(sig)
    });

    Ok(entries)
}

/// 从 OpenClaw SQLite 数据库读取记忆条目
///
/// 打开指定的 SQLite 数据库文件，从 `memories` 表中读取所有记忆条目。
/// 该函数能够自适应不同版本的 OpenClaw 数据库模式，
/// 通过列名匹配来确定键、内容和分类字段。
///
/// # 支持的列名映射
///
/// - **键列**：优先匹配 `key`、`id`、`name`，回退到 `rowid`
/// - **内容列**：匹配 `content`、`value`、`text`、`memory`（必须存在至少一个）
/// - **分类列**：匹配 `category`、`kind`、`type`，回退到默认值 `'core'`
///
/// # 参数
///
/// * `db_path` - SQLite 数据库文件路径（通常是 `memory/brain.db`）
///
/// # 返回值
///
/// 成功时返回记忆条目列表；如果文件不存在或表不存在，返回空列表
///
/// # 错误
///
/// - 数据库打开失败时返回错误
/// - 找不到内容类列时返回错误
fn read_openclaw_sqlite_entries(db_path: &Path) -> Result<Vec<SourceEntry>> {
    // 如果数据库文件不存在，返回空列表
    if !db_path.exists() {
        return Ok(Vec::new());
    }

    // 以只读模式打开数据库连接
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("Failed to open source db {}", db_path.display()))?;

    // 检查 memories 表是否存在
    let table_exists: Option<String> = conn
        .query_row(
            "SELECT name FROM sqlite_master WHERE type='table' AND name='memories' LIMIT 1",
            [],
            |row| row.get(0),
        )
        .optional()?;

    // 如果表不存在，返回空列表
    if table_exists.is_none() {
        return Ok(Vec::new());
    }

    // 获取表的列信息，用于自适应不同版本的数据库模式
    let columns = table_columns(&conn, "memories")?;

    // 根据可用列名确定 SQL 表达式
    let key_expr = pick_column_expr(&columns, &["key", "id", "name"], "CAST(rowid AS TEXT)");
    let Some(content_expr) =
        pick_optional_column_expr(&columns, &["content", "value", "text", "memory"])
    else {
        bail!("OpenClaw memories table found but no content-like column was detected");
    };
    let category_expr = pick_column_expr(&columns, &["category", "kind", "type"], "'core'");

    // 构建动态 SQL 查询
    let sql = format!(
        "SELECT {key_expr} AS key, {content_expr} AS content, {category_expr} AS category FROM memories ORDER BY rowid ASC"
    );

    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query([])?;

    let mut entries = Vec::new();
    let mut idx = 0_usize;

    // 遍历结果集并构建记忆条目
    while let Some(row) = rows.next()? {
        // 读取键，如果失败则生成默认键
        let key: String = row.get(0).unwrap_or_else(|_| format!("openclaw_sqlite_{idx}"));
        // 读取内容，如果失败则使用空字符串
        let content: String = row.get(1).unwrap_or_default();
        // 读取分类，如果失败则默认为 "core"
        let category_raw: String = row.get(2).unwrap_or_else(|_| "core".to_string());

        // 跳过内容为空的条目
        if content.trim().is_empty() {
            continue;
        }

        entries.push(SourceEntry {
            key: normalize_key(&key, idx),
            content: content.trim().to_string(),
            category: parse_category(&category_raw),
        });

        idx += 1;
    }

    Ok(entries)
}

/// 从 OpenClaw Markdown 文件读取记忆条目
///
/// 扫描源工作区中的 Markdown 文件并提取记忆条目：
/// - `MEMORY.md` 作为核心记忆
/// - `memory/*.md` 作为日常记忆
///
/// # 参数
///
/// * `source_workspace` - 源工作区路径
///
/// # 返回值
///
/// 成功时返回从所有 Markdown 文件中提取的记忆条目列表
fn read_openclaw_markdown_entries(source_workspace: &Path) -> Result<Vec<SourceEntry>> {
    let mut all = Vec::new();

    // 读取核心记忆文件 MEMORY.md
    let core_path = source_workspace.join("MEMORY.md");
    if core_path.exists() {
        let content = fs::read_to_string(&core_path)?;
        all.extend(parse_markdown_file(
            &core_path,
            &content,
            MemoryCategory::Core,
            "openclaw_core",
        ));
    }

    // 读取 memory 目录下的所有 Markdown 文件（日常记忆）
    let daily_dir = source_workspace.join("memory");
    if daily_dir.exists() {
        let mut markdown_paths = Vec::new();
        // 收集所有 .md 文件路径
        for file in fs::read_dir(&daily_dir)? {
            let path = file?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("md") {
                markdown_paths.push(path);
            }
        }
        // 按文件名排序以确保迁移顺序的可确定性
        markdown_paths.sort();

        // 解析每个 Markdown 文件
        for path in markdown_paths {
            let content = fs::read_to_string(&path)?;
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("openclaw_daily");
            all.extend(parse_markdown_file(&path, &content, MemoryCategory::Daily, stem));
        }
    }

    Ok(all)
}

/// 解析 Markdown 文件中的记忆条目
///
/// 逐行解析 Markdown 文件内容，提取非空、非标题的文本行作为记忆条目。
/// 支持两种格式：
/// 1. 结构化格式：`**键名**: 内容`（使用 `parse_structured_memory_line` 解析）
/// 2. 列表格式：`- 内容` 或纯文本行（自动生成键名）
///
/// # 参数
///
/// * `_path` - 文件路径（当前未使用，保留用于日志记录）
/// * `content` - Markdown 文件的文本内容
/// * `default_category` - 条目的默认分类
/// * `stem` - 文件基本名，用于生成自动键名
///
/// # 返回值
///
/// 返回从文件中提取的记忆条目列表
#[allow(clippy::needless_pass_by_value)]
fn parse_markdown_file(
    _path: &Path,
    content: &str,
    default_category: MemoryCategory,
    stem: &str,
) -> Vec<SourceEntry> {
    let mut entries = Vec::new();

    // 逐行处理文件内容
    for (idx, raw_line) in content.lines().enumerate() {
        let trimmed = raw_line.trim();

        // 跳过空行和标题行
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        // 移除列表项的前缀 "- "
        let line = trimmed.strip_prefix("- ").unwrap_or(trimmed);

        // 尝试解析结构化格式的行（**键名**: 内容）
        let (key, text) = match parse_structured_memory_line(line) {
            Some((k, v)) => (normalize_key(k, idx), v.trim().to_string()),
            None => (
                // 非结构化行：自动生成键名
                format!("openclaw_{stem}_{}", idx + 1),
                line.trim().to_string(),
            ),
        };

        // 跳过内容为空的条目
        if text.is_empty() {
            continue;
        }

        entries.push(SourceEntry { key, content: text, category: default_category.clone() });
    }

    entries
}

/// 解析结构化的记忆行
///
/// 尝试解析格式为 `**键名**: 内容` 的文本行。
/// 这种格式允许用户在 Markdown 中明确指定记忆条目的键名。
///
/// # 参数
///
/// * `line` - 待解析的文本行
///
/// # 返回值
///
/// 如果行符合结构化格式，返回 `Some((键名, 内容))`；否则返回 `None`
///
/// # 示例
///
/// ```ignore
/// parse_structured_memory_line("**用户偏好**: 喜欢使用深色主题")
/// // 返回 Some(("用户偏好", "喜欢使用深色主题"))
/// ```
fn parse_structured_memory_line(line: &str) -> Option<(&str, &str)> {
    // 必须以 "**" 开头
    if !line.starts_with("**") {
        return None;
    }

    // 移除开头的 "**"
    let rest = line.strip_prefix("**")?;

    // 查找结束的 "**:" 标记
    let key_end = rest.find("**:")?;
    let key = rest.get(..key_end)?.trim();
    let value = rest.get(key_end + 3..)?.trim();

    // 键和值都不能为空
    if key.is_empty() || value.is_empty() {
        return None;
    }

    Some((key, value))
}

/// 解析记忆分类字符串
///
/// 将原始分类字符串转换为 `MemoryCategory` 枚举值。
/// 支持标准分类名称，未知分类将被映射为自定义分类。
///
/// # 参数
///
/// * `raw` - 原始分类字符串
///
/// # 返回值
///
/// 对应的 `MemoryCategory` 枚举值
fn parse_category(raw: &str) -> MemoryCategory {
    match raw.trim().to_ascii_lowercase().as_str() {
        "core" | "" => MemoryCategory::Core,
        "daily" => MemoryCategory::Daily,
        "conversation" => MemoryCategory::Conversation,
        other => MemoryCategory::Custom(other.to_string()),
    }
}

/// 规范化记忆条目的键名
///
/// 清理键名中的空白字符。如果键名为空或仅包含空白，
/// 则生成一个基于索引的默认键名。
///
/// # 参数
///
/// * `key` - 原始键名
/// * `fallback_idx` - 回退索引用于生成默认键名
///
/// # 返回值
///
/// 规范化后的键名
fn normalize_key(key: &str, fallback_idx: usize) -> String {
    let trimmed = key.trim();
    if trimmed.is_empty() {
        return format!("openclaw_{fallback_idx}");
    }
    trimmed.to_string()
}

/// 生成下一个可用的非冲突键名
///
/// 当目标位置已存在相同键名的条目时，通过添加后缀来生成新的唯一键名。
/// 后缀格式为 `__openclaw_N`，其中 N 是从 1 开始的序号。
///
/// # 参数
///
/// * `memory` - 记忆后端实例
/// * `base` - 基础键名
///
/// # 返回值
///
/// 成功时返回可用的键名；如果尝试超过 10000 次仍未找到可用键名，返回错误
///
/// # 示例
///
/// ```ignore
/// // 如果 "user_preference" 已存在，尝试 "user_preference__openclaw_1"
/// let key = next_available_key(&memory, "user_preference").await?;
/// ```
async fn next_available_key(memory: &dyn Memory, base: &str) -> Result<String> {
    // 尝试最多 10000 个后缀变体
    for i in 1..=10_000 {
        let candidate = format!("{base}__openclaw_{i}");
        if memory.get(&candidate).await?.is_none() {
            return Ok(candidate);
        }
    }

    bail!("Unable to allocate non-conflicting key for '{base}'")
}

/// 获取数据库表的列名列表
///
/// 执行 SQLite PRAGMA 命令获取指定表的所有列名。
///
/// # 参数
///
/// * `conn` - SQLite 数据库连接
/// * `table` - 表名
///
/// # 返回值
///
/// 成功时返回列名列表（已转换为小写）
fn table_columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut stmt = conn.prepare(&pragma)?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;

    let mut cols = Vec::new();
    for col in rows {
        cols.push(col?.to_ascii_lowercase());
    }

    Ok(cols)
}

/// 尝试从候选列名中选择存在的列
///
/// 按顺序检查候选列名是否存在于表的列名列表中，
/// 返回第一个匹配的列名。
///
/// # 参数
///
/// * `columns` - 表的所有列名（小写）
/// * `candidates` - 候选列名列表（按优先级排序）
///
/// # 返回值
///
/// 如果找到匹配的列名，返回 `Some(列名)`；否则返回 `None`
fn pick_optional_column_expr(columns: &[String], candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .find(|candidate| columns.iter().any(|c| c == *candidate))
        .map(std::string::ToString::to_string)
}

/// 从候选列名中选择存在的列，或使用回退值
///
/// 类似于 `pick_optional_column_expr`，但如果没有找到匹配的列名，
/// 则使用提供的回退值。
///
/// # 参数
///
/// * `columns` - 表的所有列名（小写）
/// * `candidates` - 候选列名列表（按优先级排序）
/// * `fallback` - 未找到匹配时的回退 SQL 表达式
///
/// # 返回值
///
/// 返回匹配的列名或回退表达式
fn pick_column_expr(columns: &[String], candidates: &[&str], fallback: &str) -> String {
    pick_optional_column_expr(columns, candidates).unwrap_or_else(|| fallback.to_string())
}

/// 解析 OpenClaw 源工作区路径
///
/// 确定要迁移的 OpenClaw 工作区路径：
/// 1. 如果用户显式指定了路径，使用指定路径
/// 2. 否则尝试从环境变量推断默认路径
/// 3. 如果环境变量未设置，使用相对于当前工作区的默认路径
///
/// # 参数
///
/// * `config` - VibeWindow 配置对象
/// * `source` - 用户通过 --source 参数指定的可选路径
///
/// # 返回值
///
/// 返回源工作区信息（包含路径和类型）
fn resolve_openclaw_workspace(config: &Config, source: Option<PathBuf>) -> SourceWorkspace {
    // 如果用户显式指定了源路径
    if let Some(src) = source {
        return SourceWorkspace { path: src, kind: SourceWorkspaceKind::Explicit };
    }

    // 尝试从环境变量获取默认路径，或使用相对于当前工作区的路径
    let path = default_openclaw_workspace_from_env()
        .unwrap_or_else(|| config.workspace_dir.join(".openclaw").join("workspace"));

    SourceWorkspace { path, kind: SourceWorkspaceKind::Default }
}

/// 从环境变量获取默认 OpenClaw 工作区路径
///
/// 查找 `HOME`（Unix）或 `USERPROFILE`（Windows）环境变量，
/// 构建 `~/.openclaw/workspace` 路径。
///
/// # 返回值
///
/// 如果环境变量存在且非空，返回 `Some(路径)`；否则返回 `None`
fn default_openclaw_workspace_from_env() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .filter(|value| !value.is_empty())
        .map(|home| PathBuf::from(home).join(".openclaw").join("workspace"))
}

/// 比较两个路径是否相等
///
/// 尝试将两个路径规范化（解析符号链接等）后进行比较。
/// 如果规范化失败（例如路径不存在），则直接比较原始路径。
///
/// # 参数
///
/// * `a` - 第一个路径
/// * `b` - 第二个路径
///
/// # 返回值
///
/// 如果两个路径指向同一位置，返回 `true`
fn paths_equal(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

/// 打印 dry-run 模式的预览信息
///
/// 在不执行实际迁移的情况下，向用户展示将要执行的操作概览。
///
/// # 参数
///
/// * `source_workspace` - 源工作区路径
/// * `target_workspace` - 目标工作区路径
/// * `candidates` - 待迁移的条目总数
/// * `stats` - 迁移统计信息
/// * `skipped_memory_migration` - 是否因源工作区不存在而跳过了迁移
fn print_dry_run_preview(
    source_workspace: &Path,
    target_workspace: &Path,
    candidates: usize,
    stats: &MigrationStats,
    skipped_memory_migration: bool,
) {
    println!("🔎 Dry run: OpenClaw migration preview");
    println!("  Source: {}", source_workspace.display());
    println!("  Target: {}", target_workspace.display());
    println!("  Candidates: {}", candidates);
    println!("    - from sqlite:   {}", stats.from_sqlite);
    println!("    - from markdown: {}", stats.from_markdown);
    println!();

    // 如果迁移被跳过，显示相关说明
    if skipped_memory_migration {
        println!("  Notes:");
        println!("    - memory migration skipped: default OpenClaw workspace not found");
        println!("    - config migration skipped: no config import is performed by this command");
        println!();
    }
    println!("Run without --dry-run to import these entries.");
}

/// 处理源工作区不存在的情况
///
/// 根据源工作区的类型（显式指定或默认推断）采取不同的处理策略：
/// - 显式指定的路径不存在时，返回错误
/// - 默认推断的路径不存在时，在 dry-run 模式下显示预览，否则显示友好提示
///
/// # 参数
///
/// * `config` - VibeWindow 配置对象
/// * `source_workspace` - 源工作区信息
/// * `dry_run` - 是否为 dry-run 模式
///
/// # 返回值
///
/// 成功时返回 `Ok(())`；显式指定的路径不存在时返回错误
fn handle_missing_source_workspace(
    config: &Config,
    source_workspace: &SourceWorkspace,
    dry_run: bool,
) -> Result<()> {
    // 如果用户显式指定了路径但路径不存在，返回错误
    if source_workspace.kind == SourceWorkspaceKind::Explicit {
        bail!(
            "OpenClaw workspace not found at {}. Pass --source <path> if needed.",
            source_workspace.path.display()
        );
    }

    // dry-run 模式下显示预览
    if dry_run {
        print_dry_run_preview(
            &source_workspace.path,
            &config.workspace_dir,
            0,
            &MigrationStats::default(),
            true,
        );
        return Ok(());
    }

    // 默认路径不存在时的友好提示
    println!("No OpenClaw workspace found at default source {}", source_workspace.path.display());
    println!("Skipping memory migration because no default source data is available.");
    println!("Config migration skipped: this command only imports memory entries.");
    Ok(())
}

/// 备份目标工作区的现有记忆数据
///
/// 在迁移开始前，将目标工作区中现有的记忆文件复制到备份目录。
/// 备份目录位于 `memory/migrations/openclaw-{timestamp}/`。
///
/// # 备份的文件
///
/// 1. `memory/brain.db` - SQLite 数据库文件
/// 2. `MEMORY.md` - 核心记忆文件
/// 3. `memory/*.md` - 所有日常记忆 Markdown 文件
///
/// # 参数
///
/// * `workspace_dir` - 目标工作区路径
///
/// # 返回值
///
/// - 如果备份了任何文件，返回 `Some(备份目录路径)`
/// - 如果没有需要备份的文件，返回 `None`
fn backup_target_memory(workspace_dir: &Path) -> Result<Option<PathBuf>> {
    // 生成带时间戳的备份目录名
    let timestamp = chrono::Local::now().format("%Y%m%d-%H%M%S").to_string();
    let backup_root =
        workspace_dir.join("memory").join("migrations").join(format!("openclaw-{timestamp}"));

    let mut copied_any = false;
    fs::create_dir_all(&backup_root)?;

    // 需要备份的主要文件列表
    let files_to_copy =
        [workspace_dir.join("memory").join("brain.db"), workspace_dir.join("MEMORY.md")];

    // 复制主要文件
    for source in files_to_copy {
        if source.exists() {
            let Some(name) = source.file_name() else {
                continue;
            };
            fs::copy(&source, backup_root.join(name))?;
            copied_any = true;
        }
    }

    // 备份 memory 目录下的所有 Markdown 文件
    let daily_dir = workspace_dir.join("memory");
    if daily_dir.exists() {
        let daily_backup = backup_root.join("daily");
        for file in fs::read_dir(&daily_dir)? {
            let file = file?;
            let path = file.path();
            // 只备份 .md 文件
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                continue;
            }
            fs::create_dir_all(&daily_backup)?;
            let Some(name) = path.file_name() else {
                continue;
            };
            fs::copy(&path, daily_backup.join(name))?;
            copied_any = true;
        }
    }

    // 如果没有备份任何文件，删除空的备份目录
    if copied_any {
        Ok(Some(backup_root))
    } else {
        let _ = fs::remove_dir_all(&backup_root);
        Ok(None)
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
