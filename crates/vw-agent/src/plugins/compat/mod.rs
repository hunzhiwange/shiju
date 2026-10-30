//! 插件清单兼容适配与通过真实入口运行的独立验收测试。

pub(super) mod agents;
pub(super) mod commands;
pub(super) mod discovery;
pub(super) mod identity;
pub(super) mod legacy_manifest;
pub(super) mod mcp_auth;
pub(super) mod mcp_environment;
pub(super) mod mcp_legacy;
pub(super) mod mcp_portable;
pub(super) mod mcp_portable_overlay;
pub(super) mod mcp_public_auth;
pub(super) mod onboarding;
pub(super) mod path_audit;
pub(super) mod portable_manifest;
pub(super) mod skill_frontmatter;
pub(super) mod skill_invocation;
pub(super) mod skill_roots;
pub(super) mod transaction_install;

#[cfg(test)]
#[path = "t02_tests.rs"]
mod t02_tests;

#[cfg(test)]
#[path = "t03_tests.rs"]
mod t03_tests;

#[cfg(test)]
#[path = "t04_tests.rs"]
mod t04_tests;

#[cfg(test)]
#[path = "t05_tests.rs"]
mod t05_tests;

#[cfg(test)]
#[path = "t06_tests.rs"]
mod t06_tests;

#[cfg(test)]
#[path = "t07_tests.rs"]
mod t07_tests;

#[cfg(test)]
#[path = "t08_tests.rs"]
mod t08_tests;

#[cfg(test)]
#[path = "t09_tests.rs"]
mod t09_tests;

#[cfg(test)]
#[path = "t10_tests.rs"]
mod t10_tests;

#[cfg(test)]
#[path = "t11_tests.rs"]
mod t11_tests;

#[cfg(test)]
#[path = "t12_tests.rs"]
mod t12_tests;

#[cfg(test)]
#[path = "t13_tests.rs"]
mod t13_tests;

#[cfg(test)]
#[path = "t14_tests.rs"]
mod t14_tests;

#[cfg(test)]
#[path = "t15_tests.rs"]
mod t15_tests;

#[cfg(test)]
#[path = "t16_tests.rs"]
mod t16_tests;

#[cfg(test)]
#[path = "t17_tests.rs"]
mod t17_tests;

#[cfg(test)]
#[path = "t17_public_tests.rs"]
mod t17_public_tests;
