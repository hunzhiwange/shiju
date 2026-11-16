use super::*;

pub(crate) use agent_config::runtime::Config;
pub(crate) use memory_types::memory::{Memory, MemoryCategory, MemoryEntry};
pub(crate) use model_types::Provider;
pub(crate) use gateway_pairing::pairing::PairingGuard;
use async_trait::async_trait;
use axum::extract::ConnectInfo;
use axum::http::HeaderValue;
use parking_lot::Mutex;
use std::net::SocketAddr;
pub(crate) use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
pub(crate) use std::time::Duration;

/// 模拟内存存储实现
///
/// 提供最小化的 `Memory` trait 实现，所有操作均为空操作（no-op）。
/// 适用于不需要验证内存行为的测试场景。
///
/// # 特性
///
/// - `store`: 立即返回成功，不实际存储
/// - `recall`/`get`/`list`: 始终返回空结果
/// - `forget`: 始终返回 `false`（未找到）
/// - `count`: 始终返回 `0`
/// - `health_check`: 始终返回 `true`
#[derive(Default)]
struct MockMemory;

/// 为 MockMemory 实现 Memory trait
///
/// 所有方法均为最小化实现，满足 trait 签名但不执行实际操作。
/// 使用条件编译属性以同时支持 WASM 和原生平台。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Memory for MockMemory {
    /// 返回内存后端名称标识
    fn name(&self) -> &str {
        "mock"
    }

    /// 存储内存条目（空实现）
    ///
    /// # 参数
    ///
    /// - `_key`: 内存条目键名（被忽略）
    /// - `_content`: 内存条目内容（被忽略）
    /// - `_category`: 内存分类（被忽略）
    /// - `_session_id`: 会话标识（被忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(())`
    async fn store(
        &self,
        _key: &str,
        _content: &str,
        _category: MemoryCategory,
        _session_id: Option<&str>,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    /// 检索相关记忆（空实现）
    ///
    /// # 参数
    ///
    /// - `_query`: 查询字符串（被忽略）
    /// - `_limit`: 返回数量限制（被忽略）
    /// - `_session_id`: 会话标识（被忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回空的 `Vec<MemoryEntry>`
    async fn recall(
        &self,
        _query: &str,
        _limit: usize,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    /// 按键获取单个内存条目（空实现）
    ///
    /// # 参数
    ///
    /// - `_key`: 内存条目键名（被忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(None)`
    async fn get(&self, _key: &str) -> anyhow::Result<Option<MemoryEntry>> {
        Ok(None)
    }

    /// 列出内存条目（空实现）
    ///
    /// # 参数
    ///
    /// - `_category`: 可选的分类过滤器（被忽略）
    /// - `_session_id`: 会话标识（被忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回空的 `Vec<MemoryEntry>`
    async fn list(
        &self,
        _category: Option<&MemoryCategory>,
        _session_id: Option<&str>,
    ) -> anyhow::Result<Vec<MemoryEntry>> {
        Ok(Vec::new())
    }

    /// 删除内存条目（空实现）
    ///
    /// # 参数
    ///
    /// - `_key`: 要删除的内存条目键名（被忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(false)`，表示未找到条目
    async fn forget(&self, _key: &str) -> anyhow::Result<bool> {
        Ok(false)
    }

    /// 统计内存条目数量（空实现）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok(0)`
    async fn count(&self) -> anyhow::Result<usize> {
        Ok(0)
    }

    /// 健康检查
    ///
    /// # 返回值
    ///
    /// 始终返回 `true`，表示 Mock 内存始终可用
    async fn health_check(&self) -> bool {
        true
    }
}

/// 模拟 AI 提供者实现
///
/// 提供最小化的 `Provider` trait 实现，用于测试需要调用 AI 模型的场景。
/// 内部使用原子计数器跟踪调用次数，便于验证调用行为。
///
/// # 字段
///
/// - `calls`: 原子计数器，记录 `chat_with_system` 被调用的次数
///
/// # 特性
///
/// - `chat_with_system`: 增加调用计数并返回固定响应 `"ok"`
#[derive(Default)]
struct MockProvider {
    /// 调用次数计数器，使用原子类型保证线程安全
    calls: AtomicUsize,
}

/// 为 MockProvider 实现 Provider trait
///
/// 实现 `chat_with_system` 方法以满足 trait 要求，
/// 同时提供调用跟踪能力。
#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl Provider for MockProvider {
    /// 与 AI 模型进行带系统提示的对话（模拟实现）
    ///
    /// # 参数
    ///
    /// - `_system_prompt`: 系统提示词（被忽略）
    /// - `_message`: 用户消息（被忽略）
    /// - `_model`: 模型标识（被忽略）
    /// - `_temperature`: 温度参数（被忽略）
    ///
    /// # 返回值
    ///
    /// 始终返回 `Ok("ok".into())`
    ///
    /// # 副作用
    ///
    /// 增加 `calls` 计数器，使用顺序一致性内存序
    async fn chat_with_system(
        &self,
        _system_prompt: Option<&str>,
        _message: &str,
        _model: &str,
        _temperature: f64,
    ) -> anyhow::Result<String> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok("ok".into())
    }
}

/// 创建本地回环地址的测试连接信息
///
/// 生成一个使用 `127.0.0.1` 地址和端口 `30300` 的 `ConnectInfo`，
/// 用于模拟来自本地主机的连接请求。
///
/// # 返回值
///
/// 包含 `127.0.0.1:30300` 地址的 `ConnectInfo<SocketAddr>` 实例
///
/// # 用例
///
/// 在测试中模拟本地客户端连接，通常用于测试本地访问控制逻辑
fn test_connect_info() -> ConnectInfo<SocketAddr> {
    ConnectInfo(SocketAddr::from(([127, 0, 0, 1], 30_300)))
}

/// 创建公网地址的测试连接信息
///
/// 生成一个使用 `203.0.113.10` 地址（TEST-NET-3 测试网段）和端口 `30300` 的 `ConnectInfo`，
/// 用于模拟来自公网 IP 的连接请求。
///
/// # 返回值
///
/// 包含 `203.0.113.10:30300` 地址的 `ConnectInfo<SocketAddr>` 实例
///
/// # 用例
///
/// 在测试中模拟外部公网客户端连接，用于测试基于 IP 的访问控制或
/// 区分本地/远程请求的逻辑
///
/// # 注意
///
/// 使用 RFC 5737 定义的测试网段地址，该地址不会在真实网络中分配
fn test_public_connect_info() -> ConnectInfo<SocketAddr> {
    ConnectInfo(SocketAddr::from(([203, 0, 113, 10], 30_300)))
}


mod auth;
mod basics;
mod client_key;
mod memory_keys;
mod metrics;
mod node_control;
mod pairing;
mod state;
