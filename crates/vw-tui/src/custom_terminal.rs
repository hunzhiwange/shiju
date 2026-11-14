//! 绘制后端由 tui-terminal 提供；单元测试复用同一源码以保留固定尺寸辅助入口。

#[cfg(not(test))]
pub(crate) use tui_terminal::custom_terminal::*;

#[cfg(test)]
#[path = "../../libs/tui-terminal/src/custom_terminal.rs"]
mod implementation;
#[cfg(test)]
pub(crate) use implementation::*;

#[cfg(test)]
#[path = "custom_terminal_tests.rs"]
mod cross_layer_tests;
