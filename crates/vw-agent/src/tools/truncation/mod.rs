//! 工具输出截断工具
//!
//! 本模块负责管理工具执行输出的大小限制与截断策略。
//!
//! # 主要功能
//!
//! - **智能截断**：根据行数和字节限制，对超长输出进行截断，支持头部和尾部两种截断方向
//! - **完整保存**：将被截断的完整输出保存到本地文件系统，便于后续分段查看或搜索
//! - **自动清理**：定期清理过期的输出文件（默认保留 7 天）
//! - **智能提示**：生成截断提示，根据代理权限推荐最佳查看方式
//!
//! # 使用场景
//!
//! 当工具执行产生大量输出时（如日志文件、长文本文件等），直接返回可能超出上下文窗口限制。
//! 本模块通过截断 + 保存 + 提示的方式，既保证了核心信息的传递，又提供了访问完整输出的途径。

use crate::app::agent::agent;
use crate::app::agent::global;
use crate::app::agent::id;
use crate::app::agent::scheduler;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 默认最大输出行数
///
/// 当输出超过此行数时，将触发截断机制。
/// 默认值为 2000 行，足以容纳大多数常见输出。
pub const MAX_LINES: usize = 2000;

/// 默认最大输出字节数
///
/// 当输出超过此字节数时，将触发截断机制。
/// 默认值为 50KB (50 * 1024 字节)。
pub const MAX_BYTES: usize = 50 * 1024;

/// 返回工具输出存储目录的路径
///
/// 该目录用于存储被截断的完整工具输出文件。
/// 目录位置为：`<数据目录>/tool-output/`
///
/// # 返回值
///
/// 返回工具输出目录的完整路径
pub fn dir() -> PathBuf {
    global::paths().data.join("tool-output")
}

/// 工具输出文件的保留时长
///
/// 超过此时长的输出文件将在清理任务中被自动删除。
/// 默认值为 7 天。
const RETENTION: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// 清理任务的执行间隔
///
/// 自动清理任务每隔 1 小时执行一次。
const HOUR: Duration = Duration::from_secs(60 * 60);

/// 截断方向枚举
///
/// 定义输出截断时保留内容的方向。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// 从头部开始截断，保留文件开头的部分
    ///
    /// 适用于日志文件开头包含重要信息的场景。
    Head,

    /// 从尾部开始截断，保留文件末尾的部分
    ///
    /// 适用于日志文件末尾包含最新信息的场景。
    Tail,
}

/// 截断选项配置
///
/// 配置输出截断的具体参数。
#[derive(Debug, Clone)]
pub struct Options {
    /// 最大允许的输出行数
    ///
    /// 超过此行数的输出将被截断。
    pub max_lines: usize,

    /// 最大允许的输出字节数
    ///
    /// 超过此字节数的输出将被截断。
    /// 行数和字节数任一超限都会触发截断。
    pub max_bytes: usize,

    /// 截断方向
    ///
    /// 决定保留输出的哪一部分（头部或尾部）。
    pub direction: Direction,
}

impl Default for Options {
    /// 返回默认的截断选项
    ///
    /// 默认配置：
    /// - 最大行数：2000 行
    /// - 最大字节：50KB
    /// - 截断方向：保留头部
    fn default() -> Self {
        Self { max_lines: MAX_LINES, max_bytes: MAX_BYTES, direction: Direction::Head }
    }
}

/// 截断结果
///
/// 包含截断后的输出内容及相关元数据。
#[derive(Debug, Clone)]
pub struct Result {
    /// 截断后的输出内容
    ///
    /// 如果未触发截断，则为原始完整内容。
    pub content: String,

    /// 是否发生了截断
    ///
    /// `true` 表示输出被截断，`false` 表示输出完整返回。
    pub truncated: bool,

    /// 完整输出的保存路径
    ///
    /// 仅在发生截断时存在。保存完整的原始输出到文件，
    /// 以便用户通过其他工具（如 Grep 或分段 Read）访问。
    pub output_path: Option<PathBuf>,
}

/// 获取当前时间的毫秒时间戳
///
/// 使用 UNIX 纪元（1970-01-01 00:00:00 UTC）作为基准。
///
/// # 返回值
///
/// 返回当前时间的毫秒时间戳。如果获取失败，返回 `u64::MAX`。
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .try_into()
        .unwrap_or(u64::MAX)
}

/// 确保指定目录存在
///
/// 如果目录不存在，将递归创建所有必要的父目录。
/// 在 WASM 目标上不执行任何操作（文件系统操作不可用）。
///
/// # 参数
///
/// - `dir`: 需要确保存在的目录路径
fn ensure_dir_exists(dir: &Path) {
    #[cfg(not(target_arch = "wasm32"))]
    let _ = std::fs::create_dir_all(dir);
}

/// 预览计算中间结果
///
/// 用于存储截断预览计算的中间状态。
struct Preview {
    /// 预览文本内容
    text: String,

    /// 预览文本的字节数
    bytes: usize,

