//! 工具循环领域回归与真实宿主端口场景。泛型场景由宿主测试传入实际服务。
#[cfg(test)]
use anyhow::Result;
#[cfg(test)]
use tokio_util::sync::CancellationToken;
#[cfg(test)]
use crate::core::errors::ToolLoopCancelled;
#[cfg(test)]
mod llm_round;
#[cfg(test)]
mod output;
#[cfg(test)]
mod tool_identity;
pub mod discovery;
pub mod tool_execution;
