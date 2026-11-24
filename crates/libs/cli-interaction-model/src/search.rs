//! 快速打开、全局、历史与 transcript 搜索的共享状态。
//!
//! 每次请求递增 `generation`，回传结果只能更新当前活跃且代次匹配的搜索，
//! 从而拒绝较慢的旧请求覆盖新查询。

/// 搜索的数据范围与交互语义。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    QuickOpen,
    Global,
    History,
    Transcript,
}

/// 一条可选搜索结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub id: String,
    pub label: String,
    pub preview: String,
    pub line: Option<usize>,
}

/// 搜索对话框的查询、代次、结果和原始焦点。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchState {
    pub mode: SearchMode,
    pub query: String,
    pub generation: u64,
    pub results: Vec<SearchResult>,
    pub selected: usize,
    pub active: bool,
    pub original_focus: String,
    pub error: Option<String>,
}

impl SearchState {
    /// 打开指定模式的搜索，并记住取消时需恢复的焦点。
    pub fn new(mode: SearchMode, original_focus: impl Into<String>) -> Self {
        Self {
            mode,
            query: String::new(),
            generation: 0,
            results: Vec::new(),
            selected: 0,
            active: true,
            original_focus: original_focus.into(),
            error: None,
        }
    }

    /// 关闭搜索并返回打开前的焦点标识。
    pub fn cancel(&mut self) -> String {
        self.active = false;
        self.original_focus.clone()
    }
}
