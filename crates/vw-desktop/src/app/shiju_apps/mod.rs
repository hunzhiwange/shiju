//! 汇总识矩应用的桌面状态模型与安全启动能力。

mod launcher;
mod state;

// 启动器负责进程与 URL 安全边界，界面层不直接拼装启动命令。
pub(crate) use launcher::*;
// 页面与消息处理共享同一组账号范围状态模型。
pub(crate) use state::*;

#[cfg(test)]
#[path = "launcher_tests.rs"]
mod launcher_tests;
