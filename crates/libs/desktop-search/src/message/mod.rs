#[derive(Debug, Clone)]
pub enum SearchMessage {
    /// 消费模态卡片点击。
    Ignore,
    /// 打开搜索面板
    ///
    /// 会清空上一次查询，让 Command+P 或顶部搜索按钮始终从空输入开始。
    Open,

    /// 搜索输入框内容变更
    ///
    /// 当用户在搜索框中输入或修改文本时触发。
    /// 输入内容不控制搜索遮罩层，清空查询时弹窗仍保持打开。
    InputChanged(String),

    /// 切换搜索遮罩层的显示状态
    ///
    /// 用于手动控制搜索遮罩层的显示或隐藏。
    Toggle(bool),

    /// 选择项目
    ///
    /// 当用户从搜索结果中选择一个项目时触发。
    /// 会打开该项目并建立索引。
    SelectProject(String),

    /// 选择文件
    ///
    /// 当用户从搜索结果中选择一个文件时触发。
    /// 会将文件路径填入文件输入框。
    SelectFile(String),

    /// 选择会话
    ///
    /// 当用户从搜索结果中选择一个历史会话时触发。
    /// 通过项目导航入口验证来源后再打开会话。
    SelectSession(String),
}


#[derive(Debug, Default)]
pub struct State {
    pub search_text: String,
    pub show_search_overlay: bool,
}

pub enum Effect {
    None,
    RefreshFiles { focus: bool },
    OpenProject(String),
    SelectFile(String),
    OpenSession(String),
}

pub fn update(state: &mut State, message: SearchMessage) -> Effect {
    match message {
        SearchMessage::Ignore => Effect::None,
        SearchMessage::Open => {
            state.search_text.clear();
            state.show_search_overlay = true;
            Effect::RefreshFiles { focus: true }
        }
        SearchMessage::InputChanged(value) => {
            state.search_text = value;
            Effect::RefreshFiles { focus: false }
        }
        SearchMessage::Toggle(value) => {
            state.show_search_overlay = value;
            Effect::None
        }
        SearchMessage::SelectProject(path) => {
            state.show_search_overlay = false;
            Effect::OpenProject(path)
        }
        SearchMessage::SelectFile(path) => {
            state.show_search_overlay = false;
            Effect::SelectFile(path)
        }
        SearchMessage::SelectSession(id) => {
            state.show_search_overlay = false;
            Effect::OpenSession(id)
        }
    }
}

#[cfg(test)]
mod tests;
