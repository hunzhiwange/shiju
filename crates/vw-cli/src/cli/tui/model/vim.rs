//! 持久在 TUI prompt 与会话选项中的 Vim 编辑模式。
//!
//! 此处只定义稳定状态值；按键映射和模式转换由输入控制层完成。

/// Vim 兼容编辑的关闭、普通与插入状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum VimMode {
    #[default]
    Disabled,
    Normal,
    Insert,
}
