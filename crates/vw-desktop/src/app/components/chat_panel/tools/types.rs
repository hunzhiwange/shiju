//! 聊天面板工具类型定义模块
//!
//! 本模块定义了聊天面板工具系统使用的核心数据类型，包括：
//! - 文件变更信息结构
//! - 探索项数据结构
//! - 工具索引常量
//!
//! 这些类型用于在不同工具组件之间传递和共享数据。

/// 文件变更信息结构体
///
/// 用于表示单个文件的变更详情，包括文件路径、增删行数以及变更前后的内容。
/// 通常在代码审查、差异展示或版本控制相关功能中使用。
///
/// # 字段说明
///
/// * `path` - 文件路径，相对于项目根目录
/// * `additions` - 新增行数
/// * `deletions` - 删除行数
/// * `before` - 变更前的文件内容
/// * `after` - 变更后的文件内容
///
/// # 示例
///
/// ```rust,ignore
/// use crate::app::components::chat_panel::tools::types::ChangeFile;
///
/// let change = ChangeFile {
///     path: "src/main.rs".to_string(),
///     additions: 10,
///     deletions: 5,
///     before: "old content".to_string(),
///     after: "new content".to_string(),
/// };
/// ```
#[derive(Clone)]
pub struct ChangeFile {
    /// 文件路径（相对路径）
    pub path: String,
    /// 新增的代码行数
    pub additions: usize,
    /// 删除的代码行数
    pub deletions: usize,
    /// 变更前的完整文件内容
    pub before: String,
    /// 变更后的完整文件内容
    pub after: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChangeFileSummary {
    pub kind: char,
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
}

/// 探索项结构体
///
/// 用于表示工具探索过程中发现的单个项目，包含工具索引和原始数据引用。
/// 这是一个零拷贝结构，通过生命周期参数引用外部数据以提高性能。
///
/// # 类型参数
///
/// * `'a` - 数据引用的生命周期，与被引用的原始数据绑定
///
/// # 字段说明
///
/// * `tool_idx` - 产生此项的工具索引，用于追溯数据来源
/// * `raw` - 原始数据字符串
///
/// # 示例
///
/// ```rust,ignore
/// use crate::app::components::chat_panel::tools::types::ExploreItem;
///
/// let data = "some raw data";
/// let item = ExploreItem {
///     tool_idx: 0,
///     raw: data.to_string(),
/// };
/// ```
pub struct ExploreItem {
    /// 产生此探索项的工具索引
    pub tool_idx: usize,
    /// 原始数据字符串
    pub raw: String,
}

/// 连续命令摘要中的单条命令。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CommandItem {
    /// 原始工具块在消息中的工具索引。
    pub tool_idx: usize,
    /// 原始工具块，用于复用单条 Bash 视图。
    pub raw: String,
}

/// 探索组工具索引常量
///
/// 用于标识探索组（Explore Group）的特殊工具索引值。
/// 该值设置为 `u32::MAX`，作为一个哨兵值（sentinel value），
/// 用于在工具列表中表示"探索组"这一特殊概念，而非单个具体工具。
///
/// # 设计说明
///
/// 使用 `u32::MAX` 作为特殊索引的原因：
/// - 该值远超正常的工具数量，不会与实际工具索引冲突
/// - 便于在代码中快速识别这是特殊标识而非普通工具
/// - 与底层存储类型（u32）保持一致，便于类型转换
pub const EXPLORE_GROUP_TOOL_IDX: usize = u32::MAX as usize;