    /// 是否因字节限制而停止
    ///
    /// `true` 表示在达到行数限制之前，因字节限制而停止收集。
    hit_bytes: bool,

    /// 预览文本的行数
    line_count: usize,
}

/// 计算输出预览
///
/// 根据截断选项，从原始文本中提取预览部分。
///
/// # 参数
///
/// - `text`: 原始输出文本
/// - `options`: 截断选项配置
///
/// # 返回值
///
/// 返回包含预览文本及其元数据的 `Preview` 结构体
///
/// # 截断逻辑
///
/// 1. **Head 模式**：从第一行开始，逐行累加，直到达到行数或字节限制
/// 2. **Tail 模式**：从最后一行开始，反向累加，直到达到行数或字节限制
///
/// # 注意事项
///
/// - 字节计算包含换行符
/// - 使用 `saturating_add` 防止算术溢出
fn compute_preview(text: &str, options: &Options) -> Preview {
    // 按换行符分割文本为行数组
    let lines: Vec<&str> = text.split('\n').collect();
    // 存储选中输出的行
    let mut out: Vec<&str> = Vec::new();
    // 当前累计字节数
    let mut bytes: usize = 0;
    // 标记是否因字节限制而停止
    let mut hit_bytes = false;

    match options.direction {
        // Head 模式：从开头截取
        Direction::Head => {
            for (i, line) in lines.iter().enumerate() {
                // 检查行数限制
                if i >= options.max_lines {
                    break;
                }
                // 计算本行字节数（包含换行符，第一行除外）
                let size = line.len() + if i > 0 { 1 } else { 0 };
                // 检查字节限制
                if bytes.saturating_add(size) > options.max_bytes {
                    hit_bytes = true;
                    break;
                }
                out.push(line);
                bytes = bytes.saturating_add(size);
            }
        }
        // Tail 模式：从末尾截取
        Direction::Tail => {
            // 反向遍历行数组
            for (j, line) in lines.iter().enumerate().rev() {
                // 检查行数限制
                if out.len() >= options.max_lines {
                    break;
                }
                // 计算本行字节数（包含换行符，非第一行时）
                let size = line.len() + if out.is_empty() { 0 } else { 1 };
                // 检查字节限制
                if bytes.saturating_add(size) > options.max_bytes {
                    hit_bytes = true;
                    break;
                }
                // 插入到输出列表的开头，保持原始顺序
                out.insert(0, lines[j]);
                bytes = bytes.saturating_add(size);
            }
        }
    }

    // 用换行符连接选中行，构建预览文本
    Preview { text: out.join("\n"), bytes, hit_bytes, line_count: out.len() }
}

