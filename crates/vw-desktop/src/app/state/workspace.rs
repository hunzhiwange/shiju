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
#[derive(Debug, Clone)]
pub(crate) struct GitDiffLineRange {
    /// 文件路径
    pub(crate) file: String,
    /// 起始行号
    pub(crate) start: usize,
    /// 结束行号
    pub(crate) end: usize,
    /// 是否为旧版本
    pub(crate) is_old: bool,
}

/// Git 差异右键菜单状态
///
/// 记录当前打开的右键菜单所对应的行以及菜单锚点位置。
#[derive(Debug, Clone)]
pub(crate) struct GitDiffContextMenuState {
    /// 文件路径
    pub(crate) file: String,
    /// 行号
    pub(crate) line: usize,
    /// 是否为旧版本
    pub(crate) is_old: bool,
    /// 菜单锚点 X 坐标
    pub(crate) x: f32,
    /// 菜单锚点 Y 坐标
    pub(crate) y: f32,
}

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
#[derive(Debug, Clone)]
pub(crate) enum FileTreeClipboardMode {
    /// 复制模式
    Copy,
    /// 剪切模式
    Cut,
}

/// 文件树剪贴板内容
///
/// 存储文件树中复制或剪切的文件路径信息。
#[derive(Debug, Clone)]
pub(crate) struct FileTreeClipboard {
    /// 剪贴板操作模式
    pub(crate) mode: FileTreeClipboardMode,
    /// 源文件路径
    pub(crate) src_path: String,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskBoardStaffFeedTab {
    #[default]
    Tasks,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskBoardStaffAssignmentTab {
    #[default]
    Primary,
    Manager,
    Backup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TaskBoardStaffOrchestrationTab {
    #[default]
    Dispatch,
    Staffing,
}

/// 文件夹内查找匹配结果
///
/// 表示在文件夹搜索中找到的单个匹配项，
/// 包含文件路径、位置信息和预览文本。
#[derive(Debug, Clone)]
pub struct FindInFolderMatch {
    /// 匹配所在的文件路径
    pub(crate) path: String,
    /// 匹配的行号
    pub(crate) line: usize,
    /// 匹配的列号
    pub(crate) column: usize,
    /// 匹配的预览文本
    pub(crate) preview: String,
    /// 匹配文本的长度
    pub(crate) match_len: usize,
}

pub(crate) const FIND_RESULTS_PAGE_SIZE: usize = 100;

pub(crate) fn find_results_page_bounds(
    result_count: usize,
    requested_page: usize,
) -> (std::ops::Range<usize>, usize, usize) {
    let page_count = result_count.div_ceil(FIND_RESULTS_PAGE_SIZE);
    let page = requested_page.min(page_count.saturating_sub(1));
    let start = page.saturating_mul(FIND_RESULTS_PAGE_SIZE).min(result_count);
    let end = start.saturating_add(FIND_RESULTS_PAGE_SIZE).min(result_count);
    (start..end, page, page_count)
}

/// 文件夹内查找标签页状态
///
/// 管理单个"在文件夹中查找"标签页的完整状态，
/// 包括查询条件、替换文本、搜索选项和结果列表。
#[derive(Debug, Clone)]
pub(crate) struct FindInFolderTab {
    /// 标签页的唯一标识符
    pub(crate) id: String,
    /// 标签页标题
    pub(crate) title: String,
    /// 搜索范围的路径
    pub(crate) scope_path: String,
    /// 查询输入框文本
    pub(crate) query_input: String,
    /// 替换输入框文本
    pub(crate) replace_input: String,
    /// 查询编辑器内容
    pub(crate) query_editor: text_editor::Content,
    /// 替换编辑器内容
    pub(crate) replace_editor: text_editor::Content,
    /// 当前执行的查询
    pub(crate) query: String,
    /// 替换文本
    pub(crate) replace_text: String,
    /// 是否区分大小写
    pub(crate) case_sensitive: bool,
    /// 是否全词匹配
    pub(crate) whole_word: bool,
    /// 是否使用正则表达式
    pub(crate) use_regex: bool,
    /// 搜索是否正在运行
    pub(crate) running: bool,
    /// 错误信息（如果有）
    pub(crate) error: Option<String>,
    /// 是否达到结果数量限制
    pub(crate) limit_reached: bool,
    /// 匹配结果列表
    pub(crate) matches: Vec<FindInFolderMatch>,
    /// 当前显示的结果页，从 0 开始
    pub(crate) results_page: usize,
}

/// Git 差异评论草稿
///
/// 存储正在编辑的 Git 差异评论，
/// 包括评论的行范围和编辑器内容。
#[derive(Debug, Clone)]
pub(crate) struct GitDiffCommentDraft {
    /// 评论的行范围
    pub(crate) range: GitDiffLineRange,
    /// 评论编辑器内容
    pub(crate) editor: text_editor::Content,
}

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
    /// 输出长度限制（token 数）
    pub output_limit: u64,
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
