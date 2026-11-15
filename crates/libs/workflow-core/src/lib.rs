//! 工作流图模型、条件求值、变量池与模板渲染。

pub mod conditions;
pub mod model;
pub mod template;
pub mod variables;

#[cfg(not(target_arch = "wasm32"))]
pub mod store;
#[cfg(not(target_arch = "wasm32"))]
pub mod templates;
