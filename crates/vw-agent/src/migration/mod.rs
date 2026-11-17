//! OpenClaw 记忆迁移兼容导出。
#[cfg(any(test, target_arch = "wasm32"))]
pub use agent_migration::handle_command;
