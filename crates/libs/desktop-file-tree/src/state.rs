use iced::widget::text_editor;
/// 文件夹内查找匹配结果
///
/// 表示在文件夹搜索中找到的单个匹配项，
/// 包含文件路径、位置信息和预览文本。
#[derive(Debug, Clone)]
pub struct FindInFolderMatch {
    /// 匹配所在的文件路径
    pub path: String,
    /// 匹配的行号
    pub line: usize,
    /// 匹配的列号
    pub column: usize,
    /// 匹配的预览文本
    pub preview: String,
    /// 匹配文本的长度
    pub match_len: usize,
}

/// 搜索结果项包含预览、右键菜单和拖拽交互；保持较小页面以避免切换标签时重建过多组件。
pub const FIND_RESULTS_PAGE_SIZE: usize = 24;

pub fn find_results_page_bounds(
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
pub struct FindInFolderTab {
    /// 标签页的唯一标识符
    pub id: String,
    /// 标签页标题
    pub title: String,
    /// 搜索范围的路径
    pub scope_path: String,
    /// 查询输入框文本
    pub query_input: String,
    /// 替换输入框文本
    pub replace_input: String,
    /// 查询编辑器内容
    pub query_editor: text_editor::Content,
    /// 替换编辑器内容
    pub replace_editor: text_editor::Content,
    /// 当前执行的查询
    pub query: String,
    /// 替换文本
    pub replace_text: String,
    /// 是否区分大小写
    pub case_sensitive: bool,
    /// 是否全词匹配
    pub whole_word: bool,
    /// 是否使用正则表达式
    pub use_regex: bool,
    /// 搜索是否正在运行
    pub running: bool,
    /// 错误信息（如果有）
    pub error: Option<String>,
    /// 本地校验反馈保留中文模板与参数，显示时选择语言。
    pub localized_error: Option<desktop_i18n::input::Feedback>,
    /// 是否达到结果数量限制
    pub limit_reached: bool,
    /// 匹配结果列表
    pub matches: Vec<FindInFolderMatch>,
    /// 当前显示的结果页，从 0 开始
    pub results_page: usize,
}
