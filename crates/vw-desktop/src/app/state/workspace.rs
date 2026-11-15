use super::*;
use std::fmt;

/// Git 面板可切换的工作树选项。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitWorktreeOption {
    pub(crate) directory: String,
    pub(crate) label: String,
    pub(crate) branch: Option<String>,
}

impl fmt::Display for GitWorktreeOption {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.label)
    }
}

/// Git 差异选中的行信息
///
/// 表示在 Git 差异视图中被选中的单行信息，
/// 包含文件路径、行号、是否为旧版本以及行内容。
#[derive(Debug, Clone)]
pub(crate) struct GitDiffSelectedLine {
    /// 文件路径
    pub(crate) file: String,
    /// 行号（1-based）
    pub(crate) line: usize,
    /// 是否为旧版本（false 表示新版本）
    pub(crate) is_old: bool,
    /// 行的文本内容
    pub(crate) text: String,
}

/// Git 差异行范围
///
/// 表示在 Git 差异视图中选中的连续行范围，
/// 用于批量操作或添加评论。
pub(crate) use desktop_git_ui::git_panel::diff_view::GitDiffLineRange;

/// Git 差异右键菜单状态
///
/// 记录当前打开的右键菜单所对应的行以及菜单锚点位置。
pub(crate) use desktop_git_ui::git_panel::diff_view::GitDiffContextMenuState;

/// Git 差异文件菜单状态
///
/// 记录当前打开文件级操作菜单的文件路径。
#[derive(Debug, Clone)]
pub(crate) struct GitDiffFileMenuState {
    /// 文件路径
    pub(crate) file: String,
}

/// 文件树剪贴板模式
///
/// 定义剪贴板操作的模式类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileTreeClipboardMode {
    /// 复制模式
    Copy,
    /// 剪切模式
    Cut,
}

/// 文件树剪贴板内容
///
/// 存储文件树中复制或剪切的文件路径信息。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileTreeClipboard {
    /// 剪贴板操作模式
    pub(crate) mode: FileTreeClipboardMode,
    /// 源路径；同时选择父目录与子项时只保留父目录
    pub(crate) src_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectEditTab {
    General,
    Launch,
    Refresh,
    Scheduling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskBoardSettingsModalTab {
    #[default]
    Refresh,
    Scheduling,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskBoardDetailTab {
    #[default]
    Basic,
    Logs,
}

pub use desktop_file_tree::state::{FindInFolderMatch, FindInFolderTab, FIND_RESULTS_PAGE_SIZE, find_results_page_bounds};

/// Git 差异评论草稿
///
/// 存储正在编辑的 Git 差异评论，
/// 包括评论的行范围和编辑器内容。
pub(crate) use desktop_git_ui::git_panel::diff_view::GitDiffCommentDraft;

/// 聊天文本差异
///
/// 表示在聊天界面中显示的代码差异，
/// 用于展示文件修改的前后对比。
#[derive(Debug, Clone)]
pub(crate) struct ChatTextDiff {
    /// 差异标题
    pub(crate) title: String,
    /// 文件路径
    pub(crate) file: String,
    /// 修改前的内容
    pub(crate) before: String,
    /// 修改后的内容
    pub(crate) after: String,
}

/// 使用量步骤原始 LLM 载荷弹窗状态
///
/// 在步骤展开后点击“查看原始记录”时展示完整可复制内容。
#[derive(Debug, Clone)]
pub struct UsageStepRawModal {
    /// 对应的步骤索引
    pub step_index: u32,
    /// 完整原始记录文本
    pub content: String,
}

/// 使用量模型信息
///
/// 包含模型的详细使用信息和定价配置，
/// 用于显示和计算 token 使用量和成本。
#[derive(Debug, Clone)]
pub struct UsageModelInfo {
    /// 提供者标识符
    pub provider_id: String,
    /// 提供者显示名称
    pub provider_name: String,
    /// 模型标识符
    pub model_id: String,
    /// 模型显示名称
    pub model_name: String,
    /// 上下文长度限制（token 数）
    pub context_limit: u64,
    /// 当前请求可用的输入窗口（token 数）
    pub input_limit: u64,
    /// 输出长度限制（token 数）
    pub output_limit: u64,
    /// 是否使用未知模型的保守默认限制
    pub limit_is_default: bool,
    /// 输入 token 每百万的成本（美元）
    pub cost_input_per_million: f64,
    /// 输出 token 每百万的成本（美元）
    pub cost_output_per_million: f64,
    /// 缓存读取 token 每百万的成本（美元）
    pub cost_cache_read_per_million: f64,
    /// 缓存写入 token 每百万的成本（美元）
    pub cost_cache_write_per_million: f64,
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod workspace_tests;
