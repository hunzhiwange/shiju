//! 本模块负责会话标题中的模型信息。
//! ChatWidget 仅维护展示状态、历史、输入队列与覆盖层；运行时业务由外层系统负责。
pub(crate) struct SessionHeader {
    model: String,
}

impl SessionHeader {
    pub(crate) fn new(model: String) -> Self {
        Self { model }
    }

    /// Updates the header's model text.
    /// 更新标题的模型文本。
    pub(crate) fn set_model(&mut self, model: &str) {
        if self.model != model {
            self.model = model.to_string();
        }
    }
}
