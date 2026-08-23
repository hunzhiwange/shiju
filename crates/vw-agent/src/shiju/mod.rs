//! SHIJU 账户与应用平台的上游集成入口。
//!
//! 账户模块负责身份与资料，应用模块负责目录、密钥绑定和安全启动会话。

/// SHIJU 账户、登录与身份资料客户端。
pub(crate) mod account;
/// SHIJU 应用目录、IAM 密钥和运行会话客户端。
pub(crate) mod apps;
