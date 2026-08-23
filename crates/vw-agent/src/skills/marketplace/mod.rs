//! 与来源无关的技能市场提供方及安装能力入口。
//!
//! 提供方只负责获取与描述制品，安装模块独立执行签名、摘要、归档和安全审计。

/// 已验证市场制品的原子安装实现。
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod install;
/// 市场提供方契约、制品证明与来源注册表。
pub(crate) mod provider;
/// 公共 SkillHub 来源适配器。
pub(crate) mod skillhub;

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "install_tests.rs"]
mod install_tests;

#[cfg(test)]
#[path = "skillhub_tests.rs"]
mod skillhub_tests;
