//! 插件清单兼容适配与通过真实入口运行的独立验收测试。

pub(super) mod identity;
pub(super) mod legacy_manifest;
pub(super) mod path_audit;
pub(super) mod portable_manifest;

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