/// 初始化截断模块
///
/// 注册定期清理任务，自动删除过期的工具输出文件。
///
/// # 线程安全
///
/// 使用 `OnceLock` 确保初始化只执行一次，即使多次调用也是安全的。
///
/// # 平台差异
///
/// - **非 WASM 平台**：注册每小时执行的清理任务
/// - **WASM 平台**：不执行任何操作（任务调度不可用）
pub fn init() {
    static INIT: OnceLock<()> = OnceLock::new();
    INIT.get_or_init(|| {
        #[cfg(not(target_arch = "wasm32"))]
        {
            // 定义清理任务的执行函数
            let run: scheduler::RunFn = std::sync::Arc::new(|| Box::pin(run_cleanup_task()));
            // 注册定时任务
            scheduler::register(scheduler::Task {
                id: "tool.truncation.cleanup".to_string(),
                interval: HOUR,
                run,
                scope: scheduler::Scope::Global,
            });
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
async fn run_cleanup_task() -> std::result::Result<(), String> {
    // 在阻塞线程池中执行清理，避免阻塞异步运行时
    tokio::task::spawn_blocking(cleanup)
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// 清理过期的工具输出文件
///
/// 扫描工具输出目录，删除超过保留期限的文件。
///
/// # 返回值
///
/// - `Ok(())`: 清理成功或无需清理
/// - `Err(e)`: I/O 错误
///
/// # 平台差异
///
/// - **非 WASM 平台**：执行实际的文件清理操作
/// - **WASM 平台**：直接返回 `Ok(())`（文件系统操作不可用）
///
/// # 清理逻辑
///
/// 1. 计算截止时间戳（当前时间 - 保留时长）
/// 2. 生成截止时间对应的 ID
/// 3. 遍历输出目录中的所有文件
/// 4. 对于文件名以 "tool_" 开头的文件，检查其时间戳
/// 5. 删除时间戳早于截止时间的文件
pub fn cleanup() -> std::io::Result<()> {
    #[cfg(target_arch = "wasm32")]
    return Ok(());

    #[cfg(not(target_arch = "wasm32"))]
    {
        cleanup_dir(&dir(), now_ms())
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn cleanup_dir(tool_dir: &Path, now_ms: u64) -> std::io::Result<()> {
    // 确保输出目录存在
    ensure_dir_exists(tool_dir);

    // 计算截止时间戳（当前时间 - 保留时长）
    let retention_ms = u64::try_from(RETENTION.as_millis()).unwrap_or(u64::MAX);
    let cutoff_ts = now_ms.saturating_sub(retention_ms);
    // 生成截止时间对应的工具 ID
    let cutoff_id =
        id::create(id::Prefix::Tool, false, Some(cutoff_ts)).unwrap_or_else(|_| "tool".to_string());
    // 提取截止 ID 的时间戳
    let cutoff = id::timestamp(&cutoff_id).unwrap_or(0);

    // 读取目录内容
    let Ok(entries) = std::fs::read_dir(tool_dir) else {
        return Ok(());
    };

    // 遍历目录中的条目
    for entry in entries.flatten() {
        let path = entry.path();

        // 只处理文件，跳过目录
        if !path.is_file() {
            continue;
        }

        // 提取文件名
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        // 只处理以 "tool_" 开头的文件
        if !name.starts_with("tool_") {
            continue;
        }

        // 从文件名提取时间戳
        let Some(ts) = id::timestamp(name) else {
            continue;
        };

        // 跳过未过期的文件
        if ts >= cutoff {
            continue;
        }

        // 删除过期文件，忽略错误
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

/// 处理工具输出
///
/// 检查输出大小，必要时进行截断并保存完整内容。
///
/// # 参数
///
/// - `text`: 工具执行的原始输出文本
/// - `options`: 截断选项配置
/// - `agent`: 可选的代理信息，用于生成针对性的提示信息
///
/// # 返回值
///
/// 返回 `Result` 结构体，包含：
/// - `content`: 最终输出内容（可能已截断）
/// - `truncated`: 是否发生了截断
/// - `output_path`: 完整输出的保存路径（仅截断时存在）
///
/// # 处理流程
///
/// 1. **检查是否需要截断**：如果输出在行数和字节限制内，直接返回
/// 2. **计算预览**：根据截断方向提取预览部分
/// 3. **保存完整输出**：将原始完整输出保存到文件
/// 4. **生成提示信息**：根据代理权限生成访问建议
/// 5. **组合最终输出**：预览 + 截断提示 + 访问建议
///
/// # 平台差异
///
/// - **非 WASM 平台**：完整功能，包括文件保存
/// - **WASM 平台**：跳过文件保存步骤
pub fn output(text: &str, options: Options, _agent: Option<&agent::Info>) -> Result {
    let tool_dir = dir();
    output_with_dir(text, options, &tool_dir)
}

fn output_with_dir(text: &str, options: Options, tool_dir: &Path) -> Result {
    // 计算原始输出的总字节数和总行数
    let total_bytes = text.len();
    let total_lines = text.split('\n').count();

    // 如果输出在限制内，直接返回原始内容
    if total_lines <= options.max_lines && total_bytes <= options.max_bytes {
        return Result { content: text.to_string(), truncated: false, output_path: None };
    }

    // 计算预览内容
    let preview = compute_preview(text, &options);

    // 计算被截断的部分（字节数或行数）
    let removed = if preview.hit_bytes {
        total_bytes.saturating_sub(preview.bytes)
    } else {
        total_lines.saturating_sub(preview.line_count)
    };

    // 确定截断单位（字节或行）
    let unit = if preview.hit_bytes { "bytes" } else { "lines" };

    // 确保输出目录存在
    ensure_dir_exists(tool_dir);

    // 尝试保存完整输出到文件
    let mut output_path: Option<PathBuf> = None;
    #[cfg(not(target_arch = "wasm32"))]
    if let Ok(id) = id::ascending(id::Prefix::Tool, None) {
        let filepath = tool_dir.join(id);
        if std::fs::write(&filepath, text).is_ok() {
            output_path = Some(filepath);
        }
    }

    // 生成用户提示信息
    let hint = match output_path.as_ref() {
        Some(p) => {
            let full = p.to_string_lossy();
            format!(
                "工具调用已成功，但输出已截断。完整输出已保存到：{}\n你可以用 Grep 在完整内容里搜索，或用 Read（offset/limit）分段查看；如果已启用 delegate/subagent 工具，也可以委托专用子代理继续处理该文件。",
                full
            )
        }
        // 文件保存失败的提示
        None => "工具调用已成功，但输出已截断，且完整输出保存失败。\n你可以用 Grep 搜索，或用 Read（offset/limit）分段查看。".to_string(),
    };

    // 根据截断方向组合最终输出
    let content = match options.direction {
        // Head 模式：预览在前，提示在后
        Direction::Head => {
            format!("{}\n\n...已截断 {} {}...\n\n{}", preview.text, removed, unit, hint)
        }
        // Tail 模式：提示在前，预览在后
        Direction::Tail => {
            format!("...已截断 {} {}...\n\n{}\n\n{}", removed, unit, hint, preview.text)
        }
    };

    Result { content, truncated: true, output_path }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod mod_tests;
#[cfg(test)]
#[path = "tests.rs"]
mod tests;
