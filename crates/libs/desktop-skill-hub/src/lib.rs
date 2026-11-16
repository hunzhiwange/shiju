//! 技能市场领域客户端与账号范围状态。
pub mod skill_hub;
pub use skill_hub::*;

pub mod message;
pub mod skill_operations;
pub type GatewayFactory = fn() -> Result<gateway_http::GatewayTransport, String>;
