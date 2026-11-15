#[derive(Debug, Clone)]
pub struct GitDiffSelectedLine {
    /// 文件路径
    pub file: String,
    /// 行号（1-based）
    pub line: usize,
    /// 是否为旧版本（false 表示新版本）
    pub is_old: bool,
    /// 行的文本内容
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct GitDiffFileMenuState {
    /// 文件路径
    pub file: String,
}

#[derive(Debug, Clone)]
pub struct ChatTextDiff {
    /// 差异标题
    pub title: String,
    /// 文件路径
    pub file: String,
    /// 修改前的内容
    pub before: String,
    /// 修改后的内容
    pub after: String,
}

#[cfg(test)]
#[path = "workspace_state_tests.rs"]
mod tests;
