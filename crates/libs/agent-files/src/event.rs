use agent_bus;

/// 文件编辑事件定义
///
/// 当文件被编辑时发布此事件，其他模块可以订阅此事件来响应文件变更。
/// 事件类型为 `"file.edited"`。
pub const EDITED: bus::Definition = bus::Definition { r#type: "file.edited" };

#[cfg(test)]
#[path = "event_tests.rs"]
mod event_tests;
