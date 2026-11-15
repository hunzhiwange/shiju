//! 终端输入组合器的编辑、页脚渲染与本地状态。

pub mod footer;
pub mod footer_state;

pub mod chat_composer_history;
pub mod pending_input_preview;

pub mod chat_composer;
pub use chat_composer::ChatComposer;
