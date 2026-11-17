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

    /// 开始新查询并返回专用代次号，供异步回传时校验。
    #[cfg(test)]
    pub(crate) fn begin_request(&mut self, query: impl Into<String>) -> u64 {
        self.query = query.into();
        self.generation = self.generation.saturating_add(1);
        self.error = None;
        self.generation
    }

    /// 仅应用当前活跃代次的结果；返回值表示回传是否被接受。
    #[cfg(test)]
    pub(crate) fn apply_results(
        &mut self,
        generation: u64,
        result: Result<Vec<SearchResult>, String>,
    ) -> bool {
        if !self.active || generation != self.generation {
            return false;
        }
        match result {
            Ok(results) => {
                self.results = results;
                self.selected = self.selected.min(self.results.len().saturating_sub(1));
            }
            Err(error) => self.error = Some(error),
        }
        true
    }

    /// 关闭搜索并返回打开前的焦点标识。
    pub fn cancel(&mut self) -> String {
        self.active = false;
        self.original_focus.clone()
    }
}
