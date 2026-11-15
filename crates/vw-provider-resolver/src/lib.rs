//! Provider 解析兼容门面。
//!
//! 运行时实现与共享状态由各职责库唯一持有；工作区内部消费者直接依赖职责库。

/// Provider 认证信息读取。
pub use provider_auth_reader as auth;
/// 进程内异步缓存。
pub use provider_state_cache as cache;
/// Provider 相关配置读取。
pub use provider_config_reader as config;
/// Provider 调用错误解析。
pub use provider_errors as error;
/// 环境变量开关与标记。
pub use provider_request_identity::flag;
/// 全局路径与缓存目录管理。
pub use provider_runtime_paths as global;
/// 安装信息与 User-Agent 生成。
pub use provider_request_identity::installation;
/// 模型目录、快照缓存与刷新。
pub use provider_model_cache as models;
/// Provider 与模型对外查询接口。
pub use provider_resolution as provider;
/// 已连接 provider 的 OpenAI 兼容模型发现。
#[cfg(not(target_arch = "wasm32"))]
pub use provider_model_discovery as remote_models;
/// 原生优先的部署协议路由选择。
pub use provider_protocol_routes as protocol_routes;

#[cfg(test)]
mod protocol_matrix_tests;
