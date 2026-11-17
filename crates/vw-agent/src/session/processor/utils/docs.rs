//! 文档辅助逻辑由会话引擎提供。

#[cfg(any(test, target_arch = "wasm32"))]
pub(crate) use session_engine::processor_docs::{is_docs_request, list_docs};
