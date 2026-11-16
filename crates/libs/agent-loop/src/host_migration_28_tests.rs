//! 工具循环单轮与输出领域回归；真实插件、MCP 与宿主上下文仍在宿主验收。
use anyhow::Result;
use tokio_util::sync::CancellationToken;
use crate::core::errors::ToolLoopCancelled;
mod llm_round;
mod output;
mod tool_identity;
