//! SHIJU 账户、应用、连接器与知识库 HTTP 路由。
#![cfg(not(target_arch = "wasm32"))]
pub mod shiju_account;
pub mod shiju_apps;
mod shiju_connectors;
mod shiju_knowledge;
#[cfg(test)]
mod shiju_knowledge_gateway_tests;
pub mod shiju_skills;
