//! Provider 解析层。
//!
//! 本 crate 负责将多个来源的 provider 信息整理为统一的查询接口，主要包括：
//! - 读取内置与缓存的模型元数据
//! - 合并用户配置中的 provider 覆盖项
//! - 解析认证信息与运行时环境变量
//! - 对外提供 provider、model 与默认模型查询能力
//! - 统一解析上游 provider 的错误响应
//!
//! # 主要模块
//!
//! - [`auth`]：读取本地认证信息
//! - [`config`]：读取配置文件与网关端点信息
//! - [`models`]：加载模型基线元数据
//! - [`provider`]：组合多种数据源，输出最终 provider 视图
//! - [`error`]：将上游错误规范化为统一结构
//! - [`global`]：管理缓存目录与数据目录

/// Provider 认证信息读取。
pub mod auth;
/// 进程内异步缓存。
pub mod cache;
/// Provider 相关配置读取。
pub mod config;
/// Provider 调用错误解析。
pub mod error;
/// 环境变量开关与标记。
pub mod flag;
/// 全局路径与缓存目录管理。
pub mod global;
/// 安装信息与 User-Agent 生成。
pub mod installation;
/// 模型基线元数据加载。
pub mod models;
/// Provider 与模型对外查询接口。
pub mod provider;
#[cfg(not(target_arch = "wasm32"))]
/// 已连接 provider 的 OpenAI 兼容模型发现。
pub mod remote_models;

#[cfg(test)]
#[path = "error_tests.rs"]
mod error_tests;
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "remote_models_tests.rs"]
mod remote_models_tests;
