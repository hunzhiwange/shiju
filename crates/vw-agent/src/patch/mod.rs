//! patch 能力的代理导出层。
//!
//! `vw-agent` 复用 `patch` 中稳定的 patch 数据结构与解析/应用工具，
//! 本模块保持边界很薄，避免在 agent 层复制补丁语义。

pub use ::patch::patch::*;
