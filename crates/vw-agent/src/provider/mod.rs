//! # Provider 模块
//!
//! 提供模型提供者（Provider）的核心抽象与实现，是 VibeWindow 与各类 LLM/推理服务交互的统一入口层。
//!
//! ## 模块结构
//!
//! - [`auth`] — 提供者认证与鉴权：管理 API Key、Token、签名、会话等身份凭证与安全策略。
//! - [`error`] — 错误类型定义：统一封装与提供者交互过程中可能发生的各类错误（网络、协议、配额、超时等）。
//! - [`models`] — 数据模型与协议结构：定义请求体、响应体、消息格式、工具调用、嵌入向量等公共类型。
//! - [`provider`] — Provider trait 与具体实现：定义标准能力契约，并内置 OpenAI、Anthropic 等主流提供者实现。
//! - [`transform`] — 请求/响应转换与适配层：在内部表示与不同提供者的外部协议之间进行双向映射。
//!
//! ## 设计原则
//!
//! - 契约驱动：所有提供者必须实现统一的 [`Provider`] trait（定义于 `provider` 子模块），支持能力协商、健康检查与优雅降级。
//! - 可扩展性：新增提供者只需实现 trait 并在工厂注册，无需侵入核心编排逻辑。
//! - 安全优先：认证、密钥与敏感信息通过 [`auth`] 统一管理，避免在调用链路中泄露。
//! - 可观测性：错误与状态通过 [`error`] 标准化，便于日志、指标与告警集成。

pub mod auth;

pub mod error {
    pub use vw_provider_resolver::error::*;
}

pub mod models {
    pub use vw_provider_resolver::models::*;
}

pub mod provider;

pub mod transform;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
